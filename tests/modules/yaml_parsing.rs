use std::collections::HashMap;
use ycallr_core::HttpMethod;

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