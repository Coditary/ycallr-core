//! End-to-end system tests: YAML install → load `.pb` → HTTP call with mock server.

#![cfg(all(not(target_arch = "wasm32"), feature = "client"))]

use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;

use ycallr_core::models::ApiDefinition;
use ycallr_core::profile_store::{
    install_profile_from_path, list_installed_profile_names, load_installed_profile,
};
use ycallr_core::YcallrClient;

static HOME_LOCK: Mutex<()> = Mutex::new(());

fn with_home<F: FnOnce()>(home: &Path, f: F) {
    let _guard = HOME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    std::env::set_var("HOME", home);
    f();
    std::env::remove_var("HOME");
}

const SYSTEM_API_YAML: &str = r#"
name: sys-demo
version: "1"
description: System test API
base_url: https://api.placeholder.invalid
commands:
  get-user:
    endpoint: /users/{id}
    method: GET
    params:
      id:
        description: User id
        type: string
        required: true
    responses:
      success:
        message: "User {output.name}"
  repos:
    description: Repository group
    commands:
      issues:
        description: Issues
        commands:
          list:
            endpoint: /repos/{owner}/{repo}/issues
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
errors:
  default: "API failed with {status}"
  404: "User not found"
"#;

#[test]
fn test_system_install_load_call_with_mock_http() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("sys-demo.yaml");
    std::fs::write(&source, SYSTEM_API_YAML).unwrap();

    with_home(dir.path(), || {
        let (name, pb_path) = install_profile_from_path(&source).unwrap();
        assert_eq!(name, "sys-demo");
        assert!(pb_path.is_file());

        let installed = list_installed_profile_names().unwrap();
        assert_eq!(installed, ["sys-demo"]);

        let mut server = mockito::Server::new();
        let mock = server
            .mock("GET", "/users/42")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"name":"alice"}"#)
            .create();

        let api = load_installed_profile("sys-demo").unwrap();
        let api = ApiDefinition {
            base_url: server.url(),
            ..api
        };

        let client = YcallrClient::new(api).unwrap();
        let params = HashMap::from([("id".to_string(), "42".to_string())]);
        let response = client.call("get-user", &params, None).unwrap();

        mock.assert();
        assert_eq!(response.status, 200);
        assert_eq!(response.message.as_deref(), Some("User alice"));
    });
}

#[test]
fn test_system_api_errors_template_on_http_failure() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("sys-demo.yaml");
    std::fs::write(&source, SYSTEM_API_YAML).unwrap();

    with_home(dir.path(), || {
        install_profile_from_path(&source).unwrap();

        let mut server = mockito::Server::new();
        let mock = server
            .mock("GET", "/users/missing")
            .with_status(404)
            .with_header("content-type", "application/json")
            .with_body(r#"{"message":"gone"}"#)
            .create();

        let api = load_installed_profile("sys-demo").unwrap();
        let api = ApiDefinition {
            base_url: server.url(),
            ..api
        };

        let client = YcallrClient::new(api).unwrap();
        let params = HashMap::from([("id".to_string(), "missing".to_string())]);
        let response = client.call("get-user", &params, None).unwrap();

        mock.assert();
        assert_eq!(response.status, 404);
        assert_eq!(response.message.as_deref(), Some("User not found"));
    });
}

#[test]
fn test_system_nested_command_from_installed_profile() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("sys-demo.yaml");
    std::fs::write(&source, SYSTEM_API_YAML).unwrap();

    with_home(dir.path(), || {
        install_profile_from_path(&source).unwrap();

        let mut server = mockito::Server::new();
        let mock = server
            .mock("GET", "/repos/rust-lang/rust/issues")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"[]"#)
            .create();

        let api = load_installed_profile("sys-demo").unwrap();
        let api = ApiDefinition {
            base_url: server.url(),
            ..api
        };

        let client = YcallrClient::new(api).unwrap();
        let params = HashMap::from([
            ("owner".to_string(), "rust-lang".to_string()),
            ("repo".to_string(), "rust".to_string()),
        ]);
        let response = client.call("repos.issues.list", &params, None).unwrap();

        mock.assert();
        assert_eq!(response.status, 200);
    });
}

#[test]
fn test_system_proto_roundtrip_then_call_with_mock() {
    let bytes = ycallr_core::profile_store::compile_yaml_str(
        r#"
name: roundtrip
version: "1"
base_url: https://api.placeholder.invalid
commands:
  ping:
    endpoint: /ping
    method: GET
"#,
    )
    .unwrap();

    let api = ycallr_core::profile_store::load_from_proto_bytes(&bytes).unwrap();

    let mut server = mockito::Server::new();
    let mock = server
        .mock("GET", "/ping")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"ok":true}"#)
        .create();

    let api = ApiDefinition {
        base_url: server.url(),
        ..api
    };

    let client = YcallrClient::new(api).unwrap();
    let response = client.call("ping", &HashMap::new(), None).unwrap();

    mock.assert();
    assert_eq!(response.status, 200);
    assert_eq!(response.body["ok"], true);
}
