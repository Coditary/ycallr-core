#![cfg(not(target_arch = "wasm32"))]

use std::collections::HashMap;
use ycallr_core::{
    ApiDefinition, AuthConfig, Command, HttpMethod, ParamType, Parameter, YcallrClient,
};

fn create_test_api(base_url: &str) -> ApiDefinition {
    let mut commands = HashMap::new();

    let mut get_params = HashMap::new();
    get_params.insert(
        "owner".to_string(),
        Parameter {
            description: "Repository owner".to_string(),
            param_type: ParamType::String,
            required: true,
        },
    );
    get_params.insert(
        "repo".to_string(),
        Parameter {
            description: "Repository name".to_string(),
            param_type: ParamType::String,
            required: true,
        },
    );

    let mut get_headers = HashMap::new();
    get_headers.insert("Accept".to_string(), "application/json".to_string());

    commands.insert(
        "get-repo".to_string(),
        Command {
            endpoint: "/repos/{owner}/{repo}".to_string(),
            method: HttpMethod::GET,
            headers: get_headers,
            params: get_params,
        },
    );

    let mut post_params = HashMap::new();
    post_params.insert(
        "title".to_string(),
        Parameter {
            description: "Issue title".to_string(),
            param_type: ParamType::String,
            required: true,
        },
    );

    let mut post_headers = HashMap::new();
    post_headers.insert("Content-Type".to_string(), "application/json".to_string());

    commands.insert(
        "create-issue".to_string(),
        Command {
            endpoint: "/repos/{owner}/{repo}/issues".to_string(),
            method: HttpMethod::POST,
            headers: post_headers,
            params: post_params,
        },
    );

    ApiDefinition {
        name: "github".to_string(),
        version: "1.0.0".to_string(),
        description: "GitHub API".to_string(),
        base_url: base_url.to_string(),
        commands,
    }
}

#[test]
fn test_get_request() {
    let mut server = mockito::Server::new();

    let mock = server
        .mock("GET", "/repos/rust-lang/rust")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"name": "rust", "stars": 90000}"#)
        .create();

    let api = create_test_api(&server.url());
    let client = YcallrClient::new(api).unwrap();

    let mut params = HashMap::new();
    params.insert("owner".to_string(), "rust-lang".to_string());
    params.insert("repo".to_string(), "rust".to_string());

    let response = client.call("get-repo", &params, None).unwrap();

    mock.assert();
    assert_eq!(response.status, 200);
    assert_eq!(response.body["name"], "rust");
    assert_eq!(response.body["stars"], 90000);
}

#[test]
fn test_post_request_with_body() {
    let mut server = mockito::Server::new();

    let mock = server
        .mock("POST", "/repos/rust-lang/rust/issues")
        .with_status(201)
        .with_header("content-type", "application/json")
        .with_body(r#"{"id": 1, "title": "Test Issue"}"#)
        .create();

    let api = create_test_api(&server.url());
    let client = YcallrClient::new(api).unwrap();

    let mut params = HashMap::new();
    params.insert("owner".to_string(), "rust-lang".to_string());
    params.insert("repo".to_string(), "rust".to_string());

    let body = serde_json::json!({"title": "Test Issue"});
    let response = client.call("create-issue", &params, Some(&body)).unwrap();

    mock.assert();
    assert_eq!(response.status, 201);
    assert_eq!(response.body["title"], "Test Issue");
}

#[test]
fn test_auth_bearer() {
    let mut server = mockito::Server::new();

    let mock = server
        .mock("GET", "/repos/rust-lang/rust")
        .match_header("Authorization", "Bearer test-token-123")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"name": "rust"}"#)
        .create();

    let api = create_test_api(&server.url());
    let client =
        YcallrClient::with_auth(api, AuthConfig::Bearer("test-token-123".to_string())).unwrap();

    let mut params = HashMap::new();
    params.insert("owner".to_string(), "rust-lang".to_string());
    params.insert("repo".to_string(), "rust".to_string());

    let response = client.call("get-repo", &params, None).unwrap();

    mock.assert();
    assert_eq!(response.status, 200);
}

#[test]
fn test_auth_api_key() {
    let mut server = mockito::Server::new();

    let mock = server
        .mock("GET", "/repos/rust-lang/rust")
        .match_header("X-API-Key", "my-secret-key")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"name": "rust"}"#)
        .create();

    let api = create_test_api(&server.url());
    let client = YcallrClient::with_auth(
        api,
        AuthConfig::ApiKey {
            key: "my-secret-key".to_string(),
            header: "X-API-Key".to_string(),
        },
    )
    .unwrap();

    let mut params = HashMap::new();
    params.insert("owner".to_string(), "rust-lang".to_string());
    params.insert("repo".to_string(), "rust".to_string());

    let response = client.call("get-repo", &params, None).unwrap();

    mock.assert();
    assert_eq!(response.status, 200);
}

#[test]
fn test_error_response() {
    let mut server = mockito::Server::new();

    let mock = server
        .mock("GET", "/repos/rust-lang/missing")
        .with_status(404)
        .with_header("content-type", "application/json")
        .with_body(r#"{"message": "Not Found"}"#)
        .create();

    let api = create_test_api(&server.url());
    let client = YcallrClient::new(api).unwrap();

    let mut params = HashMap::new();
    params.insert("owner".to_string(), "rust-lang".to_string());
    params.insert("repo".to_string(), "missing".to_string());

    let response = client.call("get-repo", &params, None).unwrap();

    mock.assert();
    assert_eq!(response.status, 404);
    assert_eq!(response.body["message"], "Not Found");
}

#[test]
fn test_client_access_api() {
    let api = create_test_api("https://api.github.com");
    let client = YcallrClient::new(api).unwrap();
    assert_eq!(client.api().name, "github");
}
