#![cfg(all(not(target_arch = "wasm32"), feature = "client"))]

use std::collections::HashMap;
use ycallr_core::{
    client::EnvMode, ApiDefinition, AuthConfig, Command, HttpMethod, ParamType, Parameter,
    YcallrClient,
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
            description: Some("Get a repository".to_string()),
            endpoint: Some("/repos/{owner}/{repo}".to_string()),
            method: Some(HttpMethod::GET),
            auth: None,
            headers: get_headers,
            params: get_params,
            body: None,
            commands: None,
            responses: None,
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
            description: Some("Create an issue".to_string()),
            endpoint: Some("/repos/{owner}/{repo}/issues".to_string()),
            method: Some(HttpMethod::POST),
            auth: None,
            headers: post_headers,
            params: post_params,
            body: None,
            commands: None,
            responses: None,
        },
    );

    ApiDefinition {
        name: "github".to_string(),
        version: "1.0.0".to_string(),
        description: "GitHub API".to_string(),
        base_url: base_url.to_string(),
        env: vec![],
        auth: HashMap::new(),
        commands,
    }
}

fn create_nested_test_api(base_url: &str) -> ApiDefinition {
    let mut commands = HashMap::new();

    let mut repos_commands = HashMap::new();
    let mut issues_commands = HashMap::new();

    issues_commands.insert(
        "create".to_string(),
        Command {
            description: Some("Create an issue".to_string()),
            endpoint: Some("/repos/{owner}/{repo}/issues".to_string()),
            method: Some(HttpMethod::POST),
            auth: None,
            headers: HashMap::new(),
            params: HashMap::new(),
            body: None,
            commands: None,
            responses: None,
        },
    );

    repos_commands.insert(
        "issues".to_string(),
        Command {
            description: Some("Issues operations".to_string()),
            endpoint: Some("/repos/{owner}/{repo}/issues".to_string()),
            method: Some(HttpMethod::GET),
            auth: None,
            headers: HashMap::new(),
            params: HashMap::new(),
            body: None,
            commands: Some(issues_commands),
            responses: None,
        },
    );

    commands.insert(
        "repos".to_string(),
        Command {
            description: Some("Repository operations".to_string()),
            endpoint: Some("/repos".to_string()),
            method: Some(HttpMethod::GET),
            auth: None,
            headers: HashMap::new(),
            params: HashMap::new(),
            body: None,
            commands: Some(repos_commands),
            responses: None,
        },
    );

    ApiDefinition {
        name: "github-nested".to_string(),
        version: "1.0.0".to_string(),
        description: "GitHub API with nested commands".to_string(),
        base_url: base_url.to_string(),
        env: vec![],
        auth: HashMap::new(),
        commands,
    }
}

fn create_env_test_api(base_url: &str) -> ApiDefinition {
    let mut commands = HashMap::new();

    let mut headers = HashMap::new();
    headers.insert(
        "Authorization".to_string(),
        "Bearer ${GITHUB_TOKEN}".to_string(),
    );

    commands.insert(
        "get-repo".to_string(),
        Command {
            description: Some("Get a repository".to_string()),
            endpoint: Some("/repos/{owner}/{repo}".to_string()),
            method: Some(HttpMethod::GET),
            auth: None,
            headers,
            params: HashMap::new(),
            body: None,
            commands: None,
            responses: None,
        },
    );

    ApiDefinition {
        name: "github-env".to_string(),
        version: "1.0.0".to_string(),
        description: "GitHub API with env vars".to_string(),
        base_url: base_url.to_string(),
        env: vec![ycallr_core::models::EnvVar {
            name: "GITHUB_TOKEN".to_string(),
            required: true,
        }],
        auth: HashMap::new(),
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
        YcallrClient::with_auth(api, AuthConfig::bearer("test-token-123".to_string())).unwrap();

    let mut params = HashMap::new();
    params.insert("owner".to_string(), "rust-lang".to_string());
    params.insert("repo".to_string(), "rust".to_string());

    let response = client.call("get-repo", &params, None).unwrap();

    mock.assert();
    assert_eq!(response.status, 200);
}

#[test]
fn test_named_auth_bearer_from_yaml() {
    let mut server = mockito::Server::new();

    let mock = server
        .mock("GET", "/repos/rust-lang/rust")
        .match_header("Authorization", "Bearer yaml-token-123")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"name": "rust"}"#)
        .create();

    let mut commands = HashMap::new();
    let mut params = HashMap::new();
    params.insert("owner".to_string(), ycallr_core::Parameter {
        description: "Owner".to_string(),
        param_type: ycallr_core::ParamType::String,
        required: true,
    });
    params.insert("repo".to_string(), ycallr_core::Parameter {
        description: "Repo".to_string(),
        param_type: ycallr_core::ParamType::String,
        required: true,
    });

    let mut auth_map = HashMap::new();
    auth_map.insert("primary".to_string(), AuthConfig::bearer("yaml-token-123".to_string()));

    commands.insert("get-repo".to_string(), ycallr_core::Command {
        description: Some("Get repo".to_string()),
        endpoint: Some("/repos/{owner}/{repo}".to_string()),
        method: Some(ycallr_core::HttpMethod::GET),
        auth: Some("primary".to_string()),
        headers: HashMap::new(),
        params,
        body: None,
        responses: None,
        commands: None,
    });

    let api = ApiDefinition {
        name: "github".to_string(),
        version: "1.0.0".to_string(),
        description: "GitHub API".to_string(),
        base_url: server.url(),
        env: vec![],
        auth: auth_map,
        commands,
    };

    let client = YcallrClient::new(api).unwrap();

    let mut params = HashMap::new();
    params.insert("owner".to_string(), "rust-lang".to_string());
    params.insert("repo".to_string(), "rust".to_string());

    let response = client.call("get-repo", &params, None).unwrap();

    mock.assert();
    assert_eq!(response.status, 200);
}

#[test]
fn test_named_auth_api_key_from_yaml() {
    let mut server = mockito::Server::new();

    let mock = server
        .mock("GET", "/repos/rust-lang/rust")
        .match_header("X-API-Key", "my-api-key")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"name": "rust"}"#)
        .create();

    let mut commands = HashMap::new();
    let mut params = HashMap::new();
    params.insert("owner".to_string(), ycallr_core::Parameter {
        description: "Owner".to_string(),
        param_type: ycallr_core::ParamType::String,
        required: true,
    });
    params.insert("repo".to_string(), ycallr_core::Parameter {
        description: "Repo".to_string(),
        param_type: ycallr_core::ParamType::String,
        required: true,
    });

    let mut auth_map = HashMap::new();
    auth_map.insert("secondary".to_string(), AuthConfig::api_key("my-api-key".to_string(), "X-API-Key".to_string()));

    commands.insert("get-repo".to_string(), ycallr_core::Command {
        description: Some("Get repo".to_string()),
        endpoint: Some("/repos/{owner}/{repo}".to_string()),
        method: Some(ycallr_core::HttpMethod::GET),
        auth: Some("secondary".to_string()),
        headers: HashMap::new(),
        params,
        body: None,
        responses: None,
        commands: None,
    });

    let api = ApiDefinition {
        name: "github".to_string(),
        version: "1.0.0".to_string(),
        description: "GitHub API".to_string(),
        base_url: server.url(),
        env: vec![],
        auth: auth_map,
        commands,
    };

    let client = YcallrClient::new(api).unwrap();

    let mut params = HashMap::new();
    params.insert("owner".to_string(), "rust-lang".to_string());
    params.insert("repo".to_string(), "rust".to_string());

    let response = client.call("get-repo", &params, None).unwrap();

    mock.assert();
    assert_eq!(response.status, 200);
}

#[test]
fn test_command_without_auth_uses_none() {
    let mut server = mockito::Server::new();

    let mock = server
        .mock("GET", "/repos/rust-lang/rust")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"name": "rust"}"#)
        .create();

    let mut commands = HashMap::new();
    let mut params = HashMap::new();
    params.insert("owner".to_string(), ycallr_core::Parameter {
        description: "Owner".to_string(),
        param_type: ycallr_core::ParamType::String,
        required: true,
    });
    params.insert("repo".to_string(), ycallr_core::Parameter {
        description: "Repo".to_string(),
        param_type: ycallr_core::ParamType::String,
        required: true,
    });

    let mut auth_map = HashMap::new();
    auth_map.insert("primary".to_string(), AuthConfig::bearer("token".to_string()));

    commands.insert("get-repo".to_string(), ycallr_core::Command {
        description: Some("Get repo".to_string()),
        endpoint: Some("/repos/{owner}/{repo}".to_string()),
        method: Some(ycallr_core::HttpMethod::GET),
        auth: None,
        headers: HashMap::new(),
        params,
        body: None,
        responses: None,
        commands: None,
    });

    let api = ApiDefinition {
        name: "github".to_string(),
        version: "1.0.0".to_string(),
        description: "GitHub API".to_string(),
        base_url: server.url(),
        env: vec![],
        auth: auth_map,
        commands,
    };

    let client = YcallrClient::new(api).unwrap();

    let mut params = HashMap::new();
    params.insert("owner".to_string(), "rust-lang".to_string());
    params.insert("repo".to_string(), "rust".to_string());

    let response = client.call("get-repo", &params, None).unwrap();

    mock.assert();
    assert_eq!(response.status, 200);
}

#[test]
fn test_get_named_auth_config() {
    let mut auth_map = HashMap::new();
    auth_map.insert("primary".to_string(), AuthConfig::bearer("token1".to_string()));
    auth_map.insert("secondary".to_string(), AuthConfig::api_key("key1".to_string(), "X-Key".to_string()));

    let mut commands = HashMap::new();
    let api = ApiDefinition {
        name: "test".to_string(),
        version: "1.0.0".to_string(),
        description: "Test".to_string(),
        base_url: "https://api.test.com".to_string(),
        env: vec![],
        auth: auth_map,
        commands,
    };

    let client = YcallrClient::new(api).unwrap();

    assert!(client.get_auth_config("primary").is_some());
    assert!(client.get_auth_config("secondary").is_some());
    assert!(client.get_auth_config("nonexistent").is_none());
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
        AuthConfig::api_key("my-secret-key".to_string(), "X-API-Key".to_string()),
    )
    .unwrap();

    let mut params = HashMap::new();
    params.insert("owner".to_string(), "rust-lang".to_string());
    params.insert("repo".to_string(), "rust".to_string());

    let response = client.call("get-repo", &params, None).unwrap();

    mock.assert();
    assert_eq!(response.status, 200);
}
