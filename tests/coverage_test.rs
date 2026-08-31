#![cfg(all(not(target_arch = "wasm32"), feature = "client"))]

use std::collections::HashMap;
use tempfile::tempdir;
use ycallr_core::{
    client::EnvMode, ApiDefinition, BodyConfig, Command, HttpMethod, MultipartField, ParamType,
    Parameter, YcallrClient,
};

#[test]
fn test_builder_build_context() {
    let mut commands = HashMap::new();
    commands.insert(
        "ping".to_string(),
        Command {
            description: None,
            endpoint: Some("/ping".to_string()),
            method: Some(HttpMethod::GET),
            auth: None,
            headers: HashMap::new(),
            params: HashMap::new(),
            body: None,
            responses: None,
            commands: None,
        },
    );

    let api = ApiDefinition {
        name: "ctx".to_string(),
        version: "1".to_string(),
        description: "".to_string(),
        base_url: "http://127.0.0.1:9".to_string(),
        env: vec![],
        auth: HashMap::new(),
        commands,
    };

    let ctx = YcallrClient::builder(api)
        .env_mode(EnvMode::Manual)
        .build_context()
        .unwrap();
    assert_eq!(ctx.api.name, "ctx");
}

#[test]
fn test_multipart_body_over_http() {
    let mut server = mockito::Server::new();
    let mock = server
        .mock("POST", "/upload")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"ok":true}"#)
        .create();

    let dir = tempdir().unwrap();
    let file_path = dir.path().join("payload.bin");
    std::fs::write(&file_path, b"file-bytes").unwrap();
    let canonical = file_path
        .canonicalize()
        .unwrap()
        .to_string_lossy()
        .to_string();

    let mut commands = HashMap::new();
    commands.insert(
        "upload".to_string(),
        Command {
            description: None,
            endpoint: Some("/upload".to_string()),
            method: Some(HttpMethod::POST),
            auth: None,
            headers: HashMap::new(),
            params: HashMap::new(),
            body: Some(BodyConfig {
                json: None,
                form: None,
                raw: None,
                multipart: Some(vec![
                    MultipartField {
                        name: "note".to_string(),
                        text: Some("hello".to_string()),
                        file: None,
                    },
                    MultipartField {
                        name: "file".to_string(),
                        text: None,
                        file: Some(canonical),
                    },
                ]),
            }),
            responses: None,
            commands: None,
        },
    );

    let api = ApiDefinition {
        name: "upload-api".to_string(),
        version: "1".to_string(),
        description: "".to_string(),
        base_url: server.url(),
        env: vec![],
        auth: HashMap::new(),
        commands,
    };

    let client = YcallrClient::new(api).unwrap();
    let response = client.call("upload", &HashMap::new(), None).unwrap();
    mock.assert();
    assert_eq!(response.status, 200);
    assert_eq!(response.body["ok"], true);
}

#[test]
fn test_validate_params_on_client() {
    let mut commands = HashMap::new();
    let mut params = HashMap::new();
    params.insert(
        "id".to_string(),
        Parameter {
            description: "id".to_string(),
            param_type: ParamType::Number,
            required: true,
        },
    );
    commands.insert(
        "get".to_string(),
        Command {
            description: None,
            endpoint: Some("/items/{id}".to_string()),
            method: Some(HttpMethod::GET),
            auth: None,
            headers: HashMap::new(),
            params,
            body: None,
            responses: None,
            commands: None,
        },
    );

    let api = ApiDefinition {
        name: "v".to_string(),
        version: "1".to_string(),
        description: "".to_string(),
        base_url: "http://127.0.0.1:9".to_string(),
        env: vec![],
        auth: HashMap::new(),
        commands,
    };

    let client = YcallrClient::new(api).unwrap();
    let bad = HashMap::from([("id".to_string(), "not-a-number".to_string())]);
    let err = client.validate_params("get", &bad, None).unwrap_err();
    assert!(err.to_string().contains("number"));
}
