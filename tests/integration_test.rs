#![cfg(not(target_arch = "wasm32"))]

use std::collections::HashMap;
use ycallr_core::{ApiDefinition, HttpMethod, ParamType};

const FULL_YAML: &str = r#"
name: github
version: "1.0.0"
description: GitHub REST API
base_url: https://api.github.com
commands:
  create-issue:
    endpoint: /repos/{owner}/{repo}/issues
    method: POST
    headers:
      Accept: application/vnd.github.v3+json
      Content-Type: application/json
    params:
      owner:
        description: Repository owner
        type: string
        required: true
      repo:
        description: Repository name
        type: string
        required: true
      title:
        description: Issue title
        type: string
        required: true
      body:
        description: Issue body
        type: string
        required: false
  list-issues:
    endpoint: /repos/{owner}/{repo}/issues
    method: GET
    headers:
      Accept: application/vnd.github.v3+json
    params:
      owner:
        description: Repository owner
        type: string
        required: true
      repo:
        description: Repository name
        type: string
        required: true
      state:
        description: Filter by state
        type: string
        required: false
"#;

const ENV_YAML: &str = r#"
name: github
version: "1.0.0"
description: GitHub API with env vars
base_url: https://api.github.com
env:
  - name: GITHUB_TOKEN
    required: true
  - name: API_VERSION
    required: false
commands:
  get-repo:
    endpoint: /repos/{owner}/{repo}
    method: GET
    headers:
      Authorization: "Bearer ${GITHUB_TOKEN}"
      Accept: "application/vnd.github+json"
    params:
      owner:
        description: Repository owner
        type: string
        required: true
      repo:
        description: Repository name
        type: string
        required: true
"#;

const DESCRIPTION_YAML: &str = r#"
name: test
version: "1.0.0"
base_url: https://api.test.com
commands:
  get-item:
    description: "Retrieve an item by ID"
    endpoint: /items/{id}
    method: GET
    params:
      id:
        description: Item ID
        type: string
        required: true
  create-item:
    description: "Create a new item"
    endpoint: /items
    method: POST
"#;

const PATH_ALIAS_YAML: &str = r#"
name: test
version: "1.0.0"
base_url: https://api.test.com
commands:
  get-item:
    path: /items/{id}
    method: GET
    params:
      id:
        description: Item ID
        type: string
        required: true
"#;

#[test]
fn test_full_yaml_parsing() {
    let api = ycallr_core::yaml_parser::parse_yaml(FULL_YAML).unwrap();

    assert_eq!(api.name, "github");
    assert_eq!(api.version, "1.0.0");
    assert_eq!(api.base_url, "https://api.github.com");
    assert_eq!(api.commands.len(), 2);

    let create_issue = api.get_command("create-issue").unwrap();
    assert_eq!(create_issue.method.as_ref().unwrap(), &HttpMethod::POST);
    assert_eq!(create_issue.params.len(), 4);

    let list_issues = api.get_command("list-issues").unwrap();
    assert_eq!(list_issues.method.as_ref().unwrap(), &HttpMethod::GET);
}

#[test]
fn test_endpoint_resolution() {
    let api = ycallr_core::yaml_parser::parse_yaml(FULL_YAML).unwrap();
    let cmd = api.get_command("create-issue").unwrap();

    let mut params = HashMap::new();
    params.insert("owner".to_string(), "rust-lang".to_string());
    params.insert("repo".to_string(), "rust".to_string());

    let resolved = cmd.resolve_endpoint(&params).unwrap();
    assert_eq!(resolved, "/repos/rust-lang/rust/issues");
}

#[test]
fn test_protobuf_roundtrip() {
    let api = ycallr_core::yaml_parser::parse_yaml(FULL_YAML).unwrap();

    let proto_bytes = api.to_proto_bytes().unwrap();
    assert!(!proto_bytes.is_empty());

    let restored = ApiDefinition::from_proto_bytes(&proto_bytes).unwrap();
    assert_eq!(api.name, restored.name);
    assert_eq!(api.commands.len(), restored.commands.len());
}

#[test]
fn test_api_validation() {
    let valid_api = ApiDefinition {
        name: "test-api".to_string(),
        version: "1.0.0".to_string(),
        description: "Test".to_string(),
        base_url: "https://api.test.com".to_string(),
        env: vec![],
        commands: HashMap::new(),
    };
    assert!(valid_api.validate().is_ok());

    let invalid_api = ApiDefinition {
        name: "".to_string(),
        version: "1.0.0".to_string(),
        description: "Test".to_string(),
        base_url: "https://api.test.com".to_string(),
        env: vec![],
        commands: HashMap::new(),
    };
    assert!(invalid_api.validate().is_err());
}

#[test]
fn test_parameter_types() {
    let yaml = r#"
name: test
version: "1.0.0"
base_url: https://api.test.com
commands:
  test:
    endpoint: /test
    method: POST
    params:
      str_param:
        description: String param
        type: string
        required: true
      num_param:
        description: Number param
        type: number
        required: false
      bool_param:
        description: Boolean param
        type: boolean
        required: false
      arr_param:
        description: Array param
        type: array
        required: false
"#;

    let api = ycallr_core::yaml_parser::parse_yaml(yaml).unwrap();
    let cmd = api.commands.get("test").unwrap();

    assert_eq!(
        cmd.params.get("str_param").unwrap().param_type,
        ParamType::String
    );
    assert_eq!(
        cmd.params.get("num_param").unwrap().param_type,
        ParamType::Number
    );
    assert_eq!(
        cmd.params.get("bool_param").unwrap().param_type,
        ParamType::Boolean
    );
    assert_eq!(
        cmd.params.get("arr_param").unwrap().param_type,
        ParamType::Array
    );
}

#[test]
fn test_http_methods() {
    let yaml = r#"
name: test
version: "1.0.0"
base_url: https://api.test.com
commands:
  get_test:
    endpoint: /get
    method: GET
  post_test:
    endpoint: /post
    method: POST
  put_test:
    endpoint: /put
    method: PUT
  delete_test:
    endpoint: /delete
    method: DELETE
  patch_test:
    endpoint: /patch
    method: PATCH
"#;

    let api = ycallr_core::yaml_parser::parse_yaml(yaml).unwrap();

    assert_eq!(
        api.commands.get("get_test").unwrap().method,
        Some(HttpMethod::GET)
    );
    assert_eq!(
        api.commands.get("post_test").unwrap().method,
        Some(HttpMethod::POST)
    );
    assert_eq!(
        api.commands.get("put_test").unwrap().method,
        Some(HttpMethod::PUT)
    );
    assert_eq!(
        api.commands.get("delete_test").unwrap().method,
        Some(HttpMethod::DELETE)
    );
    assert_eq!(
        api.commands.get("patch_test").unwrap().method,
        Some(HttpMethod::PATCH)
    );
}

const NESTED_YAML: &str = r#"
name: github
version: "1.0.0"
description: GitHub REST API with nested commands
base_url: https://api.github.com
commands:
  repos:
    endpoint: /repos
    method: GET
    commands:
      issues:
        endpoint: /repos/{owner}/{repo}/issues
        method: GET
        commands:
          create:
            endpoint: /repos/{owner}/{repo}/issues
            method: POST
          list:
            endpoint: /repos/{owner}/{repo}/issues
            method: GET
  users:
    endpoint: /users
    method: GET
    commands:
      get:
        endpoint: /users/{username}
        method: GET
"#;

#[test]
fn test_nested_yaml_parsing() {
    let api = ycallr_core::yaml_parser::parse_yaml(NESTED_YAML).unwrap();
    assert_eq!(api.name, "github");
    assert_eq!(api.commands.len(), 2);
}

#[test]
fn test_nested_command_lookup() {
    let api = ycallr_core::yaml_parser::parse_yaml(NESTED_YAML).unwrap();

    let repos = api.get_command("repos");
    assert!(repos.is_ok());

    let issues = api.get_command("repos.issues");
    assert!(issues.is_ok());

    let create = api.get_command("repos.issues.create");
    assert!(create.is_ok());
    assert_eq!(create.unwrap().method.as_ref().unwrap(), &HttpMethod::POST);

    let list = api.get_command("repos.issues.list");
    assert!(list.is_ok());
    assert_eq!(list.unwrap().method.as_ref().unwrap(), &HttpMethod::GET);

    let users_get = api.get_command("users.get");
    assert!(users_get.is_ok());
    assert_eq!(
        users_get.unwrap().method.as_ref().unwrap(),
        &HttpMethod::GET
    );
}

#[test]
fn test_nested_command_not_found() {
    let api = ycallr_core::yaml_parser::parse_yaml(NESTED_YAML).unwrap();

    assert!(api.get_command("nonexistent").is_err());
    assert!(api.get_command("repos.nonexistent").is_err());
    assert!(api.get_command("repos.issues.nonexistent").is_err());
}

#[test]
fn test_nested_endpoint_resolution() {
    let api = ycallr_core::yaml_parser::parse_yaml(NESTED_YAML).unwrap();
    let cmd = api.get_command("repos.issues.create").unwrap();

    let mut params = HashMap::new();
    params.insert("owner".to_string(), "rust-lang".to_string());
    params.insert("repo".to_string(), "rust".to_string());

    let resolved = cmd.resolve_endpoint(&params).unwrap();
    assert_eq!(resolved, "/repos/rust-lang/rust/issues");
}

#[test]
fn test_nested_protobuf_roundtrip() {
    let api = ycallr_core::yaml_parser::parse_yaml(NESTED_YAML).unwrap();
    let proto_bytes = api.to_proto_bytes().unwrap();
    let restored = ApiDefinition::from_proto_bytes(&proto_bytes).unwrap();

    assert_eq!(api.name, restored.name);
    assert_eq!(api.commands.len(), restored.commands.len());

    let create = restored.get_command("repos.issues.create").unwrap();
    assert_eq!(create.method.as_ref().unwrap(), &HttpMethod::POST);
}

#[test]
fn test_env_yaml_parsing() {
    let api = ycallr_core::yaml_parser::parse_yaml(ENV_YAML).unwrap();
    assert_eq!(api.env.len(), 2);
    assert_eq!(api.env[0].name, "GITHUB_TOKEN");
    assert!(api.env[0].required);
    assert_eq!(api.env[1].name, "API_VERSION");
    assert!(!api.env[1].required);
}

#[test]
fn test_env_yaml_substitution_in_headers() {
    let api = ycallr_core::yaml_parser::parse_yaml(ENV_YAML).unwrap();
    let cmd = api.commands.get("get-repo").unwrap();
    assert_eq!(
        cmd.headers.get("Authorization").unwrap(),
        "Bearer ${GITHUB_TOKEN}"
    );
}

#[test]
fn test_env_protobuf_roundtrip() {
    let api = ycallr_core::yaml_parser::parse_yaml(ENV_YAML).unwrap();
    let proto_bytes = api.to_proto_bytes().unwrap();
    let restored = ApiDefinition::from_proto_bytes(&proto_bytes).unwrap();

    assert_eq!(restored.env.len(), 2);
    assert_eq!(restored.env[0].name, "GITHUB_TOKEN");
    assert!(restored.env[0].required);

    let cmd = restored.commands.get("get-repo").unwrap();
    assert_eq!(
        cmd.headers.get("Authorization").unwrap(),
        "Bearer ${GITHUB_TOKEN}"
    );
}

const BODY_YAML: &str = r#"
name: body-api
version: "1.0.0"
description: API with body
base_url: https://api.test.com
commands:
  create-issue:
    endpoint: /repos/{owner}/{repo}/issues
    method: POST
    params:
      owner:
        description: Repository owner
        type: string
        required: true
      repo:
        description: Repository name
        type: string
        required: true
    body:
      json:
        owner_id: "{owner}"
        issue_title: "Issue in {repo}"
        labels:
          - bug
          - urgent
"#;

#[test]
fn test_body_yaml_parsing() {
    let api = ycallr_core::yaml_parser::parse_yaml(BODY_YAML).unwrap();
    let cmd = api.commands.get("create-issue").unwrap();
    let body = cmd.body.as_ref().unwrap();
    let json = body.json.as_ref().unwrap();

    assert!(json.is_object());
    assert_eq!(json["owner_id"], "{owner}");
    assert_eq!(json["issue_title"], "Issue in {repo}");
    assert_eq!(json["labels"][0], "bug");
    assert_eq!(json["labels"][1], "urgent");
}

#[test]
fn test_body_protobuf_roundtrip() {
    let api = ycallr_core::yaml_parser::parse_yaml(BODY_YAML).unwrap();
    let proto_bytes = api.to_proto_bytes().unwrap();
    let restored = ApiDefinition::from_proto_bytes(&proto_bytes).unwrap();

    let cmd = restored.commands.get("create-issue").unwrap();
    let body = cmd.body.as_ref().unwrap();
    let json = body.json.as_ref().unwrap();

    assert!(json.is_object());
    assert_eq!(json["owner_id"], "{owner}");
    assert_eq!(json["issue_title"], "Issue in {repo}");
    assert_eq!(json["labels"][0], "bug");
    assert_eq!(json["labels"][1], "urgent");
}

#[test]
fn test_body_template_resolution() {
    let api = ycallr_core::yaml_parser::parse_yaml(BODY_YAML).unwrap();
    let json = api
        .commands
        .get("create-issue")
        .unwrap()
        .body
        .as_ref()
        .unwrap()
        .json
        .as_ref()
        .unwrap()
        .clone();

    let mut params = HashMap::new();
    params.insert("owner".to_string(), "rust-lang".to_string());
    params.insert("repo".to_string(), "rust".to_string());

    let client = ycallr_core::YcallrClient::new(api).unwrap();
    let resolved = client.resolve_json_templates(&json, &params).unwrap();

    assert_eq!(resolved["owner_id"], "rust-lang");
    assert_eq!(resolved["issue_title"], "Issue in rust");
    assert_eq!(resolved["labels"][0], "bug");
    assert_eq!(resolved["labels"][1], "urgent");
}

#[test]
fn test_body_json_contains_template_vars() {
    let api = ycallr_core::yaml_parser::parse_yaml(BODY_YAML).unwrap();
    let json_str = serde_json::to_string(&api).unwrap();
    assert!(json_str.contains("{owner}"));
    assert!(json_str.contains("Issue in {repo}"));
}

#[test]
fn test_description_yaml_parsing() {
    let api = ycallr_core::yaml_parser::parse_yaml(DESCRIPTION_YAML).unwrap();
    let get_item = api.commands.get("get-item").unwrap();
    assert_eq!(
        get_item.description,
        Some("Retrieve an item by ID".to_string())
    );

    let create_item = api.commands.get("create-item").unwrap();
    assert_eq!(
        create_item.description,
        Some("Create a new item".to_string())
    );
}

#[test]
fn test_path_alias_yaml_parsing() {
    let api = ycallr_core::yaml_parser::parse_yaml(PATH_ALIAS_YAML).unwrap();
    let cmd = api.commands.get("get-item").unwrap();
    assert_eq!(cmd.endpoint.as_deref(), Some("/items/{id}"));
}

#[test]
fn test_description_protobuf_roundtrip() {
    let api = ycallr_core::yaml_parser::parse_yaml(DESCRIPTION_YAML).unwrap();
    let proto_bytes = api.to_proto_bytes().unwrap();
    let restored = ApiDefinition::from_proto_bytes(&proto_bytes).unwrap();

    let get_item = restored.commands.get("get-item").unwrap();
    assert_eq!(
        get_item.description,
        Some("Retrieve an item by ID".to_string())
    );
}
