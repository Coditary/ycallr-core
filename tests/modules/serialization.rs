use ycallr_core::{ApiDefinition, HttpMethod};

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
fn test_protobuf_roundtrip() {
    let api = ycallr_core::yaml_parser::parse_yaml(FULL_YAML).unwrap();

    let proto_bytes = api.to_proto_bytes().unwrap();
    assert!(!proto_bytes.is_empty());

    let restored = ApiDefinition::from_proto_bytes(&proto_bytes).unwrap();
    assert_eq!(api.name, restored.name);
    assert_eq!(api.commands.len(), restored.commands.len());
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
