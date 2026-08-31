use std::collections::HashMap;
use ycallr_core::{ApiDefinition, HttpMethod, ParamType};

#[test]
fn test_api_validation() {
    let valid_api = ApiDefinition {
        name: "test-api".to_string(),
        version: "1.0.0".to_string(),
        description: "Test".to_string(),
        base_url: "https://api.test.com".to_string(),
        env: vec![],
        auth: HashMap::new(),
        commands: HashMap::new(),
    };
    assert!(valid_api.validate().is_ok());

    let invalid_api = ApiDefinition {
        name: "".to_string(),
        version: "1.0.0".to_string(),
        description: "Test".to_string(),
        base_url: "https://api.test.com".to_string(),
        env: vec![],
        auth: HashMap::new(),
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
fn test_validate_params_rejects_wrong_types_on_parsed_yaml() {
    let yaml = r#"
name: test
version: "1.0.0"
base_url: https://api.test.com
commands:
  test:
    endpoint: /test
    method: POST
    params:
      num_param:
        description: Number param
        type: number
        required: true
"#;

    let api = ycallr_core::yaml_parser::parse_yaml(yaml).unwrap();
    let cmd = api.commands.get("test").unwrap();

    let result = cmd.validate_params(
        &HashMap::from([("num_param".to_string(), "not-a-number".to_string())]),
        None,
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("must be a number"));
}

#[test]
fn test_validate_rejects_loopback_base_url() {
    let api = ApiDefinition {
        name: "test-api".to_string(),
        version: "1.0.0".to_string(),
        description: "Test".to_string(),
        base_url: "http://127.0.0.1".to_string(),
        env: vec![],
        auth: HashMap::new(),
        commands: HashMap::new(),
    };
    assert!(api.validate().is_err());
}

#[test]
fn test_validate_rejects_non_http_scheme() {
    let api = ApiDefinition {
        name: "test-api".to_string(),
        version: "1.0.0".to_string(),
        description: "Test".to_string(),
        base_url: "ftp://api.test.com".to_string(),
        env: vec![],
        auth: HashMap::new(),
        commands: HashMap::new(),
    };
    assert!(api.validate().is_err());
}
