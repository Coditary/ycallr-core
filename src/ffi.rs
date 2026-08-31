#![allow(clippy::not_unsafe_ptr_arg_deref)]

use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use crate::client::{EnvMode, YcallrClient};
use crate::models::{ApiDefinition, AuthConfig};

// ─── Thread-local error ───────────────────────────────────────────────

thread_local! {
    static LAST_ERROR: RefCell<Option<CString>> = const { RefCell::new(None) };
    static LAST_INSTALL_RESULT: RefCell<Option<String>> = const { RefCell::new(None) };
}

fn set_last_error(msg: String) {
    let sanitized = msg.replace('\0', "");
    LAST_ERROR.with(|e| {
        *e.borrow_mut() = CString::new(sanitized).ok();
    });
}

fn set_last_install_result(name: &str, pb_path: &std::path::Path) {
    let json = serde_json::json!({
        "name": name,
        "pb_path": pb_path.to_string_lossy(),
    });
    LAST_INSTALL_RESULT.with(|r| {
        *r.borrow_mut() = serde_json::to_string(&json).ok();
    });
}

fn compute_missing_params_json(
    api: &ApiDefinition,
    command_path: &str,
    params: &HashMap<String, String>,
) -> Result<String, String> {
    let cmd = api.get_command(command_path).map_err(|e| e.to_string())?;
    let missing = missing_required_params(cmd, params);
    serde_json::to_string(&missing).map_err(|e| e.to_string())
}

fn compute_implicit_body_json(
    api: &ApiDefinition,
    command_path: &str,
    params: &HashMap<String, String>,
) -> Option<String> {
    let cmd = api.get_command(command_path).ok()?;
    build_implicit_body(cmd, params).map(|v| v.to_string())
}

fn missing_required_params(
    cmd: &crate::models::Command,
    params: &HashMap<String, String>,
) -> Vec<String> {
    let mut missing = Vec::new();

    for (name, param) in &cmd.params {
        if param.required && !params.contains_key(name) {
            missing.push(name.clone());
        }
    }

    for path_param in cmd.endpoint_path_param_names() {
        if !cmd.params.contains_key(&path_param) && !params.contains_key(&path_param) {
            if !missing.contains(&path_param) {
                missing.push(path_param);
            }
        }
    }

    missing
}

fn build_implicit_body(
    cmd: &crate::models::Command,
    params: &HashMap<String, String>,
) -> Option<serde_json::Value> {
    if cmd.body.is_some() {
        return None;
    }

    match cmd.method.as_ref() {
        Some(crate::models::HttpMethod::POST)
        | Some(crate::models::HttpMethod::PUT)
        | Some(crate::models::HttpMethod::PATCH) => {
            let path_params = cmd.endpoint_path_param_names();
            let body: serde_json::Map<String, serde_json::Value> = params
                .iter()
                .filter(|(key, _)| !path_params.contains(key))
                .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone())))
                .collect();

            if body.is_empty() {
                None
            } else {
                Some(serde_json::Value::Object(body))
            }
        }
        _ => None,
    }
}

#[no_mangle]
pub extern "C" fn ycallr_get_last_error() -> *const c_char {
    LAST_ERROR.with(|e| {
        e.borrow()
            .as_ref()
            .map(|s| s.as_ptr())
            .unwrap_or(std::ptr::null())
    })
}

#[no_mangle]
pub extern "C" fn ycallr_error_free(err: *mut c_char) {
    if !err.is_null() {
        unsafe {
            let _ = CString::from_raw(err);
        }
    }
}

// ─── Helpers ──────────────────────────────────────────────────────────

unsafe fn cstr_to_str<'a>(ptr: *const c_char) -> Option<&'a str> {
    if ptr.is_null() {
        return None;
    }
    CStr::from_ptr(ptr).to_str().ok()
}

fn parse_required_json(ptr: *const c_char, field_name: &str) -> Result<serde_json::Value, String> {
    if ptr.is_null() {
        return Err(format!("{} is required", field_name));
    }

    let s = match unsafe { cstr_to_str(ptr) } {
        Some(s) => s,
        None => return Err(format!("Invalid UTF-8 in {}", field_name)),
    };

    serde_json::from_str(s).map_err(|e| format!("Invalid {}: {}", field_name, e))
}

/// Parse optional JSON body. Null pointer yields None; invalid UTF-8 or JSON returns an error.
fn parse_body_json(ptr: *const c_char) -> Result<Option<serde_json::Value>, String> {
    if ptr.is_null() {
        return Ok(None);
    }

    let s = match unsafe { cstr_to_str(ptr) } {
        Some(s) => s,
        None => return Err("Invalid UTF-8 in body_json".into()),
    };

    serde_json::from_str(s)
        .map(Some)
        .map_err(|e| format!("Invalid body_json: {}", e))
}

/// Parse a JSON object into HashMap<String,String>.
/// Null pointer yields an empty map; invalid UTF-8 or JSON returns an error.
fn parse_params(ptr: *const c_char) -> Result<HashMap<String, String>, String> {
    if ptr.is_null() {
        return Ok(HashMap::new());
    }

    let s = match unsafe { cstr_to_str(ptr) } {
        Some(s) => s,
        None => return Err("Invalid UTF-8 in params_json".into()),
    };

    serde_json::from_str(s).map_err(|e| format!("Invalid params_json: {}", e))
}

fn into_raw_cstring(s: String) -> *mut c_char {
    match CString::new(s) {
        Ok(c) => c.into_raw(),
        Err(_) => {
            set_last_error("String contains interior NUL byte".into());
            std::ptr::null_mut()
        }
    }
}

fn wrap_api(api: ApiDefinition) -> *mut YcallrApi {
    Box::into_raw(Box::new(YcallrApi {
        name: into_raw_cstring(api.name.clone()),
        version: into_raw_cstring(api.version.clone()),
        description: into_raw_cstring(api.description.clone()),
        base_url: into_raw_cstring(api.base_url.clone()),
        _inner: Box::new(api),
    }))
}

fn parse_runtime_auth(
    auth_type: &str,
    auth_data: &serde_json::Value,
) -> Result<AuthConfig, String> {
    let auth = match auth_type {
        "bearer" => {
            let token = auth_data
                .get("token")
                .and_then(|v| v.as_str())
                .filter(|s| !s.trim().is_empty())
                .ok_or("bearer auth requires non-empty 'token'")?;
            AuthConfig::bearer(token.to_string())
        }
        "api_key" => {
            let key = auth_data
                .get("key")
                .and_then(|v| v.as_str())
                .filter(|s| !s.trim().is_empty())
                .ok_or("api_key auth requires non-empty 'key'")?;
            let name = auth_data
                .get("name")
                .and_then(|v| v.as_str())
                .filter(|s| !s.trim().is_empty())
                .unwrap_or("X-API-Key");
            let in_str = auth_data
                .get("in")
                .and_then(|v| v.as_str())
                .unwrap_or("header");
            let in_ = match in_str {
                "header" => crate::models::ApiKeyLocation::Header,
                "query" => crate::models::ApiKeyLocation::Query,
                "cookie" => crate::models::ApiKeyLocation::Cookie,
                other => {
                    return Err(format!(
                        "Unknown api_key location '{}': expected header, query, or cookie",
                        other
                    ));
                }
            };
            AuthConfig::api_key_in(key.to_string(), name.to_string(), in_)
        }
        "http_basic" => {
            let username = auth_data
                .get("username")
                .and_then(|v| v.as_str())
                .filter(|s| !s.trim().is_empty())
                .ok_or("http_basic auth requires non-empty 'username'")?;
            let password = auth_data
                .get("password")
                .and_then(|v| v.as_str())
                .filter(|s| !s.trim().is_empty())
                .ok_or("http_basic auth requires non-empty 'password'")?;
            AuthConfig::http_basic(username.to_string(), password.to_string())
        }
        "http_custom" => {
            let prefix = auth_data
                .get("prefix")
                .and_then(|v| v.as_str())
                .filter(|s| !s.trim().is_empty())
                .ok_or("http_custom auth requires non-empty 'prefix'")?;
            let token = auth_data
                .get("token")
                .and_then(|v| v.as_str())
                .filter(|s| !s.trim().is_empty())
                .ok_or("http_custom auth requires non-empty 'token'")?;
            AuthConfig::http_custom(prefix.to_string(), token.to_string())
        }
        other => return Err(format!("Unknown auth_type: '{}'", other)),
    };

    crate::models::validate_auth_config("client", &auth).map_err(|e| e.to_string())?;

    Ok(auth)
}

// ─── ApiDefinition ────────────────────────────────────────────────────

#[repr(C)]
pub struct YcallrApi {
    name: *mut c_char,
    version: *mut c_char,
    description: *mut c_char,
    base_url: *mut c_char,
    _inner: Box<ApiDefinition>,
}

#[no_mangle]
pub extern "C" fn ycallr_parse_yaml(yaml: *const c_char) -> *mut YcallrApi {
    let yaml_str = match unsafe { cstr_to_str(yaml) } {
        Some(s) => s,
        None => {
            set_last_error("Invalid UTF-8 in YAML input".into());
            return std::ptr::null_mut();
        }
    };

    let api = match crate::profile_store::compile_yaml_str(yaml_str) {
        Ok(bytes) => match crate::profile_store::load_from_proto_bytes(&bytes) {
            Ok(api) => api,
            Err(e) => {
                set_last_error(e.to_string());
                return std::ptr::null_mut();
            }
        },
        Err(e) => {
            set_last_error(e.to_string());
            return std::ptr::null_mut();
        }
    };

    wrap_api(api)
}

/// Load a compiled profile from `~/.config/ycallr/apis/<name>.pb`.
#[no_mangle]
pub extern "C" fn ycallr_load_installed(name: *const c_char) -> *mut YcallrApi {
    let name = match unsafe { cstr_to_str(name) } {
        Some(s) => s,
        None => {
            set_last_error("Invalid UTF-8 in profile name".into());
            return std::ptr::null_mut();
        }
    };

    match crate::profile_store::load_installed_profile(name) {
        Ok(api) => wrap_api(api),
        Err(e) => {
            set_last_error(e.to_string());
            std::ptr::null_mut()
        }
    }
}

/// Decode a compiled protobuf profile from memory (e.g. embedded or custom storage).
#[no_mangle]
pub extern "C" fn ycallr_parse_proto(data: *const u8, len: usize) -> *mut YcallrApi {
    if data.is_null() || len == 0 {
        set_last_error("proto data pointer is null or empty".into());
        return std::ptr::null_mut();
    }

    let bytes = unsafe { std::slice::from_raw_parts(data, len) };
    match crate::profile_store::load_from_proto_bytes(bytes) {
        Ok(api) => wrap_api(api),
        Err(e) => {
            set_last_error(e.to_string());
            std::ptr::null_mut()
        }
    }
}

/// Compile `~/.config/ycallr/apis/<name>.yaml` to `<name>.pb`. Returns 0 on success.
#[no_mangle]
pub extern "C" fn ycallr_install(name: *const c_char) -> i32 {
    let name = match unsafe { cstr_to_str(name) } {
        Some(s) => s,
        None => {
            set_last_error("Invalid UTF-8 in profile name".into());
            return -1;
        }
    };

    match crate::profile_store::install_profile(name) {
        Ok(pb_path) => {
            set_last_install_result(name, &pb_path);
            0
        }
        Err(e) => {
            set_last_error(e.to_string());
            -1
        }
    }
}

/// Install from a YAML file path (copies into apis dir when needed). Returns 0 on success.
#[no_mangle]
pub extern "C" fn ycallr_install_yaml_file(path: *const c_char) -> i32 {
    let path_str = match unsafe { cstr_to_str(path) } {
        Some(s) => s,
        None => {
            set_last_error("Invalid UTF-8 in file path".into());
            return -1;
        }
    };

    match crate::profile_store::install_profile_from_path(std::path::Path::new(path_str)) {
        Ok((name, pb_path)) => {
            set_last_install_result(&name, &pb_path);
            0
        }
        Err(e) => {
            set_last_error(e.to_string());
            -1
        }
    }
}

#[no_mangle]
pub extern "C" fn ycallr_free_api(api: *mut YcallrApi) {
    if !api.is_null() {
        unsafe {
            let _ = from_raw_cstring((*api).name);
            let _ = from_raw_cstring((*api).version);
            let _ = from_raw_cstring((*api).description);
            let _ = from_raw_cstring((*api).base_url);
            let _ = Box::from_raw(api);
        }
    }
}

/// Override base URL at runtime (e.g. point profile at a mock server). Returns 0 on success, -1 on error.
#[no_mangle]
pub extern "C" fn ycallr_set_base_url(api: *mut YcallrApi, url: *const c_char) -> i32 {
    if api.is_null() {
        set_last_error("Null API pointer".into());
        return -1;
    }

    let url_str = match unsafe { cstr_to_str(url) } {
        Some(s) => s,
        None => {
            set_last_error("Invalid UTF-8 in base_url".into());
            return -1;
        }
    };

    if url_str.trim().is_empty() {
        set_last_error("Base URL cannot be empty".into());
        return -1;
    }

    unsafe {
        (*api)._inner.base_url = url_str.to_string();
        let old = (*api).base_url;
        (*api).base_url = into_raw_cstring((*api)._inner.base_url.clone());
        if !old.is_null() {
            let _ = from_raw_cstring(old);
        }
    }

    0
}

#[no_mangle]
pub extern "C" fn ycallr_get_name(api: *const YcallrApi) -> *const c_char {
    if api.is_null() {
        return std::ptr::null();
    }
    unsafe { (*api).name }
}

#[no_mangle]
pub extern "C" fn ycallr_get_version(api: *const YcallrApi) -> *const c_char {
    if api.is_null() {
        return std::ptr::null();
    }
    unsafe { (*api).version }
}

#[no_mangle]
pub extern "C" fn ycallr_get_base_url(api: *const YcallrApi) -> *const c_char {
    if api.is_null() {
        return std::ptr::null();
    }
    unsafe { (*api).base_url }
}

#[no_mangle]
pub extern "C" fn ycallr_get_description(api: *const YcallrApi) -> *const c_char {
    if api.is_null() {
        return std::ptr::null();
    }
    unsafe { (*api).description }
}

/// Returns a JSON array of command names at the top level: `["repos","users"]`
#[no_mangle]
pub extern "C" fn ycallr_list_commands(api: *const YcallrApi) -> *mut c_char {
    if api.is_null() {
        return std::ptr::null_mut();
    }
    let inner = unsafe { &(*api)._inner };
    let names: Vec<&str> = inner.commands.keys().map(|s| s.as_str()).collect();
    match serde_json::to_string(&names) {
        Ok(json) => into_raw_cstring(json),
        Err(e) => {
            set_last_error(format!("Failed to serialize command list: {}", e));
            std::ptr::null_mut()
        }
    }
}

/// Returns JSON array of installed profile names: `["github","demo"]`.
#[no_mangle]
pub extern "C" fn ycallr_list_installed() -> *mut c_char {
    match crate::profile_store::list_installed_profile_names() {
        Ok(names) => match serde_json::to_string(&names) {
            Ok(json) => into_raw_cstring(json),
            Err(e) => {
                set_last_error(format!("Failed to serialize installed list: {}", e));
                std::ptr::null_mut()
            }
        },
        Err(e) => {
            set_last_error(e.to_string());
            std::ptr::null_mut()
        }
    }
}

/// After `ycallr_install` / `ycallr_install_yaml_file`: `{"name":"...","pb_path":"..."}`.
#[no_mangle]
pub extern "C" fn ycallr_get_last_install_result() -> *mut c_char {
    LAST_INSTALL_RESULT.with(|r| match r.borrow().as_ref() {
        Some(json) => into_raw_cstring(json.clone()),
        None => std::ptr::null_mut(),
    })
}

/// Returns filesystem path to `~/.config/ycallr/apis/<name>.pb`.
#[no_mangle]
pub extern "C" fn ycallr_compiled_profile_path(name: *const c_char) -> *mut c_char {
    let name = match unsafe { cstr_to_str(name) } {
        Some(s) => s,
        None => {
            set_last_error("Invalid UTF-8 in profile name".into());
            return std::ptr::null_mut();
        }
    };
    let path = crate::profile_store::compiled_profile_path(name);
    into_raw_cstring(path.to_string_lossy().into_owned())
}

/// Returns JSON array of subcommand names for `path` (use empty string for top level).
#[no_mangle]
pub extern "C" fn ycallr_list_subcommands(
    api: *const YcallrApi,
    path: *const c_char,
) -> *mut c_char {
    if api.is_null() {
        return std::ptr::null_mut();
    }

    let inner = unsafe { &(*api)._inner };
    let path_str = if path.is_null() {
        ""
    } else {
        match unsafe { cstr_to_str(path) } {
            Some(s) => s,
            None => {
                set_last_error("Invalid UTF-8 in command path".into());
                return std::ptr::null_mut();
            }
        }
    };

    let names = if path_str.is_empty() {
        let mut names: Vec<String> = inner.commands.keys().cloned().collect();
        names.sort();
        names
    } else {
        match inner.list_subcommands(path_str) {
            Ok(names) => names,
            Err(e) => {
                set_last_error(e.to_string());
                return std::ptr::null_mut();
            }
        }
    };

    match serde_json::to_string(&names) {
        Ok(json) => into_raw_cstring(json),
        Err(e) => {
            set_last_error(format!("Failed to serialize subcommands: {}", e));
            std::ptr::null_mut()
        }
    }
}

/// Returns JSON array of missing required parameter names before a call.
#[no_mangle]
pub extern "C" fn ycallr_missing_params_json(
    api: *const YcallrApi,
    command_path: *const c_char,
    params_json: *const c_char,
) -> *mut c_char {
    if api.is_null() {
        set_last_error("Null API pointer".into());
        return std::ptr::null_mut();
    }

    let path_str = match unsafe { cstr_to_str(command_path) } {
        Some(s) => s,
        None => {
            set_last_error("Invalid UTF-8 in command path".into());
            return std::ptr::null_mut();
        }
    };

    let params = match parse_params(params_json) {
        Ok(p) => p,
        Err(err) => {
            set_last_error(err);
            return std::ptr::null_mut();
        }
    };

    let inner = unsafe { (*api)._inner.as_ref() };
    match compute_missing_params_json(inner, path_str, &params) {
        Ok(json) => into_raw_cstring(json),
        Err(err) => {
            set_last_error(err);
            std::ptr::null_mut()
        }
    }
}

/// Builds implicit JSON body for POST/PUT/PATCH when YAML has no body; NULL if none.
#[no_mangle]
pub extern "C" fn ycallr_build_implicit_body_json(
    api: *const YcallrApi,
    command_path: *const c_char,
    params_json: *const c_char,
) -> *mut c_char {
    if api.is_null() {
        set_last_error("Null API pointer".into());
        return std::ptr::null_mut();
    }

    let path_str = match unsafe { cstr_to_str(command_path) } {
        Some(s) => s,
        None => {
            set_last_error("Invalid UTF-8 in command path".into());
            return std::ptr::null_mut();
        }
    };

    let params = match parse_params(params_json) {
        Ok(p) => p,
        Err(err) => {
            set_last_error(err);
            return std::ptr::null_mut();
        }
    };

    let inner = unsafe { (*api)._inner.as_ref() };
    match compute_implicit_body_json(inner, path_str, &params) {
        Some(json) => into_raw_cstring(json),
        None => std::ptr::null_mut(),
    }
}

// ─── Command ──────────────────────────────────────────────────────────

pub struct YcallrCommand {
    endpoint: Option<String>,
    method: Option<String>,
    description: Option<String>,
    is_leaf: bool,
    is_branch: bool,
    has_body: bool,
    path_params: Vec<String>,
    auth: Option<String>,
    headers_json: String,
    params_json: String,
}

#[no_mangle]
pub extern "C" fn ycallr_get_command(
    api: *const YcallrApi,
    path: *const c_char,
) -> *mut YcallrCommand {
    if api.is_null() || path.is_null() {
        return std::ptr::null_mut();
    }
    let inner = unsafe { &(*api)._inner };
    let path_str = match unsafe { cstr_to_str(path) } {
        Some(s) => s,
        None => {
            set_last_error("Invalid UTF-8 in command path".into());
            return std::ptr::null_mut();
        }
    };

    let cmd = match inner.get_command(path_str) {
        Ok(c) => c,
        Err(e) => {
            set_last_error(e.to_string());
            return std::ptr::null_mut();
        }
    };

    let headers_json = serde_json::to_string(&cmd.headers).unwrap_or_default();
    let params_json = serde_json::to_string(&cmd.params).unwrap_or_default();

    Box::into_raw(Box::new(YcallrCommand {
        endpoint: cmd.endpoint.clone(),
        method: cmd.method.as_ref().map(|m| m.as_str().to_string()),
        description: cmd.description.clone(),
        is_leaf: cmd.is_leaf(),
        is_branch: cmd.is_branch(),
        has_body: cmd.body.is_some(),
        path_params: cmd.endpoint_path_param_names(),
        auth: cmd.auth.clone(),
        headers_json,
        params_json,
    }))
}

#[no_mangle]
pub extern "C" fn ycallr_free_command(cmd: *mut YcallrCommand) {
    if !cmd.is_null() {
        unsafe {
            let _ = Box::from_raw(cmd);
        }
    }
}

#[no_mangle]
pub extern "C" fn ycallr_command_get_endpoint(cmd: *const YcallrCommand) -> *mut c_char {
    if cmd.is_null() {
        return std::ptr::null_mut();
    }
    unsafe {
        match &(*cmd).endpoint {
            Some(s) => into_raw_cstring(s.clone()),
            None => std::ptr::null_mut(),
        }
    }
}

#[no_mangle]
pub extern "C" fn ycallr_command_get_method(cmd: *const YcallrCommand) -> *mut c_char {
    if cmd.is_null() {
        return std::ptr::null_mut();
    }
    unsafe {
        match &(*cmd).method {
            Some(s) => into_raw_cstring(s.clone()),
            None => std::ptr::null_mut(),
        }
    }
}

#[no_mangle]
pub extern "C" fn ycallr_command_get_description(cmd: *const YcallrCommand) -> *mut c_char {
    if cmd.is_null() {
        return std::ptr::null_mut();
    }
    unsafe {
        match &(*cmd).description {
            Some(s) => into_raw_cstring(s.clone()),
            None => std::ptr::null_mut(),
        }
    }
}

#[no_mangle]
pub extern "C" fn ycallr_command_get_auth(cmd: *const YcallrCommand) -> *mut c_char {
    if cmd.is_null() {
        return std::ptr::null_mut();
    }
    unsafe {
        match &(*cmd).auth {
            Some(s) => into_raw_cstring(s.clone()),
            None => std::ptr::null_mut(),
        }
    }
}

#[no_mangle]
pub extern "C" fn ycallr_command_get_headers_json(cmd: *const YcallrCommand) -> *mut c_char {
    if cmd.is_null() {
        return std::ptr::null_mut();
    }
    let s = unsafe { (*cmd).headers_json.clone() };
    into_raw_cstring(s)
}

#[no_mangle]
pub extern "C" fn ycallr_command_get_params_json(cmd: *const YcallrCommand) -> *mut c_char {
    if cmd.is_null() {
        return std::ptr::null_mut();
    }
    let s = unsafe { (*cmd).params_json.clone() };
    into_raw_cstring(s)
}

#[no_mangle]
pub extern "C" fn ycallr_command_is_leaf(cmd: *const YcallrCommand) -> bool {
    if cmd.is_null() {
        return false;
    }
    unsafe { (*cmd).is_leaf }
}

#[no_mangle]
pub extern "C" fn ycallr_command_is_branch(cmd: *const YcallrCommand) -> bool {
    if cmd.is_null() {
        return false;
    }
    unsafe { (*cmd).is_branch }
}

#[no_mangle]
pub extern "C" fn ycallr_command_has_body(cmd: *const YcallrCommand) -> bool {
    if cmd.is_null() {
        return false;
    }
    unsafe { (*cmd).has_body }
}

/// Returns JSON array of path parameter names from the endpoint template.
#[no_mangle]
pub extern "C" fn ycallr_command_get_path_params_json(cmd: *const YcallrCommand) -> *mut c_char {
    if cmd.is_null() {
        return std::ptr::null_mut();
    }
    let names = unsafe { (*cmd).path_params.clone() };
    match serde_json::to_string(&names) {
        Ok(json) => into_raw_cstring(json),
        Err(e) => {
            set_last_error(format!("Failed to serialize path params: {}", e));
            std::ptr::null_mut()
        }
    }
}

// ─── Client ───────────────────────────────────────────────────────────

pub struct YcallrClientWrapper {
    client: YcallrClient,
}

/// Create a client. env_mode: 0=Auto, 1=Manual. envs_json: `{"KEY":"val"}` or NULL.
#[no_mangle]
pub extern "C" fn ycallr_client_new(
    api: *const YcallrApi,
    env_mode: u8,
    envs_json: *const c_char,
) -> *mut YcallrClientWrapper {
    if api.is_null() {
        set_last_error("Null API pointer".into());
        return std::ptr::null_mut();
    }

    let api_def = unsafe { (*api)._inner.as_ref().clone() };

    let mode = match env_mode {
        0 => EnvMode::Auto,
        1 => EnvMode::Manual,
        _ => {
            set_last_error("Invalid env_mode: expected 0 (Auto) or 1 (Manual)".into());
            return std::ptr::null_mut();
        }
    };

    let envs = match parse_params(envs_json) {
        Ok(vars) => vars,
        Err(err) => {
            set_last_error(err);
            return std::ptr::null_mut();
        }
    };

    let mut builder = YcallrClient::builder(api_def).env_mode(mode);

    if !envs.is_empty() {
        builder = builder.envs(envs);
    }

    match builder.build() {
        Ok(client) => Box::into_raw(Box::new(YcallrClientWrapper { client })),
        Err(e) => {
            set_last_error(e.to_string());
            std::ptr::null_mut()
        }
    }
}

/// Create a client with auth. auth_type: "bearer"|"api_key"|"http_basic"|"http_custom".
/// auth_data_json varies by type:
///   bearer:       {"token":"xxx"}
///   api_key:      {"key":"xxx","name":"X-API-Key","in":"header"|"query"|"cookie"}
///   http_basic:   {"username":"u","password":"p"}
///   http_custom:  {"prefix":"xxx","token":"yyy"}
#[no_mangle]
pub extern "C" fn ycallr_client_new_with_auth(
    api: *const YcallrApi,
    auth_type: *const c_char,
    auth_data_json: *const c_char,
    env_mode: u8,
    envs_json: *const c_char,
) -> *mut YcallrClientWrapper {
    if api.is_null() {
        set_last_error("Null API pointer".into());
        return std::ptr::null_mut();
    }

    let auth_type_str = match unsafe { cstr_to_str(auth_type) } {
        Some(s) => s,
        None => {
            set_last_error("Invalid UTF-8 in auth_type".into());
            return std::ptr::null_mut();
        }
    };

    let auth_data = match parse_required_json(auth_data_json, "auth_data_json") {
        Ok(data) => data,
        Err(err) => {
            set_last_error(err);
            return std::ptr::null_mut();
        }
    };

    let auth_config = match parse_runtime_auth(auth_type_str, &auth_data) {
        Ok(auth) => auth,
        Err(err) => {
            set_last_error(err);
            return std::ptr::null_mut();
        }
    };

    let api_def = unsafe { (*api)._inner.as_ref().clone() };

    let mode = match env_mode {
        0 => EnvMode::Auto,
        1 => EnvMode::Manual,
        _ => {
            set_last_error("Invalid env_mode: expected 0 (Auto) or 1 (Manual)".into());
            return std::ptr::null_mut();
        }
    };

    let envs = match parse_params(envs_json) {
        Ok(vars) => vars,
        Err(err) => {
            set_last_error(err);
            return std::ptr::null_mut();
        }
    };

    let mut builder = YcallrClient::builder(api_def)
        .env_mode(mode)
        .auth(auth_config);

    if !envs.is_empty() {
        builder = builder.envs(envs);
    }

    match builder.build() {
        Ok(client) => Box::into_raw(Box::new(YcallrClientWrapper { client })),
        Err(e) => {
            set_last_error(e.to_string());
            std::ptr::null_mut()
        }
    }
}

#[no_mangle]
pub extern "C" fn ycallr_client_free(client: *mut YcallrClientWrapper) {
    if !client.is_null() {
        unsafe {
            let _ = Box::from_raw(client);
        }
    }
}

// ─── Response ─────────────────────────────────────────────────────────

pub struct YcallrResponse {
    status: u16,
    headers_json: String,
    body_json: String,
    message: Option<String>,
}

#[no_mangle]
pub extern "C" fn ycallr_call(
    client: *const YcallrClientWrapper,
    command: *const c_char,
    params_json: *const c_char,
    body_json: *const c_char,
) -> *mut YcallrResponse {
    if client.is_null() {
        set_last_error("Null client pointer".into());
        return std::ptr::null_mut();
    }
    let command_str = match unsafe { cstr_to_str(command) } {
        Some(s) => s,
        None => {
            set_last_error("Invalid UTF-8 in command".into());
            return std::ptr::null_mut();
        }
    };

    let params = match parse_params(params_json) {
        Ok(params) => params,
        Err(err) => {
            set_last_error(err);
            return std::ptr::null_mut();
        }
    };
    let body = match parse_body_json(body_json) {
        Ok(body) => body,
        Err(err) => {
            set_last_error(err);
            return std::ptr::null_mut();
        }
    };

    let body_ref = body.as_ref();

    let resp = match unsafe { &(*client).client }.call(command_str, &params, body_ref) {
        Ok(r) => r,
        Err(e) => {
            set_last_error(e.to_string());
            return std::ptr::null_mut();
        }
    };

    let headers_json = serde_json::to_string(&resp.headers).unwrap_or_default();
    let body_json_str = resp.body.to_string();

    Box::into_raw(Box::new(YcallrResponse {
        status: resp.status,
        headers_json,
        body_json: body_json_str,
        message: resp.message,
    }))
}

#[no_mangle]
pub extern "C" fn ycallr_free_response(resp: *mut YcallrResponse) {
    if !resp.is_null() {
        unsafe {
            let _ = Box::from_raw(resp);
        }
    }
}

#[no_mangle]
pub extern "C" fn ycallr_response_get_status(resp: *const YcallrResponse) -> u16 {
    if resp.is_null() {
        return 0;
    }
    unsafe { (*resp).status }
}

/// Returns headers as JSON object: `{"content-type":"application/json",...}`.
/// Caller must free with ycallr_string_free().
#[no_mangle]
pub extern "C" fn ycallr_response_get_headers_json(resp: *const YcallrResponse) -> *mut c_char {
    if resp.is_null() {
        return std::ptr::null_mut();
    }
    let s = unsafe { (*resp).headers_json.clone() };
    into_raw_cstring(s)
}

/// Returns body as JSON string. Caller must free with ycallr_string_free().
#[no_mangle]
pub extern "C" fn ycallr_response_get_body_json(resp: *const YcallrResponse) -> *mut c_char {
    if resp.is_null() {
        return std::ptr::null_mut();
    }
    let s = unsafe { (*resp).body_json.clone() };
    into_raw_cstring(s)
}

/// Returns response message (if configured in YAML) or NULL.
/// Caller must free with ycallr_string_free() if non-null.
#[no_mangle]
pub extern "C" fn ycallr_response_get_message(resp: *const YcallrResponse) -> *mut c_char {
    if resp.is_null() {
        return std::ptr::null_mut();
    }
    match unsafe { &(*resp).message } {
        Some(s) => into_raw_cstring(s.clone()),
        None => std::ptr::null_mut(),
    }
}

/// Free a string returned by any ycallr_response_get_* or ycallr_list_commands.
#[no_mangle]
pub extern "C" fn ycallr_string_free(s: *mut c_char) {
    if !s.is_null() {
        unsafe {
            let _ = CString::from_raw(s);
        }
    }
}

#[cfg(test)]
mod ffi_helper_tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_missing_required_params_helper() {
        let yaml = r#"
name: g
version: "1"
base_url: https://api.example.com
commands:
  get-repo:
    endpoint: /repos/{owner}/{repo}
    method: GET
    params:
      owner:
        description: o
        type: string
        required: true
      repo:
        description: r
        type: string
        required: true
"#;
        let bytes = crate::profile_store::compile_yaml_str(yaml).unwrap();
        let def = crate::profile_store::load_from_proto_bytes(&bytes).unwrap();
        let mut params = HashMap::new();
        params.insert("owner".to_string(), "o".to_string());
        let json = compute_missing_params_json(&def, "get-repo", &params).unwrap();
        assert_eq!(json, r#"["repo"]"#);
    }
}

unsafe fn from_raw_cstring(ptr: *mut c_char) -> CString {
    CString::from_raw(ptr)
}
