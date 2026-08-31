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
        "owner".to_string(),
        Parameter {
            description: "Repository owner".to_string(),
            param_type: ParamType::String,
            required: true,
        },
    );
    post_params.insert(
        "repo".to_string(),
        Parameter {
            description: "Repository name".to_string(),
            param_type: ParamType::String,
            required: true,
        },
    );
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
        errors: None,
    }
}

fn create_nested_test_api(base_url: &str) -> ApiDefinition {
    let mut commands = HashMap::new();

    let mut repos_commands = HashMap::new();
    let mut issues_commands = HashMap::new();
    let path_params = owner_repo_params();

    issues_commands.insert(
        "create".to_string(),
        Command {
            description: Some("Create an issue".to_string()),
            endpoint: Some("/repos/{owner}/{repo}/issues".to_string()),
            method: Some(HttpMethod::POST),
            auth: None,
            headers: HashMap::new(),
            params: path_params.clone(),
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
            params: path_params,
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
        errors: None,
    }
}

fn owner_repo_params() -> HashMap<String, ycallr_core::Parameter> {
    HashMap::from([
        (
            "owner".to_string(),
            ycallr_core::Parameter {
                description: "Repository owner".to_string(),
                param_type: ycallr_core::ParamType::String,
                required: true,
            },
        ),
        (
            "repo".to_string(),
            ycallr_core::Parameter {
                description: "Repository name".to_string(),
                param_type: ycallr_core::ParamType::String,
                required: true,
            },
        ),
    ])
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
            params: owner_repo_params(),
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
        errors: None,
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
    params.insert(
        "owner".to_string(),
        ycallr_core::Parameter {
            description: "Owner".to_string(),
            param_type: ycallr_core::ParamType::String,
            required: true,
        },
    );
    params.insert(
        "repo".to_string(),
        ycallr_core::Parameter {
            description: "Repo".to_string(),
            param_type: ycallr_core::ParamType::String,
            required: true,
        },
    );

    let mut auth_map = HashMap::new();
    auth_map.insert(
        "primary".to_string(),
        AuthConfig::bearer("yaml-token-123".to_string()),
    );

    commands.insert(
        "get-repo".to_string(),
        ycallr_core::Command {
            description: Some("Get repo".to_string()),
            endpoint: Some("/repos/{owner}/{repo}".to_string()),
            method: Some(ycallr_core::HttpMethod::GET),
            auth: Some("primary".to_string()),
            headers: HashMap::new(),
            params,
            body: None,
            responses: None,
            commands: None,
        },
    );

    let api = ApiDefinition {
        name: "github".to_string(),
        version: "1.0.0".to_string(),
        description: "GitHub API".to_string(),
        base_url: server.url(),
        env: vec![],
        auth: auth_map,
        commands,
        errors: None,
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
    params.insert(
        "owner".to_string(),
        ycallr_core::Parameter {
            description: "Owner".to_string(),
            param_type: ycallr_core::ParamType::String,
            required: true,
        },
    );
    params.insert(
        "repo".to_string(),
        ycallr_core::Parameter {
            description: "Repo".to_string(),
            param_type: ycallr_core::ParamType::String,
            required: true,
        },
    );

    let mut auth_map = HashMap::new();
    auth_map.insert(
        "secondary".to_string(),
        AuthConfig::api_key("my-api-key".to_string(), "X-API-Key".to_string()),
    );

    commands.insert(
        "get-repo".to_string(),
        ycallr_core::Command {
            description: Some("Get repo".to_string()),
            endpoint: Some("/repos/{owner}/{repo}".to_string()),
            method: Some(ycallr_core::HttpMethod::GET),
            auth: Some("secondary".to_string()),
            headers: HashMap::new(),
            params,
            body: None,
            responses: None,
            commands: None,
        },
    );

    let api = ApiDefinition {
        name: "github".to_string(),
        version: "1.0.0".to_string(),
        description: "GitHub API".to_string(),
        base_url: server.url(),
        env: vec![],
        auth: auth_map,
        commands,
        errors: None,
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
    params.insert(
        "owner".to_string(),
        ycallr_core::Parameter {
            description: "Owner".to_string(),
            param_type: ycallr_core::ParamType::String,
            required: true,
        },
    );
    params.insert(
        "repo".to_string(),
        ycallr_core::Parameter {
            description: "Repo".to_string(),
            param_type: ycallr_core::ParamType::String,
            required: true,
        },
    );

    let mut auth_map = HashMap::new();
    auth_map.insert(
        "primary".to_string(),
        AuthConfig::bearer("token".to_string()),
    );

    commands.insert(
        "get-repo".to_string(),
        ycallr_core::Command {
            description: Some("Get repo".to_string()),
            endpoint: Some("/repos/{owner}/{repo}".to_string()),
            method: Some(ycallr_core::HttpMethod::GET),
            auth: None,
            headers: HashMap::new(),
            params,
            body: None,
            responses: None,
            commands: None,
        },
    );

    let api = ApiDefinition {
        name: "github".to_string(),
        version: "1.0.0".to_string(),
        description: "GitHub API".to_string(),
        base_url: server.url(),
        env: vec![],
        auth: auth_map,
        commands,
        errors: None,
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
    auth_map.insert(
        "primary".to_string(),
        AuthConfig::bearer("token1".to_string()),
    );
    auth_map.insert(
        "secondary".to_string(),
        AuthConfig::api_key("key1".to_string(), "X-Key".to_string()),
    );

    let mut commands = HashMap::new();
    let api = ApiDefinition {
        name: "test".to_string(),
        version: "1.0.0".to_string(),
        description: "Test".to_string(),
        base_url: "https://api.test.com".to_string(),
        env: vec![],
        auth: auth_map,
        commands,
        errors: None,
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

#[test]
fn test_get_query_params_appended() {
    let mut server = mockito::Server::new();

    let mock = server
        .mock("GET", "/repos/rust-lang/rust/issues")
        .match_query(mockito::Matcher::UrlEncoded("state".into(), "open".into()))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"[]"#)
        .create();

    let mut commands = HashMap::new();
    let mut params = HashMap::new();
    params.insert(
        "owner".to_string(),
        ycallr_core::Parameter {
            description: "Owner".to_string(),
            param_type: ycallr_core::ParamType::String,
            required: true,
        },
    );
    params.insert(
        "repo".to_string(),
        ycallr_core::Parameter {
            description: "Repo".to_string(),
            param_type: ycallr_core::ParamType::String,
            required: true,
        },
    );
    params.insert(
        "state".to_string(),
        ycallr_core::Parameter {
            description: "State".to_string(),
            param_type: ycallr_core::ParamType::String,
            required: false,
        },
    );

    commands.insert(
        "list-issues".to_string(),
        ycallr_core::Command {
            description: Some("List issues".to_string()),
            endpoint: Some("/repos/{owner}/{repo}/issues".to_string()),
            method: Some(ycallr_core::HttpMethod::GET),
            auth: None,
            headers: HashMap::new(),
            params,
            body: None,
            responses: None,
            commands: None,
        },
    );

    let api = ApiDefinition {
        name: "github".to_string(),
        version: "1.0.0".to_string(),
        description: "GitHub API".to_string(),
        base_url: server.url(),
        env: vec![],
        auth: HashMap::new(),
        commands,
        errors: None,
    };

    let client = YcallrClient::new(api).unwrap();
    let mut call_params = HashMap::new();
    call_params.insert("owner".to_string(), "rust-lang".to_string());
    call_params.insert("repo".to_string(), "rust".to_string());
    call_params.insert("state".to_string(), "open".to_string());

    let response = client.call("list-issues", &call_params, None).unwrap();

    mock.assert();
    assert_eq!(response.status, 200);
}

#[test]
fn test_query_auth_preserves_headers() {
    let mut server = mockito::Server::new();

    let mock = server
        .mock("GET", "/api/data")
        .match_header("Accept", "application/json")
        .match_query(mockito::Matcher::UrlEncoded(
            "apikey".into(),
            "secret".into(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"ok": true}"#)
        .create();

    let mut auth_map = HashMap::new();
    auth_map.insert(
        "query_key".to_string(),
        AuthConfig::api_key_in(
            "secret".to_string(),
            "apikey".to_string(),
            ycallr_core::models::ApiKeyLocation::Query,
        ),
    );

    let mut commands = HashMap::new();
    commands.insert(
        "query-auth".to_string(),
        ycallr_core::Command {
            description: None,
            endpoint: Some("/api/data".to_string()),
            method: Some(ycallr_core::HttpMethod::GET),
            auth: Some("query_key".to_string()),
            headers: HashMap::from([("Accept".to_string(), "application/json".to_string())]),
            params: HashMap::new(),
            body: None,
            responses: None,
            commands: None,
        },
    );

    let api = ApiDefinition {
        name: "test".to_string(),
        version: "1.0.0".to_string(),
        description: "Test API".to_string(),
        base_url: server.url(),
        env: vec![],
        auth: auth_map,
        commands,
        errors: None,
    };

    let client = YcallrClient::new(api).unwrap();
    let response = client.call("query-auth", &HashMap::new(), None).unwrap();

    mock.assert();
    assert_eq!(response.status, 200);
}

#[test]
fn test_missing_named_auth_errors() {
    let mut commands = HashMap::new();
    commands.insert(
        "get-repo".to_string(),
        ycallr_core::Command {
            description: None,
            endpoint: Some("/repos/{owner}/{repo}".to_string()),
            method: Some(ycallr_core::HttpMethod::GET),
            auth: Some("missing".to_string()),
            headers: HashMap::new(),
            params: HashMap::new(),
            body: None,
            responses: None,
            commands: None,
        },
    );

    let api = ApiDefinition {
        name: "github".to_string(),
        version: "1.0.0".to_string(),
        description: "GitHub API".to_string(),
        base_url: "https://api.github.com".to_string(),
        env: vec![],
        auth: HashMap::new(),
        commands,
        errors: None,
    };

    let result = YcallrClient::new(api);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("missing"));
}

#[test]
fn test_missing_required_param_errors() {
    let mut server = mockito::Server::new();

    let api = create_test_api(&server.url());
    let client = YcallrClient::new(api).unwrap();

    let result = client.call("get-repo", &HashMap::new(), None);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Missing required parameter"));
}

#[test]
fn test_invalid_param_type_errors() {
    let mut server = mockito::Server::new();

    let mut commands = HashMap::new();
    commands.insert(
        "get-item".to_string(),
        ycallr_core::Command {
            description: None,
            endpoint: Some("/items/{id}".to_string()),
            method: Some(ycallr_core::HttpMethod::GET),
            auth: None,
            headers: HashMap::new(),
            params: HashMap::from([(
                "id".to_string(),
                ycallr_core::Parameter {
                    description: "Item ID".to_string(),
                    param_type: ycallr_core::ParamType::Number,
                    required: true,
                },
            )]),
            body: None,
            responses: None,
            commands: None,
        },
    );

    let api = ApiDefinition {
        name: "test".to_string(),
        version: "1.0.0".to_string(),
        description: "Test".to_string(),
        base_url: server.url(),
        env: vec![],
        auth: HashMap::new(),
        commands,
        errors: None,
    };

    let client = YcallrClient::new(api).unwrap();
    let result = client.call(
        "get-item",
        &HashMap::from([("id".to_string(), "not-a-number".to_string())]),
        None,
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("must be a number"));
}

#[test]
fn test_unknown_param_errors() {
    let mut server = mockito::Server::new();
    let api = create_test_api(&server.url());
    let client = YcallrClient::new(api).unwrap();

    let mut params = HashMap::new();
    params.insert("owner".to_string(), "rust-lang".to_string());
    params.insert("repo".to_string(), "rust".to_string());
    params.insert("unknown".to_string(), "x".to_string());

    let result = client.call("get-repo", &params, None);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Unknown parameter"));
}

#[test]
fn test_path_params_are_url_encoded() {
    let mut server = mockito::Server::new();

    let mock = server
        .mock("GET", "/repos/foo%26bar/a%2Fb")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"name": "encoded"}"#)
        .create();

    let api = create_test_api(&server.url());
    let client = YcallrClient::new(api).unwrap();

    let mut params = HashMap::new();
    params.insert("owner".to_string(), "foo&bar".to_string());
    params.insert("repo".to_string(), "a/b".to_string());

    let response = client.call("get-repo", &params, None).unwrap();

    mock.assert();
    assert_eq!(response.status, 200);
    assert_eq!(response.body["name"], "encoded");
}

#[test]
fn test_query_params_are_url_encoded() {
    let mut server = mockito::Server::new();

    let mock = server
        .mock("GET", "/repos/rust-lang/rust/issues")
        .match_query(mockito::Matcher::AllOf(vec![
            mockito::Matcher::UrlEncoded("state".into(), "open".into()),
            mockito::Matcher::UrlEncoded("labels".into(), "bug&help".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"[]"#)
        .create();

    let mut commands = HashMap::new();
    let mut params = HashMap::new();
    params.insert(
        "owner".to_string(),
        ycallr_core::Parameter {
            description: "Owner".to_string(),
            param_type: ycallr_core::ParamType::String,
            required: true,
        },
    );
    params.insert(
        "repo".to_string(),
        ycallr_core::Parameter {
            description: "Repo".to_string(),
            param_type: ycallr_core::ParamType::String,
            required: true,
        },
    );
    params.insert(
        "state".to_string(),
        ycallr_core::Parameter {
            description: "State".to_string(),
            param_type: ycallr_core::ParamType::String,
            required: false,
        },
    );
    params.insert(
        "labels".to_string(),
        ycallr_core::Parameter {
            description: "Labels".to_string(),
            param_type: ycallr_core::ParamType::String,
            required: false,
        },
    );

    commands.insert(
        "list-issues".to_string(),
        ycallr_core::Command {
            description: Some("List issues".to_string()),
            endpoint: Some("/repos/{owner}/{repo}/issues".to_string()),
            method: Some(ycallr_core::HttpMethod::GET),
            auth: None,
            headers: HashMap::new(),
            params,
            body: None,
            responses: None,
            commands: None,
        },
    );

    let api = ApiDefinition {
        name: "github".to_string(),
        version: "1.0.0".to_string(),
        description: "GitHub API".to_string(),
        base_url: server.url(),
        env: vec![],
        auth: HashMap::new(),
        commands,
        errors: None,
    };

    let client = YcallrClient::new(api).unwrap();
    let mut call_params = HashMap::new();
    call_params.insert("owner".to_string(), "rust-lang".to_string());
    call_params.insert("repo".to_string(), "rust".to_string());
    call_params.insert("state".to_string(), "open".to_string());
    call_params.insert("labels".to_string(), "bug&help".to_string());

    let response = client.call("list-issues", &call_params, None).unwrap();

    mock.assert();
    assert_eq!(response.status, 200);
}

#[test]
fn test_hybrid_command_call_repos() {
    let mut server = mockito::Server::new();

    let mock = server
        .mock("GET", "/repos")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"[{"full_name": "rust-lang/rust"}]"#)
        .create();

    let api = create_nested_test_api(&server.url());
    let client = YcallrClient::new(api).unwrap();

    let response = client.call("repos", &HashMap::new(), None).unwrap();

    mock.assert();
    assert_eq!(response.status, 200);
    assert!(response.body.is_array());
}

#[test]
fn test_hybrid_command_call_repos_issues() {
    let mut server = mockito::Server::new();

    let mock = server
        .mock("GET", "/repos/rust-lang/rust/issues")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"[{"title": "bug"}]"#)
        .create();

    let api = create_nested_test_api(&server.url());
    let client = YcallrClient::new(api).unwrap();

    let params = HashMap::from([
        ("owner".to_string(), "rust-lang".to_string()),
        ("repo".to_string(), "rust".to_string()),
    ]);

    let response = client.call("repos.issues", &params, None).unwrap();

    mock.assert();
    assert_eq!(response.status, 200);
}

#[test]
fn test_hybrid_command_call_leaf_create() {
    let mut server = mockito::Server::new();

    let mock = server
        .mock("POST", "/repos/rust-lang/rust/issues")
        .with_status(201)
        .with_header("content-type", "application/json")
        .with_body(r#"{"title": "bug"}"#)
        .create();

    let api = create_nested_test_api(&server.url());
    let client = YcallrClient::new(api).unwrap();

    let params = HashMap::from([
        ("owner".to_string(), "rust-lang".to_string()),
        ("repo".to_string(), "rust".to_string()),
    ]);
    let body = serde_json::json!({"title": "bug"});

    let response = client
        .call("repos.issues.create", &params, Some(&body))
        .unwrap();

    mock.assert();
    assert_eq!(response.status, 201);
}

#[test]
fn test_branch_only_command_call_fails() {
    let mut server = mockito::Server::new();

    let mock = server
        .mock("GET", "/repos/rust-lang/rust/issues")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"[]"#)
        .create();

    let mut issues_commands = HashMap::new();
    issues_commands.insert(
        "list".to_string(),
        Command {
            description: Some("List issues".to_string()),
            endpoint: Some("/repos/{owner}/{repo}/issues".to_string()),
            method: Some(HttpMethod::GET),
            auth: None,
            headers: HashMap::new(),
            params: owner_repo_params(),
            body: None,
            commands: None,
            responses: None,
        },
    );

    let mut repos_commands = HashMap::new();
    repos_commands.insert(
        "issues".to_string(),
        Command {
            description: Some("Issues operations".to_string()),
            endpoint: None,
            method: None,
            auth: None,
            headers: HashMap::new(),
            params: HashMap::new(),
            body: None,
            commands: Some(issues_commands),
            responses: None,
        },
    );

    let mut commands = HashMap::new();
    commands.insert(
        "repos".to_string(),
        Command {
            description: Some("Repository operations".to_string()),
            endpoint: None,
            method: None,
            auth: None,
            headers: HashMap::new(),
            params: HashMap::new(),
            body: None,
            commands: Some(repos_commands),
            responses: None,
        },
    );

    let api = ApiDefinition {
        name: "branch-only".to_string(),
        version: "1.0.0".to_string(),
        description: "Branch-only nested API".to_string(),
        base_url: server.url(),
        env: vec![],
        auth: HashMap::new(),
        commands,
        errors: None,
    };

    let client = YcallrClient::new(api).unwrap();
    let result = client.call("repos", &HashMap::new(), None);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("no endpoint"));

    let params = HashMap::from([
        ("owner".to_string(), "rust-lang".to_string()),
        ("repo".to_string(), "rust".to_string()),
    ]);
    let leaf_result = client.call("repos.issues.list", &params, None);
    assert!(leaf_result.is_ok());
    mock.assert();
}

#[test]
fn test_command_details_hybrid_and_subcommands() {
    let mut server = mockito::Server::new();
    let api = create_nested_test_api(&server.url());
    let client = YcallrClient::new(api).unwrap();

    let repos = client.command_details("repos").unwrap();
    assert_eq!(repos.path, "repos");
    assert_eq!(repos.endpoint.as_deref(), Some("/repos"));
    assert_eq!(repos.method.as_ref(), Some(&HttpMethod::GET));
    assert!(repos.is_branch);
    assert!(repos.is_leaf);
    assert!(repos.is_callable);
    assert!(repos.params.is_empty());
    assert_eq!(repos.subcommands, vec!["issues".to_string()]);

    let issues = client.command_details("repos.issues").unwrap();
    assert_eq!(issues.path, "repos.issues");
    assert_eq!(issues.params.len(), 2);
    assert!(issues.params.contains_key("owner"));
    assert!(issues.params.contains_key("repo"));
    assert_eq!(issues.subcommands, vec!["create".to_string()]);

    let subcmds = client.list_subcommands("repos").unwrap();
    assert_eq!(subcmds, vec!["issues".to_string()]);
}

#[test]
fn test_env_substitution_in_http_headers() {
    let mut server = mockito::Server::new();

    let mock = server
        .mock("GET", "/repos/rust-lang/rust")
        .match_header("Authorization", "Bearer ghp_secret_token")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"name": "rust"}"#)
        .create();

    let api = create_env_test_api(&server.url());
    let client = YcallrClient::builder(api)
        .env_mode(EnvMode::Manual)
        .env("GITHUB_TOKEN", "ghp_secret_token")
        .build()
        .unwrap();

    let params = HashMap::from([
        ("owner".to_string(), "rust-lang".to_string()),
        ("repo".to_string(), "rust".to_string()),
    ]);
    let response = client.call("get-repo", &params, None).unwrap();

    mock.assert();
    assert_eq!(response.status, 200);
}

#[test]
fn test_http_put_request() {
    let mut server = mockito::Server::new();

    let mock = server
        .mock("PUT", "/items/42")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"id": 42, "title": "updated"}"#)
        .create();

    let api = create_method_api(&server.url(), "update-item", HttpMethod::PUT, "/items/{id}");
    let client = YcallrClient::new(api).unwrap();

    let params = HashMap::from([("id".to_string(), "42".to_string())]);
    let body = serde_json::json!({"title": "updated"});
    let response = client.call("update-item", &params, Some(&body)).unwrap();

    mock.assert();
    assert_eq!(response.body["title"], "updated");
}

#[test]
fn test_http_delete_request() {
    let mut server = mockito::Server::new();

    let mock = server.mock("DELETE", "/items/42").with_status(204).create();

    let api = create_method_api(
        &server.url(),
        "delete-item",
        HttpMethod::DELETE,
        "/items/{id}",
    );
    let client = YcallrClient::new(api).unwrap();

    let params = HashMap::from([("id".to_string(), "42".to_string())]);
    let response = client.call("delete-item", &params, None).unwrap();

    mock.assert();
    assert_eq!(response.status, 204);
}

#[test]
fn test_http_patch_request() {
    let mut server = mockito::Server::new();

    let mock = server
        .mock("PATCH", "/items/42")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"id": 42, "active": true}"#)
        .create();

    let api = create_method_api(
        &server.url(),
        "patch-item",
        HttpMethod::PATCH,
        "/items/{id}",
    );
    let client = YcallrClient::new(api).unwrap();

    let params = HashMap::from([("id".to_string(), "42".to_string())]);
    let body = serde_json::json!({"active": true});
    let response = client.call("patch-item", &params, Some(&body)).unwrap();

    mock.assert();
    assert_eq!(response.body["active"], true);
}

#[test]
fn test_yaml_form_body_over_http() {
    let mut server = mockito::Server::new();

    let mock = server
        .mock("POST", "/auth/login/testuser")
        .match_body(mockito::Matcher::AllOf(vec![
            mockito::Matcher::UrlEncoded("username".into(), "testuser".into()),
            mockito::Matcher::UrlEncoded("password".into(), "secret123".into()),
            mockito::Matcher::UrlEncoded("grant_type".into(), "password".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"token": "abc"}"#)
        .create();

    let yaml = r#"
name: form-api
version: "1.0.0"
base_url: PLACEHOLDER
commands:
  login:
    endpoint: /auth/login/{user}
    method: POST
    params:
      user:
        description: Username
        type: string
        required: true
    body:
      form:
        username: "{user}"
        password: "secret123"
        grant_type: "password"
"#;
    let yaml = yaml.replace("PLACEHOLDER", "https://api.test.com");
    let mut api = ycallr_core::yaml_parser::parse_yaml(&yaml).unwrap();
    api.base_url = server.url();
    let client = YcallrClient::new(api).unwrap();

    let params = HashMap::from([("user".to_string(), "testuser".to_string())]);
    let response = client.call("login", &params, None).unwrap();

    mock.assert();
    assert_eq!(response.body["token"], "abc");
}

#[test]
fn test_yaml_raw_body_over_http() {
    let mut server = mockito::Server::new();

    let mock = server
        .mock("POST", "/api/xml/alice")
        .match_body("<request><name>alice</name></request>")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"ok": true}"#)
        .create();

    let yaml = r#"
name: raw-api
version: "1.0.0"
base_url: PLACEHOLDER
commands:
  send-xml:
    endpoint: /api/xml/{name}
    method: POST
    params:
      name:
        description: Name
        type: string
        required: true
    body:
      raw: "<request><name>{name}</name></request>"
"#;
    let yaml = yaml.replace("PLACEHOLDER", "https://api.test.com");
    let mut api = ycallr_core::yaml_parser::parse_yaml(&yaml).unwrap();
    api.base_url = server.url();
    let client = YcallrClient::new(api).unwrap();

    let params = HashMap::from([("name".to_string(), "alice".to_string())]);
    let response = client.call("send-xml", &params, None).unwrap();

    mock.assert();
    assert_eq!(response.body["ok"], true);
}

#[test]
fn test_cookie_auth_over_http() {
    let mut server = mockito::Server::new();

    let mock = server
        .mock("GET", "/session")
        .match_header("cookie", "session=cookie-value-123")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"authenticated": true}"#)
        .create();

    let mut auth = HashMap::new();
    auth.insert(
        "cookie_auth".to_string(),
        AuthConfig::ApiKey {
            key: "cookie-value-123".to_string(),
            name: "session".to_string(),
            in_: ycallr_core::ApiKeyLocation::Cookie,
        },
    );

    let mut commands = HashMap::new();
    commands.insert(
        "check-session".to_string(),
        Command {
            description: None,
            endpoint: Some("/session".to_string()),
            method: Some(HttpMethod::GET),
            auth: Some("cookie_auth".to_string()),
            headers: HashMap::new(),
            params: HashMap::new(),
            body: None,
            commands: None,
            responses: None,
        },
    );

    let api = ApiDefinition {
        name: "cookie-api".to_string(),
        version: "1.0.0".to_string(),
        description: "Cookie auth".to_string(),
        base_url: server.url(),
        env: vec![],
        auth,
        commands,
        errors: None,
    };

    let client = YcallrClient::new(api).unwrap();
    let response = client.call("check-session", &HashMap::new(), None).unwrap();

    mock.assert();
    assert_eq!(response.body["authenticated"], true);
}

#[test]
fn test_basic_auth_over_http() {
    let mut server = mockito::Server::new();

    let mock = server
        .mock("GET", "/protected")
        .match_header("authorization", "Basic dXNlcjpwYXNz")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"ok": true}"#)
        .create();

    let api = create_http_auth_api(
        &server.url(),
        AuthConfig::Http {
            scheme: "basic".to_string(),
            token: None,
            username: Some("user".to_string()),
            password: Some("pass".to_string()),
            prefix: None,
        },
    );

    let client = YcallrClient::new(api).unwrap();
    let response = client.call("protected", &HashMap::new(), None).unwrap();

    mock.assert();
    assert_eq!(response.body["ok"], true);
}

#[test]
fn test_custom_http_auth_over_http() {
    let mut server = mockito::Server::new();

    let mock = server
        .mock("GET", "/protected")
        .match_header("authorization", "Token my-api-token")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"ok": true}"#)
        .create();

    let api = create_http_auth_api(
        &server.url(),
        AuthConfig::Http {
            scheme: "custom".to_string(),
            token: Some("my-api-token".to_string()),
            username: None,
            password: None,
            prefix: Some("Token ".to_string()),
        },
    );

    let client = YcallrClient::new(api).unwrap();
    let response = client.call("protected", &HashMap::new(), None).unwrap();

    mock.assert();
    assert_eq!(response.body["ok"], true);
}

#[test]
fn test_response_exact_status_code_message() {
    let mut server = mockito::Server::new();

    let mock = server
        .mock("GET", "/repos/missing/rust")
        .with_status(404)
        .with_header("content-type", "application/json")
        .with_body(r#"{"message": "Not Found"}"#)
        .create();

    let api = create_api_with_responses(&server.url());
    let client = YcallrClient::new(api).unwrap();

    let params = HashMap::from([
        ("owner".to_string(), "missing".to_string()),
        ("repo".to_string(), "rust".to_string()),
    ]);
    let response = client.call("get-repo", &params, None).unwrap();

    mock.assert();
    assert_eq!(response.status, 404);
    assert_eq!(response.message.as_deref(), Some("Not found: missing"));
}

#[test]
fn test_response_warn_status_message() {
    let mut server = mockito::Server::new();

    let mock = server
        .mock("GET", "/redirect")
        .with_status(301)
        .with_header("content-type", "application/json")
        .with_body(r#"{"location": "/new-path"}"#)
        .create();

    let mut commands = HashMap::new();
    commands.insert(
        "redirect".to_string(),
        Command {
            description: None,
            endpoint: Some("/redirect".to_string()),
            method: Some(HttpMethod::GET),
            auth: None,
            headers: HashMap::new(),
            params: HashMap::new(),
            body: None,
            commands: None,
            responses: Some(ycallr_core::ResponseConfig {
                success: None,
                failure: None,
                warn: Some(ycallr_core::ResponseEntry {
                    message: "Redirected to {output.location}".to_string(),
                }),
                codes: HashMap::new(),
            }),
        },
    );

    let api = ApiDefinition {
        name: "redirect-api".to_string(),
        version: "1.0.0".to_string(),
        description: "Redirect API".to_string(),
        base_url: server.url(),
        env: vec![],
        auth: HashMap::new(),
        commands,
        errors: None,
    };

    let client = YcallrClient::new(api).unwrap();
    let response = client.call("redirect", &HashMap::new(), None).unwrap();

    mock.assert();
    assert_eq!(response.status, 301);
    assert_eq!(response.message.as_deref(), Some("Redirected to /new-path"));
}

#[test]
fn test_non_json_response_parsed_as_string() {
    let mut server = mockito::Server::new();

    let mock = server
        .mock("GET", "/plain")
        .with_status(200)
        .with_header("content-type", "text/plain")
        .with_body("plain text response")
        .create();

    let api = create_method_api(&server.url(), "get-plain", HttpMethod::GET, "/plain");
    let client = YcallrClient::new(api).unwrap();
    let response = client.call("get-plain", &HashMap::new(), None).unwrap();

    mock.assert();
    assert_eq!(
        response.body,
        serde_json::Value::String("plain text response".to_string())
    );
}

#[test]
fn test_call_body_overrides_yaml_body() {
    let mut server = mockito::Server::new();

    let mock = server
        .mock("POST", "/items")
        .match_body(r#"{"caller":"value"}"#)
        .with_status(201)
        .with_header("content-type", "application/json")
        .with_body(r#"{"id": 1}"#)
        .create();

    let yaml = r#"
name: body-api
version: "1.0.0"
base_url: PLACEHOLDER
commands:
  create:
    endpoint: /items
    method: POST
    body:
      json:
        yaml_only: "ignored"
"#;
    let yaml = yaml.replace("PLACEHOLDER", "https://api.test.com");
    let mut api = ycallr_core::yaml_parser::parse_yaml(&yaml).unwrap();
    api.base_url = server.url();
    let client = YcallrClient::new(api).unwrap();

    let caller_body = serde_json::json!({"caller": "value"});
    let response = client
        .call("create", &HashMap::new(), Some(&caller_body))
        .unwrap();

    mock.assert();
    assert_eq!(response.status, 201);
}

#[test]
fn test_base_url_trailing_slash_join() {
    let mut server = mockito::Server::new();

    let mock = server
        .mock("GET", "/repos")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"[]"#)
        .create();

    let api = ApiDefinition {
        name: "slash-api".to_string(),
        version: "1.0.0".to_string(),
        description: "Trailing slash".to_string(),
        base_url: format!("{}/", server.url()),
        env: vec![],
        auth: HashMap::new(),
        commands: HashMap::from([(
            "list".to_string(),
            Command {
                description: None,
                endpoint: Some("/repos".to_string()),
                method: Some(HttpMethod::GET),
                auth: None,
                headers: HashMap::new(),
                params: HashMap::new(),
                body: None,
                commands: None,
                responses: None,
            },
        )]),
        errors: None,
    };

    let client = YcallrClient::new(api).unwrap();
    let response = client.call("list", &HashMap::new(), None).unwrap();

    mock.assert();
    assert_eq!(response.status, 200);
}

#[test]
fn test_empty_bearer_auth_token_errors() {
    let mut server = mockito::Server::new();
    let api = create_test_api(&server.url());
    let client = YcallrClient::with_auth(
        api,
        AuthConfig::Bearer {
            token: "".to_string(),
        },
    )
    .unwrap();

    let params = HashMap::from([
        ("owner".to_string(), "rust-lang".to_string()),
        ("repo".to_string(), "rust".to_string()),
    ]);

    let result = client.call("get-repo", &params, None);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("bearer token"));
}

#[test]
fn test_unresolved_bearer_env_token_errors() {
    let mut server = mockito::Server::new();

    let mut commands = HashMap::new();
    commands.insert(
        "get-repo".to_string(),
        Command {
            description: None,
            endpoint: Some("/repos/{owner}/{repo}".to_string()),
            method: Some(HttpMethod::GET),
            auth: Some("primary".to_string()),
            headers: HashMap::new(),
            params: owner_repo_params(),
            body: None,
            commands: None,
            responses: None,
        },
    );

    let mut auth = HashMap::new();
    auth.insert(
        "primary".to_string(),
        AuthConfig::Bearer {
            token: "${GITHUB_TOKEN}".to_string(),
        },
    );

    let api = ApiDefinition {
        name: "auth-env".to_string(),
        version: "1.0.0".to_string(),
        description: "Auth env test".to_string(),
        base_url: server.url(),
        env: vec![ycallr_core::models::EnvVar {
            name: "GITHUB_TOKEN".to_string(),
            required: true,
        }],
        auth,
        commands,
        errors: None,
    };

    let result = YcallrClient::builder(api)
        .env_mode(EnvMode::Manual)
        .env("GITHUB_TOKEN", "")
        .build();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("cannot be empty"));
}

#[test]
fn test_global_auth_skipped_when_command_auth_none() {
    let mut server = mockito::Server::new();

    let mock = server
        .mock("GET", "/public")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"ok": true}"#)
        .create();

    let mut commands = HashMap::new();
    commands.insert(
        "public".to_string(),
        Command {
            description: Some("Public endpoint".to_string()),
            endpoint: Some("/public".to_string()),
            method: Some(HttpMethod::GET),
            auth: Some(ycallr_core::COMMAND_AUTH_NONE.to_string()),
            headers: HashMap::new(),
            params: HashMap::new(),
            body: None,
            commands: None,
            responses: None,
        },
    );

    let api = ApiDefinition {
        name: "auth-opt-out".to_string(),
        version: "1.0.0".to_string(),
        description: "Auth opt-out".to_string(),
        base_url: server.url(),
        env: vec![],
        auth: HashMap::new(),
        commands,
        errors: None,
    };

    let client = YcallrClient::with_auth(
        api,
        AuthConfig::Bearer {
            token: "global-token".to_string(),
        },
    )
    .unwrap();

    let response = client.call("public", &HashMap::new(), None).unwrap();

    mock.assert();
    assert_eq!(response.status, 200);
    assert_eq!(response.body["ok"], true);
}

#[test]
fn test_path_placeholders_without_yaml_params_over_http() {
    let mut server = mockito::Server::new();

    let mock = server
        .mock("GET", "/users/octocat")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"login": "octocat"}"#)
        .create();

    let yaml = r#"
name: users-api
version: "1.0.0"
base_url: PLACEHOLDER
commands:
  get-user:
    endpoint: /users/{username}
    method: GET
"#;
    let yaml = yaml.replace("PLACEHOLDER", "https://api.test.com");
    let mut api = ycallr_core::yaml_parser::parse_yaml(&yaml).unwrap();
    api.base_url = server.url();
    let client = YcallrClient::new(api).unwrap();

    let params = HashMap::from([("username".to_string(), "octocat".to_string())]);
    let response = client.call("get-user", &params, None).unwrap();

    mock.assert();
    assert_eq!(response.status, 200);
    assert_eq!(response.body["login"], "octocat");
}

#[test]
fn test_path_placeholder_with_query_param_without_yaml_path_param() {
    let mut server = mockito::Server::new();

    let mock = server
        .mock("GET", "/items/42")
        .match_query(mockito::Matcher::UrlEncoded(
            "filter".into(),
            "active".into(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"id": 42}"#)
        .create();

    let yaml = r#"
name: items-api
version: "1.0.0"
base_url: PLACEHOLDER
commands:
  get-item:
    endpoint: /items/{id}
    method: GET
    params:
      filter:
        description: Filter value
        type: string
        required: false
"#;
    let yaml = yaml.replace("PLACEHOLDER", "https://api.test.com");
    let mut api = ycallr_core::yaml_parser::parse_yaml(&yaml).unwrap();
    api.base_url = server.url();
    let client = YcallrClient::new(api).unwrap();

    let params = HashMap::from([
        ("id".to_string(), "42".to_string()),
        ("filter".to_string(), "active".to_string()),
    ]);
    let response = client.call("get-item", &params, None).unwrap();

    mock.assert();
    assert_eq!(response.body["id"], 42);
}

#[test]
fn test_yaml_json_body_over_http() {
    let mut server = mockito::Server::new();

    let mock = server
        .mock("POST", "/items")
        .match_body(mockito::Matcher::AllOf(vec![
            mockito::Matcher::PartialJson(serde_json::json!({"title": "hello"})),
            mockito::Matcher::PartialJson(serde_json::json!({"priority": 1})),
        ]))
        .with_status(201)
        .with_header("content-type", "application/json")
        .with_body(r#"{"id": 1, "title": "hello"}"#)
        .create();

    let yaml = r#"
name: json-body-api
version: "1.0.0"
base_url: PLACEHOLDER
commands:
  create-item:
    endpoint: /items
    method: POST
    body:
      json:
        title: "hello"
        priority: 1
"#;
    let yaml = yaml.replace("PLACEHOLDER", "https://api.test.com");
    let mut api = ycallr_core::yaml_parser::parse_yaml(&yaml).unwrap();
    api.base_url = server.url();
    let client = YcallrClient::new(api).unwrap();

    let response = client.call("create-item", &HashMap::new(), None).unwrap();

    mock.assert();
    assert_eq!(response.status, 201);
    assert_eq!(response.body["title"], "hello");
}

#[test]
fn test_mixed_body_types_rejected_at_yaml_parse() {
    let yaml = r#"
name: mixed-api
version: "1.0.0"
base_url: https://api.test.com
commands:
  broken:
    endpoint: /broken
    method: POST
    body:
      json:
        key: value
      raw: "not allowed"
"#;
    let result = ycallr_core::yaml_parser::parse_yaml(yaml);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("only one of json, form, raw, or multipart"));
}

#[test]
fn test_redirects_are_not_followed() {
    let mut server = mockito::Server::new();

    let mock = server
        .mock("GET", "/start")
        .with_status(302)
        .with_header("Location", "http://127.0.0.1/sensitive")
        .with_body("redirecting")
        .create();

    let mut commands = HashMap::new();
    commands.insert(
        "start".to_string(),
        Command {
            description: None,
            endpoint: Some("/start".to_string()),
            method: Some(HttpMethod::GET),
            auth: None,
            headers: HashMap::new(),
            params: HashMap::new(),
            body: None,
            commands: None,
            responses: None,
        },
    );

    let api = ApiDefinition {
        name: "redirect-api".to_string(),
        version: "1.0.0".to_string(),
        description: "Redirect API".to_string(),
        base_url: server.url(),
        env: vec![],
        auth: HashMap::new(),
        commands,
        errors: None,
    };

    let client = YcallrClient::new(api).unwrap();
    let response = client.call("start", &HashMap::new(), None).unwrap();

    mock.assert();
    assert_eq!(response.status, 302);
}

#[test]
fn test_cookie_auth_rejects_injection_chars() {
    let mut server = mockito::Server::new();

    let api = ApiDefinition {
        name: "cookie-api".to_string(),
        version: "1.0.0".to_string(),
        description: "Cookie API".to_string(),
        base_url: server.url(),
        env: vec![],
        auth: HashMap::new(),
        commands: HashMap::from([(
            "secure".to_string(),
            Command {
                description: None,
                endpoint: Some("/secure".to_string()),
                method: Some(HttpMethod::GET),
                auth: None,
                headers: HashMap::new(),
                params: HashMap::new(),
                body: None,
                commands: None,
                responses: None,
            },
        )]),
        errors: None,
    };

    let client = YcallrClient::with_auth(
        api,
        AuthConfig::api_key_in(
            "legit;admin=true".to_string(),
            "session".to_string(),
            ycallr_core::ApiKeyLocation::Cookie,
        ),
    )
    .unwrap();

    let result = client.call("secure", &HashMap::new(), None);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("invalid characters"));
}

#[test]
fn test_duplicate_query_param_from_auth_and_params_errors() {
    let mut server = mockito::Server::new();

    let mut params = HashMap::new();
    params.insert(
        "api_key".to_string(),
        Parameter {
            description: "API key".to_string(),
            param_type: ParamType::String,
            required: false,
        },
    );

    let mut commands = HashMap::new();
    commands.insert(
        "search".to_string(),
        Command {
            description: None,
            endpoint: Some("/search".to_string()),
            method: Some(HttpMethod::GET),
            auth: Some("primary".to_string()),
            headers: HashMap::new(),
            params,
            body: None,
            commands: None,
            responses: None,
        },
    );

    let mut auth = HashMap::new();
    auth.insert(
        "primary".to_string(),
        AuthConfig::api_key_in(
            "secret".to_string(),
            "api_key".to_string(),
            ycallr_core::ApiKeyLocation::Query,
        ),
    );

    let api = ApiDefinition {
        name: "query-api".to_string(),
        version: "1.0.0".to_string(),
        description: "Query API".to_string(),
        base_url: server.url(),
        env: vec![],
        auth,
        commands,
        errors: None,
    };

    let client = YcallrClient::new(api).unwrap();
    let call_params = HashMap::from([("api_key".to_string(), "caller-value".to_string())]);
    let result = client.call("search", &call_params, None);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Duplicate query parameter"));
}

fn create_method_api(
    base_url: &str,
    name: &str,
    method: HttpMethod,
    endpoint: &str,
) -> ApiDefinition {
    let mut params = HashMap::new();
    if endpoint.contains("{id}") {
        params.insert(
            "id".to_string(),
            Parameter {
                description: "ID".to_string(),
                param_type: ParamType::String,
                required: true,
            },
        );
    }

    let mut commands = HashMap::new();
    commands.insert(
        name.to_string(),
        Command {
            description: None,
            endpoint: Some(endpoint.to_string()),
            method: Some(method),
            auth: None,
            headers: HashMap::new(),
            params,
            body: None,
            commands: None,
            responses: None,
        },
    );

    ApiDefinition {
        name: "methods-api".to_string(),
        version: "1.0.0".to_string(),
        description: "HTTP methods".to_string(),
        base_url: base_url.to_string(),
        env: vec![],
        auth: HashMap::new(),
        commands,
        errors: None,
    }
}

fn create_http_auth_api(base_url: &str, auth_config: AuthConfig) -> ApiDefinition {
    let mut auth = HashMap::new();
    auth.insert("primary".to_string(), auth_config);

    let mut commands = HashMap::new();
    commands.insert(
        "protected".to_string(),
        Command {
            description: None,
            endpoint: Some("/protected".to_string()),
            method: Some(HttpMethod::GET),
            auth: Some("primary".to_string()),
            headers: HashMap::new(),
            params: HashMap::new(),
            body: None,
            commands: None,
            responses: None,
        },
    );

    ApiDefinition {
        name: "http-auth-api".to_string(),
        version: "1.0.0".to_string(),
        description: "HTTP auth".to_string(),
        base_url: base_url.to_string(),
        env: vec![],
        auth,
        commands,
        errors: None,
    }
}

fn create_api_with_responses(base_url: &str) -> ApiDefinition {
    let mut codes = HashMap::new();
    codes.insert(
        "404".to_string(),
        ycallr_core::ResponseEntry {
            message: "Not found: {input.owner}".to_string(),
        },
    );

    let responses = ycallr_core::ResponseConfig {
        success: Some(ycallr_core::ResponseEntry {
            message: "Got {output.name}".to_string(),
        }),
        failure: Some(ycallr_core::ResponseEntry {
            message: "Failed".to_string(),
        }),
        warn: None,
        codes,
    };

    let mut commands = HashMap::new();
    commands.insert(
        "get-repo".to_string(),
        Command {
            description: None,
            endpoint: Some("/repos/{owner}/{repo}".to_string()),
            method: Some(HttpMethod::GET),
            auth: None,
            headers: HashMap::new(),
            params: owner_repo_params(),
            body: None,
            commands: None,
            responses: Some(responses),
        },
    );

    ApiDefinition {
        name: "responses-api".to_string(),
        version: "1.0.0".to_string(),
        description: "Responses API".to_string(),
        base_url: base_url.to_string(),
        env: vec![],
        auth: HashMap::new(),
        commands,
        errors: None,
    }
}
