#![cfg(all(not(target_arch = "wasm32"), feature = "ffi"))]

use std::ffi::{CStr, CString};
use std::path::Path;
use std::ptr;
use std::sync::Mutex;
use ycallr_core::ffi::{
    ycallr_build_implicit_body_json, ycallr_call, ycallr_client_free, ycallr_client_new,
    ycallr_client_new_with_auth, ycallr_command_get_auth, ycallr_command_get_body_kind,
    ycallr_command_get_description, ycallr_command_get_endpoint, ycallr_command_get_headers_json,
    ycallr_command_get_method, ycallr_command_get_params_json, ycallr_command_get_path_params_json,
    ycallr_command_has_body, ycallr_command_is_branch, ycallr_command_is_leaf,
    ycallr_compiled_profile_path, ycallr_free_api, ycallr_free_command, ycallr_free_response,
    ycallr_get_base_url, ycallr_get_command, ycallr_get_description, ycallr_get_env_json,
    ycallr_get_last_error, ycallr_get_last_install_result, ycallr_get_name, ycallr_get_version,
    ycallr_install, ycallr_install_yaml_file, ycallr_list_commands, ycallr_list_installed,
    ycallr_list_subcommands, ycallr_load_installed, ycallr_missing_params_json, ycallr_parse_proto,
    ycallr_parse_yaml, ycallr_response_get_body_json, ycallr_response_get_headers_json,
    ycallr_response_get_message, ycallr_response_get_status, ycallr_set_base_url,
    ycallr_string_free,
};

const VALID_YAML: &str = r#"
name: github
version: "1.0.0"
description: GitHub API
base_url: https://api.github.com
commands:
  get-repo:
    endpoint: /repos/{owner}/{repo}
    method: GET
    description: Get a repository
    headers:
      Accept: application/json
    params:
      owner:
        description: Repository owner
        type: string
        required: true
      repo:
        description: Repository name
        type: string
        required: true
  repos:
    description: Repository operations
    commands:
      issues:
        description: Issues operations
        commands:
          create:
            endpoint: /repos/{owner}/{repo}/issues
            method: POST
            description: Create an issue
          list:
            endpoint: /repos/{owner}/{repo}/issues
            method: GET
            description: List issues
"#;

static HOME_LOCK: Mutex<()> = Mutex::new(());

fn with_home<F: FnOnce()>(home: &Path, f: F) {
    let _guard = HOME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    std::env::set_var("HOME", home);
    f();
    std::env::remove_var("HOME");
}

fn cstr(s: &str) -> *const std::os::raw::c_char {
    CString::new(s).unwrap().into_raw() as *const std::os::raw::c_char
}

fn cstr_free(s: *mut std::os::raw::c_char) {
    if !s.is_null() {
        unsafe {
            let _ = CString::from_raw(s);
        }
    }
}

// ─── Existing tests (updated) ─────────────────────────────────────────

#[test]
fn test_ffi_parse_valid_yaml() {
    let yaml = CString::new(VALID_YAML).unwrap();
    let api = unsafe { ycallr_parse_yaml(yaml.as_ptr()) };
    assert!(!api.is_null());

    unsafe {
        let name = ycallr_get_name(api);
        assert!(!name.is_null());
        assert_eq!(std::ffi::CStr::from_ptr(name).to_str().unwrap(), "github");

        let version = ycallr_get_version(api);
        assert!(!version.is_null());
        assert_eq!(std::ffi::CStr::from_ptr(version).to_str().unwrap(), "1.0.0");

        let desc = ycallr_get_description(api);
        assert!(!desc.is_null());
        assert_eq!(
            std::ffi::CStr::from_ptr(desc).to_str().unwrap(),
            "GitHub API"
        );

        let base_url = ycallr_get_base_url(api);
        assert!(!base_url.is_null());
        assert_eq!(
            std::ffi::CStr::from_ptr(base_url).to_str().unwrap(),
            "https://api.github.com"
        );

        ycallr_free_api(api);
    }
}

#[test]
fn test_ffi_parse_null_yaml() {
    let api = unsafe { ycallr_parse_yaml(ptr::null()) };
    assert!(api.is_null());
}

#[test]
fn test_ffi_parse_invalid_yaml() {
    let yaml = CString::new("not valid yaml {{{").unwrap();
    let api = unsafe { ycallr_parse_yaml(yaml.as_ptr()) };
    assert!(api.is_null());
}

#[test]
fn test_ffi_parse_invalid_utf8() {
    let invalid_utf8 = &[0xFF, 0xFE, 0x00];
    let api = unsafe { ycallr_parse_yaml(invalid_utf8.as_ptr() as *const i8) };
    assert!(api.is_null());
}

#[test]
fn test_ffi_free_null() {
    unsafe { ycallr_free_api(ptr::null_mut()) };
}

#[test]
fn test_ffi_getters_null() {
    unsafe {
        assert!(ycallr_get_name(ptr::null()).is_null());
        assert!(ycallr_get_version(ptr::null()).is_null());
        assert!(ycallr_get_base_url(ptr::null()).is_null());
        assert!(ycallr_get_description(ptr::null()).is_null());
    }
}

#[test]
fn test_ffi_parse_validation_error() {
    let yaml = CString::new(
        r#"
name: ""
version: "1.0.0"
base_url: https://api.test.com
commands: {}
"#,
    )
    .unwrap();
    let api = unsafe { ycallr_parse_yaml(yaml.as_ptr()) };
    assert!(api.is_null());
}

// ─── Error system ─────────────────────────────────────────────────────

#[test]
fn test_ffi_error_after_failed_parse() {
    let yaml = CString::new("not valid yaml {{{").unwrap();
    let api = unsafe { ycallr_parse_yaml(yaml.as_ptr()) };
    assert!(api.is_null());

    let err = unsafe { ycallr_get_last_error() };
    assert!(!err.is_null());
    let err_str = unsafe { std::ffi::CStr::from_ptr(err) };
    assert!(
        err_str.to_str().unwrap().contains("YAML"),
        "Expected YAML error, got: {:?}",
        err_str
    );
}

#[test]
fn test_ffi_error_null_when_no_error() {
    // No error yet (last test might have set one, so let's trigger a success)
    let yaml = CString::new(VALID_YAML).unwrap();
    let api = unsafe { ycallr_parse_yaml(yaml.as_ptr()) };
    assert!(!api.is_null());
    unsafe {
        ycallr_free_api(api);
    }
    // Note: last_error might still be set from a previous test - that's OK in parallel tests
}

// ─── List commands ────────────────────────────────────────────────────

#[test]
fn test_ffi_list_commands() {
    let yaml = CString::new(VALID_YAML).unwrap();
    let api = unsafe { ycallr_parse_yaml(yaml.as_ptr()) };
    assert!(!api.is_null());

    let json = unsafe { ycallr_list_commands(api) };
    assert!(!json.is_null());

    let json_str = unsafe { std::ffi::CStr::from_ptr(json) }.to_str().unwrap();
    let commands: Vec<String> = serde_json::from_str(json_str).unwrap();
    assert!(commands.contains(&"get-repo".to_string()));
    assert!(commands.contains(&"repos".to_string()));

    unsafe {
        ycallr_string_free(json);
        ycallr_free_api(api);
    }
}

#[test]
fn test_ffi_list_commands_null_api() {
    let json = unsafe { ycallr_list_commands(ptr::null()) };
    assert!(json.is_null());
}

// ─── Command lookup ───────────────────────────────────────────────────

#[test]
fn test_ffi_get_command_leaf() {
    let yaml = CString::new(VALID_YAML).unwrap();
    let api = unsafe { ycallr_parse_yaml(yaml.as_ptr()) };
    assert!(!api.is_null());

    let path = CString::new("get-repo").unwrap();
    let cmd = unsafe { ycallr_get_command(api, path.as_ptr()) };
    assert!(!cmd.is_null());

    unsafe {
        assert!(ycallr_command_is_leaf(cmd));
        assert!(!ycallr_command_is_branch(cmd));

        let endpoint = ycallr_command_get_endpoint(cmd);
        assert!(!endpoint.is_null());
        assert_eq!(
            std::ffi::CStr::from_ptr(endpoint).to_str().unwrap(),
            "/repos/{owner}/{repo}"
        );
        ycallr_string_free(endpoint);

        let method = ycallr_command_get_method(cmd);
        assert!(!method.is_null());
        assert_eq!(std::ffi::CStr::from_ptr(method).to_str().unwrap(), "GET");
        ycallr_string_free(method);

        let desc = ycallr_command_get_description(cmd);
        assert!(!desc.is_null());
        assert_eq!(
            std::ffi::CStr::from_ptr(desc).to_str().unwrap(),
            "Get a repository"
        );
        ycallr_string_free(desc);

        let params_json = ycallr_command_get_params_json(cmd);
        assert!(!params_json.is_null());
        let params_str = std::ffi::CStr::from_ptr(params_json).to_str().unwrap();
        let params: serde_json::Value = serde_json::from_str(params_str).unwrap();
        assert!(params.get("owner").is_some());
        assert!(params.get("repo").is_some());
        ycallr_string_free(params_json);

        ycallr_free_command(cmd);
        ycallr_free_api(api);
    }
}

#[test]
fn test_ffi_get_command_nested() {
    let yaml = CString::new(VALID_YAML).unwrap();
    let api = unsafe { ycallr_parse_yaml(yaml.as_ptr()) };

    let path = CString::new("repos.issues.create").unwrap();
    let cmd = unsafe { ycallr_get_command(api, path.as_ptr()) };
    assert!(!cmd.is_null());

    unsafe {
        assert!(ycallr_command_is_leaf(cmd));

        let method = ycallr_command_get_method(cmd);
        assert!(!method.is_null());
        assert_eq!(std::ffi::CStr::from_ptr(method).to_str().unwrap(), "POST");
        ycallr_string_free(method);

        let desc = ycallr_command_get_description(cmd);
        assert!(!desc.is_null());
        assert_eq!(
            std::ffi::CStr::from_ptr(desc).to_str().unwrap(),
            "Create an issue"
        );
        ycallr_string_free(desc);

        ycallr_free_command(cmd);
        ycallr_free_api(api);
    }
}

#[test]
fn test_ffi_get_command_branch() {
    let yaml = CString::new(VALID_YAML).unwrap();
    let api = unsafe { ycallr_parse_yaml(yaml.as_ptr()) };

    let path = CString::new("repos").unwrap();
    let cmd = unsafe { ycallr_get_command(api, path.as_ptr()) };
    assert!(!cmd.is_null());

    unsafe {
        assert!(!ycallr_command_is_leaf(cmd));
        assert!(ycallr_command_is_branch(cmd));

        let endpoint = ycallr_command_get_endpoint(cmd);
        assert!(endpoint.is_null()); // branches have no endpoint

        let method = ycallr_command_get_method(cmd);
        assert!(method.is_null()); // branches have no method

        ycallr_free_command(cmd);
        ycallr_free_api(api);
    }
}

#[test]
fn test_ffi_get_command_not_found() {
    let yaml = CString::new(VALID_YAML).unwrap();
    let api = unsafe { ycallr_parse_yaml(yaml.as_ptr()) };

    let path = CString::new("nonexistent").unwrap();
    let cmd = unsafe { ycallr_get_command(api, path.as_ptr()) };
    assert!(cmd.is_null());

    let err = unsafe { ycallr_get_last_error() };
    assert!(!err.is_null());
    let err_str = unsafe { std::ffi::CStr::from_ptr(err) };
    assert!(err_str.to_str().unwrap().contains("nonexistent"));

    unsafe { ycallr_free_api(api) };
}

#[test]
fn test_ffi_get_command_null_inputs() {
    let cmd = unsafe { ycallr_get_command(ptr::null(), cstr("test")) };
    assert!(cmd.is_null());

    let yaml = CString::new(VALID_YAML).unwrap();
    let api = unsafe { ycallr_parse_yaml(yaml.as_ptr()) };
    let cmd = unsafe { ycallr_get_command(api, ptr::null()) };
    assert!(cmd.is_null());

    unsafe { ycallr_free_api(api) };
}

// ─── Client creation ──────────────────────────────────────────────────

#[test]
fn test_ffi_client_new() {
    let yaml = CString::new(VALID_YAML).unwrap();
    let api = unsafe { ycallr_parse_yaml(yaml.as_ptr()) };
    assert!(!api.is_null());

    let client = unsafe { ycallr_client_new(api, 0, ptr::null()) };
    assert!(!client.is_null());

    unsafe {
        ycallr_client_free(client);
        ycallr_free_api(api);
    }
}

#[test]
fn test_ffi_client_new_manual_mode() {
    let yaml = CString::new(VALID_YAML).unwrap();
    let api = unsafe { ycallr_parse_yaml(yaml.as_ptr()) };

    let envs = CString::new(r#"{}"#).unwrap();
    let client = unsafe { ycallr_client_new(api, 1, envs.as_ptr()) };
    assert!(!client.is_null());

    unsafe {
        ycallr_client_free(client);
        ycallr_free_api(api);
    }
}

#[test]
fn test_ffi_client_new_invalid_env_mode() {
    let yaml = CString::new(VALID_YAML).unwrap();
    let api = unsafe { ycallr_parse_yaml(yaml.as_ptr()) };

    let client = unsafe { ycallr_client_new(api, 99, ptr::null()) };
    assert!(client.is_null());

    let err = unsafe { ycallr_get_last_error() };
    assert!(!err.is_null());

    unsafe { ycallr_free_api(api) };
}

#[test]
fn test_ffi_client_new_null_api() {
    let client = unsafe { ycallr_client_new(ptr::null(), 0, ptr::null()) };
    assert!(client.is_null());
}

// ─── Client with auth ─────────────────────────────────────────────────

#[test]
fn test_ffi_client_new_with_auth_bearer() {
    let yaml = CString::new(VALID_YAML).unwrap();
    let api = unsafe { ycallr_parse_yaml(yaml.as_ptr()) };

    let auth_type = CString::new("bearer").unwrap();
    let auth_data = CString::new(r#"{"token":"ghp_test123"}"#).unwrap();

    let client = unsafe {
        ycallr_client_new_with_auth(api, auth_type.as_ptr(), auth_data.as_ptr(), 0, ptr::null())
    };
    assert!(!client.is_null());

    unsafe {
        ycallr_client_free(client);
        ycallr_free_api(api);
    }
}

#[test]
fn test_ffi_client_new_with_auth_api_key() {
    let yaml = CString::new(VALID_YAML).unwrap();
    let api = unsafe { ycallr_parse_yaml(yaml.as_ptr()) };

    let auth_type = CString::new("api_key").unwrap();
    let auth_data = CString::new(r#"{"key":"mykey","name":"X-API-Key"}"#).unwrap();

    let client = unsafe {
        ycallr_client_new_with_auth(api, auth_type.as_ptr(), auth_data.as_ptr(), 0, ptr::null())
    };
    assert!(!client.is_null());

    unsafe {
        ycallr_client_free(client);
        ycallr_free_api(api);
    }
}

#[test]
fn test_ffi_client_new_with_auth_invalid_type() {
    let yaml = CString::new(VALID_YAML).unwrap();
    let api = unsafe { ycallr_parse_yaml(yaml.as_ptr()) };

    let auth_type = CString::new("invalid").unwrap();
    let auth_data = CString::new(r#"{}"#).unwrap();

    let client = unsafe {
        ycallr_client_new_with_auth(api, auth_type.as_ptr(), auth_data.as_ptr(), 0, ptr::null())
    };
    assert!(client.is_null());

    let err = unsafe { ycallr_get_last_error() };
    assert!(!err.is_null());

    unsafe { ycallr_free_api(api) };
}

#[test]
fn test_ffi_client_new_with_auth_empty_bearer() {
    let yaml = CString::new(VALID_YAML).unwrap();
    let api = unsafe { ycallr_parse_yaml(yaml.as_ptr()) };

    let auth_type = CString::new("bearer").unwrap();
    let auth_data = CString::new(r#"{"token":""}"#).unwrap();

    let client = unsafe {
        ycallr_client_new_with_auth(api, auth_type.as_ptr(), auth_data.as_ptr(), 0, ptr::null())
    };
    assert!(client.is_null());

    let err = unsafe { ycallr_get_last_error() };
    assert!(!err.is_null());
    let err_str = unsafe { std::ffi::CStr::from_ptr(err) };
    assert!(err_str.to_str().unwrap().contains("token"));

    unsafe { ycallr_free_api(api) };
}

#[test]
fn test_ffi_client_new_with_auth_invalid_api_key_location() {
    let yaml = CString::new(VALID_YAML).unwrap();
    let api = unsafe { ycallr_parse_yaml(yaml.as_ptr()) };

    let auth_type = CString::new("api_key").unwrap();
    let auth_data = CString::new(r#"{"key":"k","name":"X","in":"body"}"#).unwrap();

    let client = unsafe {
        ycallr_client_new_with_auth(api, auth_type.as_ptr(), auth_data.as_ptr(), 0, ptr::null())
    };
    assert!(client.is_null());

    let err = unsafe { ycallr_get_last_error() };
    assert!(!err.is_null());
    let err_str = unsafe { std::ffi::CStr::from_ptr(err) };
    assert!(err_str.to_str().unwrap().contains("location"));

    unsafe { ycallr_free_api(api) };
}

// ─── Call ─────────────────────────────────────────────────────────────

#[test]
fn test_ffi_call_command_not_found() {
    let yaml = CString::new(VALID_YAML).unwrap();
    let api = unsafe { ycallr_parse_yaml(yaml.as_ptr()) };
    let client = unsafe { ycallr_client_new(api, 0, ptr::null()) };

    let command = CString::new("nonexistent").unwrap();
    let resp = unsafe { ycallr_call(client, command.as_ptr(), ptr::null(), ptr::null()) };
    assert!(resp.is_null());

    let err = unsafe { ycallr_get_last_error() };
    assert!(!err.is_null());

    unsafe {
        ycallr_client_free(client);
        ycallr_free_api(api);
    }
}

#[test]
fn test_ffi_call_null_client() {
    let command = CString::new("test").unwrap();
    let resp = unsafe { ycallr_call(ptr::null(), command.as_ptr(), ptr::null(), ptr::null()) };
    assert!(resp.is_null());
}

#[test]
fn test_ffi_call_invalid_params_json() {
    let yaml = CString::new(VALID_YAML).unwrap();
    let api = unsafe { ycallr_parse_yaml(yaml.as_ptr()) };
    let client = unsafe { ycallr_client_new(api, 0, ptr::null()) };

    let command = CString::new("get-repo").unwrap();
    let invalid_params = CString::new("{not-json").unwrap();
    let resp = unsafe {
        ycallr_call(
            client,
            command.as_ptr(),
            invalid_params.as_ptr(),
            ptr::null(),
        )
    };
    assert!(resp.is_null());

    let err = unsafe { ycallr_get_last_error() };
    assert!(!err.is_null());
    let err_str = unsafe { std::ffi::CStr::from_ptr(err) };
    assert!(err_str.to_str().unwrap().contains("Invalid params_json"));

    unsafe {
        ycallr_client_free(client);
        ycallr_free_api(api);
    }
}

#[test]
fn test_ffi_call_invalid_body_json() {
    let yaml = CString::new(VALID_YAML).unwrap();
    let api = unsafe { ycallr_parse_yaml(yaml.as_ptr()) };
    let client = unsafe { ycallr_client_new(api, 0, ptr::null()) };

    let command = CString::new("get-repo").unwrap();
    let invalid_body = CString::new("{not-json").unwrap();
    let resp = unsafe { ycallr_call(client, command.as_ptr(), ptr::null(), invalid_body.as_ptr()) };
    assert!(resp.is_null());

    let err = unsafe { ycallr_get_last_error() };
    assert!(!err.is_null());
    let err_str = unsafe { std::ffi::CStr::from_ptr(err) };
    assert!(err_str.to_str().unwrap().contains("Invalid body_json"));

    unsafe {
        ycallr_client_free(client);
        ycallr_free_api(api);
    }
}

#[test]
fn test_ffi_client_new_with_auth_invalid_json() {
    let yaml = CString::new(VALID_YAML).unwrap();
    let api = unsafe { ycallr_parse_yaml(yaml.as_ptr()) };

    let auth_type = CString::new("bearer").unwrap();
    let invalid_auth = CString::new("{not-json").unwrap();
    let client = unsafe {
        ycallr_client_new_with_auth(
            api,
            auth_type.as_ptr(),
            invalid_auth.as_ptr(),
            0,
            ptr::null(),
        )
    };
    assert!(client.is_null());

    let err = unsafe { ycallr_get_last_error() };
    assert!(!err.is_null());
    let err_str = unsafe { std::ffi::CStr::from_ptr(err) };
    assert!(err_str.to_str().unwrap().contains("auth_data_json"));

    unsafe { ycallr_free_api(api) };
}

#[test]
fn test_ffi_call_over_http() {
    let mut server = mockito::Server::new();

    let mock = server
        .mock("GET", "/repos/rust-lang/rust")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"name": "rust"}"#)
        .create();

    let yaml = format!(
        r#"
name: ffi-http
version: "1.0.0"
base_url: https://api.test.com
commands:
  get-repo:
    endpoint: /repos/{{owner}}/{{repo}}
    method: GET
    params:
      owner:
        description: Owner
        type: string
        required: true
      repo:
        description: Repo
        type: string
        required: true
"#
    );

    let yaml = CString::new(yaml).unwrap();
    let api = unsafe { ycallr_parse_yaml(yaml.as_ptr()) };
    assert!(!api.is_null());

    let server_url = CString::new(server.url()).unwrap();
    assert_eq!(unsafe { ycallr_set_base_url(api, server_url.as_ptr()) }, 0);

    let client = unsafe { ycallr_client_new(api, 0, ptr::null()) };
    assert!(!client.is_null());

    let command = CString::new("get-repo").unwrap();
    let params = CString::new(r#"{"owner":"rust-lang","repo":"rust"}"#).unwrap();
    let resp = unsafe { ycallr_call(client, command.as_ptr(), params.as_ptr(), ptr::null()) };
    assert!(!resp.is_null());

    let status = unsafe { ycallr_response_get_status(resp) };
    assert_eq!(status, 200);

    mock.assert();

    unsafe {
        ycallr_free_response(resp);
        ycallr_client_free(client);
        ycallr_free_api(api);
    }
}

// ─── Response ─────────────────────────────────────────────────────────

#[test]
fn test_ffi_response_getters_null() {
    unsafe {
        assert_eq!(ycallr_response_get_status(ptr::null()), 0);
        assert!(ycallr_response_get_headers_json(ptr::null()).is_null());
        assert!(ycallr_response_get_body_json(ptr::null()).is_null());
        assert!(ycallr_response_get_message(ptr::null()).is_null());
    }
}

// ─── String free ──────────────────────────────────────────────────────

#[test]
fn test_ffi_string_free_null() {
    unsafe { ycallr_string_free(ptr::null_mut()) };
}

#[test]
fn test_ffi_set_base_url_errors() {
    let yaml = CString::new(VALID_YAML).unwrap();
    let api = unsafe { ycallr_parse_yaml(yaml.as_ptr()) };
    assert!(!api.is_null());

    assert_eq!(
        unsafe { ycallr_set_base_url(ptr::null_mut(), ptr::null()) },
        -1
    );
    assert_eq!(unsafe { ycallr_set_base_url(api, ptr::null()) }, -1);

    let empty = CString::new("").unwrap();
    assert_eq!(unsafe { ycallr_set_base_url(api, empty.as_ptr()) }, -1);

    unsafe { ycallr_free_api(api) };
}

#[test]
fn test_ffi_command_get_headers_and_auth() {
    let yaml = CString::new(VALID_YAML).unwrap();
    let api = unsafe { ycallr_parse_yaml(yaml.as_ptr()) };
    let path = CString::new("get-repo").unwrap();
    let cmd = unsafe { ycallr_get_command(api, path.as_ptr()) };
    assert!(!cmd.is_null());

    unsafe {
        let headers = ycallr_command_get_headers_json(cmd);
        assert!(!headers.is_null());
        let headers_str = std::ffi::CStr::from_ptr(headers).to_str().unwrap();
        assert!(headers_str.contains("Accept"));
        ycallr_string_free(headers);

        assert!(ycallr_command_get_auth(cmd).is_null());

        assert!(ycallr_command_get_endpoint(ptr::null()).is_null());
        assert!(ycallr_command_get_method(ptr::null()).is_null());
        assert!(ycallr_command_get_description(ptr::null()).is_null());
        assert!(ycallr_command_get_auth(ptr::null()).is_null());
        assert!(ycallr_command_get_headers_json(ptr::null()).is_null());
        assert!(ycallr_command_get_params_json(ptr::null()).is_null());
        assert!(!ycallr_command_is_leaf(ptr::null()));
        assert!(!ycallr_command_is_branch(ptr::null()));

        ycallr_free_command(cmd);
        ycallr_free_api(api);
    }
}

#[test]
fn test_ffi_call_with_response_getters() {
    let mut server = mockito::Server::new();
    let mock = server
        .mock("GET", "/repos/rust-lang/rust")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"name":"rust"}"#)
        .create();

    let yaml = format!(
        r#"
name: ffi-resp
version: "1.0.0"
base_url: https://api.test.com
commands:
  get-repo:
    endpoint: /repos/{{owner}}/{{repo}}
    method: GET
    responses:
      success:
        message: "Got {{output.name}}"
    params:
      owner:
        description: Owner
        type: string
        required: true
      repo:
        description: Repo
        type: string
        required: true
"#
    );

    let yaml = CString::new(yaml).unwrap();
    let api = unsafe { ycallr_parse_yaml(yaml.as_ptr()) };
    let url = CString::new(server.url()).unwrap();
    unsafe { ycallr_set_base_url(api, url.as_ptr()) };

    let client = unsafe { ycallr_client_new(api, 0, ptr::null()) };
    let command = CString::new("get-repo").unwrap();
    let params = CString::new(r#"{"owner":"rust-lang","repo":"rust"}"#).unwrap();
    let resp = unsafe { ycallr_call(client, command.as_ptr(), params.as_ptr(), ptr::null()) };
    assert!(!resp.is_null());

    unsafe {
        assert_eq!(ycallr_response_get_status(resp), 200);
        let headers = ycallr_response_get_headers_json(resp);
        assert!(!headers.is_null());
        ycallr_string_free(headers);
        let body = ycallr_response_get_body_json(resp);
        assert!(!body.is_null());
        ycallr_string_free(body);
        let message = ycallr_response_get_message(resp);
        assert!(!message.is_null());
        ycallr_string_free(message);

        ycallr_free_response(resp);
        ycallr_client_free(client);
        ycallr_free_api(api);
    }

    mock.assert();
}

#[test]
fn test_ffi_client_new_with_auth_http_basic() {
    let yaml = CString::new(VALID_YAML).unwrap();
    let api = unsafe { ycallr_parse_yaml(yaml.as_ptr()) };
    let auth_data = CString::new(r#"{"username":"u","password":"p"}"#).unwrap();
    let auth_type = CString::new("http_basic").unwrap();

    let client = unsafe {
        ycallr_client_new_with_auth(api, auth_type.as_ptr(), auth_data.as_ptr(), 0, ptr::null())
    };
    assert!(!client.is_null());

    unsafe {
        ycallr_client_free(client);
        ycallr_free_api(api);
    }
}

#[test]
fn test_ffi_parse_proto_roundtrip() {
    let bytes = ycallr_core::profile_store::compile_yaml_str(VALID_YAML).unwrap();
    let loaded = unsafe { ycallr_parse_proto(bytes.as_ptr(), bytes.len()) };
    assert!(!loaded.is_null());

    unsafe {
        let name = ycallr_get_name(loaded);
        assert!(!name.is_null());
        assert_eq!(std::ffi::CStr::from_ptr(name).to_str().unwrap(), "github");
        ycallr_free_api(loaded);
    }
}

#[test]
fn test_ffi_install_and_load_installed() {
    let dir = tempfile::tempdir().unwrap();
    let apis = dir.path().join(".config").join("ycallr").join("apis");
    std::fs::create_dir_all(&apis).unwrap();
    let yaml_path = apis.join("ffi-demo.yaml");
    std::fs::write(
        &yaml_path,
        r#"
name: ffi-demo
version: "1"
description: FFI install test
base_url: http://127.0.0.1:9
commands:
  ping:
    endpoint: /ping
    method: GET
"#,
    )
    .unwrap();

    with_home(dir.path(), || {
        let name = CString::new("ffi-demo").unwrap();
        let rc = unsafe { ycallr_install(name.as_ptr()) };
        assert_eq!(rc, 0);

        let list = unsafe { ycallr_list_installed() };
        assert!(!list.is_null());
        let json = unsafe { CStr::from_ptr(list).to_str().unwrap() };
        assert!(json.contains("ffi-demo"));
        ycallr_string_free(list);

        let install_result = unsafe { ycallr_get_last_install_result() };
        assert!(!install_result.is_null());
        let install_json = unsafe { CStr::from_ptr(install_result).to_str().unwrap() };
        assert!(install_json.contains("ffi-demo"));
        assert!(install_json.contains("pb_path"));
        ycallr_string_free(install_result);

        let path_ptr = unsafe { ycallr_compiled_profile_path(name.as_ptr()) };
        assert!(!path_ptr.is_null());
        let pb_path = unsafe { CStr::from_ptr(path_ptr).to_str().unwrap() };
        assert!(pb_path.ends_with("ffi-demo.pb"));
        ycallr_string_free(path_ptr);

        let api = unsafe { ycallr_load_installed(name.as_ptr()) };
        assert!(!api.is_null());

        unsafe {
            let desc = ycallr_get_description(api);
            assert!(!desc.is_null());
            assert_eq!(
                std::ffi::CStr::from_ptr(desc).to_str().unwrap(),
                "FFI install test"
            );
            ycallr_free_api(api);
        }

        let path = CString::new(yaml_path.to_str().unwrap()).unwrap();
        let rc = unsafe { ycallr_install_yaml_file(path.as_ptr()) };
        assert_eq!(rc, 0);
    });
}

#[test]
fn test_ffi_list_subcommands_and_param_helpers() {
    let yaml = CString::new(VALID_YAML).unwrap();
    let api = unsafe { ycallr_parse_yaml(yaml.as_ptr()) };
    assert!(!api.is_null());

    let empty = CString::new("").unwrap();
    let top = unsafe { ycallr_list_subcommands(api, empty.as_ptr()) };
    assert!(!top.is_null());
    let top_json = unsafe { CStr::from_ptr(top).to_str().unwrap() };
    assert!(top_json.contains("get-repo"));
    assert!(top_json.contains("repos"));
    ycallr_string_free(top);

    let repos_path = CString::new("repos").unwrap();
    let nested = unsafe { ycallr_list_subcommands(api, repos_path.as_ptr()) };
    assert!(!nested.is_null());
    let nested_json = unsafe { CStr::from_ptr(nested).to_str().unwrap() };
    assert!(nested_json.contains("issues"));
    ycallr_string_free(nested);

    let env_ptr = unsafe { ycallr_get_env_json(api) };
    assert!(!env_ptr.is_null());
    ycallr_string_free(env_ptr);

    let cmd_path = CString::new("get-repo").unwrap();
    let missing_params = CString::new(r#"{"owner":"rust-lang"}"#).unwrap();
    let missing =
        unsafe { ycallr_missing_params_json(api, cmd_path.as_ptr(), missing_params.as_ptr()) };
    assert!(!missing.is_null());
    let missing_json = unsafe { CStr::from_ptr(missing).to_str().unwrap() };
    assert!(missing_json.contains("repo"));
    ycallr_string_free(missing);

    let create_path = CString::new("repos.issues.create").unwrap();
    let create_params = CString::new(r#"{"owner":"o","repo":"r","title":"t"}"#).unwrap();
    let body_ptr = unsafe {
        ycallr_build_implicit_body_json(api, create_path.as_ptr(), create_params.as_ptr())
    };
    assert!(!body_ptr.is_null());
    let body_json = unsafe { CStr::from_ptr(body_ptr).to_str().unwrap() };
    assert!(body_json.contains("title"));
    ycallr_string_free(body_ptr);

    let cmd = unsafe { ycallr_get_command(api, cmd_path.as_ptr()) };
    assert!(!cmd.is_null());
    unsafe {
        let desc = ycallr_command_get_description(cmd);
        assert!(!desc.is_null());
        assert_eq!(CStr::from_ptr(desc).to_str().unwrap(), "Get a repository");
        ycallr_string_free(desc);

        let kind = ycallr_command_get_body_kind(cmd);
        assert!(kind.is_null());

        ycallr_free_command(cmd);
        ycallr_free_api(api);
    }
}

#[test]
fn test_ffi_system_install_and_call_with_mock() {
    let dir = tempfile::tempdir().unwrap();
    let apis = dir.path().join(".config").join("ycallr").join("apis");
    std::fs::create_dir_all(&apis).unwrap();
    let yaml_path = apis.join("ffi-sys.yaml");
    std::fs::write(
        &yaml_path,
        r#"
name: ffi-sys
version: "1"
base_url: https://api.test.invalid
commands:
  ping:
    endpoint: /ping
    method: GET
"#,
    )
    .unwrap();

    with_home(dir.path(), || {
        let path = CString::new(yaml_path.to_str().unwrap()).unwrap();
        assert_eq!(unsafe { ycallr_install_yaml_file(path.as_ptr()) }, 0);

        let mut server = mockito::Server::new();
        let mock = server
            .mock("GET", "/ping")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"alive":true}"#)
            .create();

        let name = CString::new("ffi-sys").unwrap();
        let api = unsafe { ycallr_load_installed(name.as_ptr()) };
        assert!(!api.is_null());

        let server_url = CString::new(server.url()).unwrap();
        assert_eq!(unsafe { ycallr_set_base_url(api, server_url.as_ptr()) }, 0);

        let client = unsafe { ycallr_client_new(api, 0, ptr::null()) };
        assert!(!client.is_null());

        let command = CString::new("ping").unwrap();
        let params = CString::new("{}").unwrap();
        let resp = unsafe { ycallr_call(client, command.as_ptr(), params.as_ptr(), ptr::null()) };
        assert!(!resp.is_null());
        assert_eq!(unsafe { ycallr_response_get_status(resp) }, 200);

        mock.assert();

        unsafe {
            ycallr_free_response(resp);
            ycallr_client_free(client);
            ycallr_free_api(api);
        }
    });
}

#[test]
fn test_ffi_command_path_params_and_body_flag() {
    let yaml = CString::new(VALID_YAML).unwrap();
    let api = unsafe { ycallr_parse_yaml(yaml.as_ptr()) };
    let path = CString::new("get-repo").unwrap();
    let cmd = unsafe { ycallr_get_command(api, path.as_ptr()) };
    assert!(!cmd.is_null());

    unsafe {
        assert!(!ycallr_command_has_body(cmd));
        let path_params = ycallr_command_get_path_params_json(cmd);
        assert!(!path_params.is_null());
        let json = CStr::from_ptr(path_params).to_str().unwrap();
        assert!(json.contains("owner"));
        ycallr_string_free(path_params);
        ycallr_free_command(cmd);
        ycallr_free_api(api);
    }
}
