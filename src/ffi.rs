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
}

fn set_last_error(msg: String) {
    let sanitized = msg.replace('\0', "");
    LAST_ERROR.with(|e| {
        *e.borrow_mut() = CString::new(sanitized).ok();
    });
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

    let api = match crate::yaml_parser::parse_yaml(yaml_str) {
        Ok(a) => a,
        Err(e) => {
            set_last_error(e.to_string());
            return std::ptr::null_mut();
        }
    };

    if let Err(e) = api.validate() {
        set_last_error(e.to_string());
        return std::ptr::null_mut();
    }

    Box::into_raw(Box::new(YcallrApi {
        name: into_raw_cstring(api.name.clone()),
        version: into_raw_cstring(api.version.clone()),
        description: into_raw_cstring(api.description.clone()),
        base_url: into_raw_cstring(api.base_url.clone()),
        _inner: Box::new(api),
    }))
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

// ─── Command ──────────────────────────────────────────────────────────

pub struct YcallrCommand {
    endpoint: Option<String>,
    method: Option<String>,
    description: Option<String>,
    is_leaf: bool,
    is_branch: bool,
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

// ─── Keep from_raw_cstring for internal use ───────────────────────────

unsafe fn from_raw_cstring(ptr: *mut c_char) -> CString {
    CString::from_raw(ptr)
}
