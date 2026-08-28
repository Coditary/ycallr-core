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

const NAMED_AUTH_YAML: &str = r#"
name: github
version: "1.0.0"
description: GitHub API with named auth
base_url: https://api.github.com
auth:
  primary:
    type: bearer
    token: ${GITHUB_TOKEN}
  secondary:
    type: api_key
    key: ${API_KEY}
    name: X-API-Key
    in: header
  query_key:
    type: api_key
    key: ${API_KEY}
    name: apikey
    in: query
  cookie_key:
    type: api_key
    key: ${API_KEY}
    name: session
    in: cookie
  basic:
    type: http
    scheme: basic
    username: ${USER}
    password: ${PASS}
  custom_http:
    type: http
    scheme: custom
    prefix: "Token "
    token: ${TOKEN}
env:
  - name: GITHUB_TOKEN
    required: true
  - name: API_KEY
    required: true
  - name: USER
    required: true
  - name: PASS
    required: true
  - name: TOKEN
    required: true
commands:
  get-repo:
    endpoint: /repos/{owner}/{repo}
    method: GET
    auth: primary
    params:
      owner:
        description: Repository owner
        type: string
        required: true
      repo:
        description: Repository name
        type: string
        required: true
  search-repos:
    endpoint: /search/repositories
    method: GET
    auth: secondary
    params:
      query:
        description: Search query
        type: string
        required: true
  query-auth:
    endpoint: /api/data
    method: GET
    auth: query_key
    params: {}
  cookie-auth:
    endpoint: /api/data
    method: GET
    auth: cookie_key
    params: {}
  basic-auth:
    endpoint: /api/data
    method: GET
    auth: basic
    params: {}
  custom-auth:
    endpoint: /api/data
    method: GET
    auth: custom_http
    params: {}
  public-endpoint:
    endpoint: /status
    method: GET
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

#[test]
fn test_named_auth_yaml_parsing() {
    let api = ycallr_core::yaml_parser::parse_yaml(NAMED_AUTH_YAML).unwrap();
    assert_eq!(api.auth.len(), 6);
    assert!(api.auth.contains_key("primary"));
    assert!(api.auth.contains_key("secondary"));
    assert!(api.auth.contains_key("query_key"));
    assert!(api.auth.contains_key("cookie_key"));
    assert!(api.auth.contains_key("basic"));
    assert!(api.auth.contains_key("custom_http"));
}

#[test]
fn test_named_auth_bearer_config() {
    let api = ycallr_core::yaml_parser::parse_yaml(NAMED_AUTH_YAML).unwrap();
    let primary = api.auth.get("primary").unwrap();
    match primary {
        ycallr_core::AuthConfig::Bearer { token } => {
            assert_eq!(token, "${GITHUB_TOKEN}");
        }
        _ => panic!("Expected Bearer auth"),
    }
}

#[test]
fn test_named_auth_api_key_header() {
    let api = ycallr_core::yaml_parser::parse_yaml(NAMED_AUTH_YAML).unwrap();
    let secondary = api.auth.get("secondary").unwrap();
    match secondary {
        ycallr_core::AuthConfig::ApiKey { key, name, in_ } => {
            assert_eq!(key, "${API_KEY}");
            assert_eq!(name, "X-API-Key");
            assert_eq!(in_, &ycallr_core::models::ApiKeyLocation::Header);
        }
        _ => panic!("Expected ApiKey auth"),
    }
}

#[test]
fn test_named_auth_api_key_query() {
    let api = ycallr_core::yaml_parser::parse_yaml(NAMED_AUTH_YAML).unwrap();
    let query_key = api.auth.get("query_key").unwrap();
    match query_key {
        ycallr_core::AuthConfig::ApiKey { key, name, in_ } => {
            assert_eq!(key, "${API_KEY}");
            assert_eq!(name, "apikey");
            assert_eq!(in_, &ycallr_core::models::ApiKeyLocation::Query);
        }
        _ => panic!("Expected ApiKey auth"),
    }
}

#[test]
fn test_named_auth_api_key_cookie() {
    let api = ycallr_core::yaml_parser::parse_yaml(NAMED_AUTH_YAML).unwrap();
    let cookie_key = api.auth.get("cookie_key").unwrap();
    match cookie_key {
        ycallr_core::AuthConfig::ApiKey { key, name, in_ } => {
            assert_eq!(key, "${API_KEY}");
            assert_eq!(name, "session");
            assert_eq!(in_, &ycallr_core::models::ApiKeyLocation::Cookie);
        }
        _ => panic!("Expected ApiKey auth"),
    }
}

#[test]
fn test_named_auth_http_basic() {
    let api = ycallr_core::yaml_parser::parse_yaml(NAMED_AUTH_YAML).unwrap();
    let basic = api.auth.get("basic").unwrap();
    match basic {
        ycallr_core::AuthConfig::Http {
            scheme,
            username,
            password,
            ..
        } => {
            assert_eq!(scheme, "basic");
            assert_eq!(username.as_deref(), Some("${USER}"));
            assert_eq!(password.as_deref(), Some("${PASS}"));
        }
        _ => panic!("Expected Http auth"),
    }
}

#[test]
fn test_named_auth_http_custom() {
    let api = ycallr_core::yaml_parser::parse_yaml(NAMED_AUTH_YAML).unwrap();
    let custom = api.auth.get("custom_http").unwrap();
    match custom {
        ycallr_core::AuthConfig::Http {
            scheme,
            prefix,
            token,
            ..
        } => {
            assert_eq!(scheme, "custom");
            assert_eq!(prefix.as_deref(), Some("Token "));
            assert_eq!(token.as_deref(), Some("${TOKEN}"));
        }
        _ => panic!("Expected Http auth"),
    }
}

#[test]
fn test_command_auth_reference() {
    let api = ycallr_core::yaml_parser::parse_yaml(NAMED_AUTH_YAML).unwrap();

    let get_repo = api.commands.get("get-repo").unwrap();
    assert_eq!(get_repo.auth.as_deref(), Some("primary"));

    let search = api.commands.get("search-repos").unwrap();
    assert_eq!(search.auth.as_deref(), Some("secondary"));

    let query = api.commands.get("query-auth").unwrap();
    assert_eq!(query.auth.as_deref(), Some("query_key"));

    let cookie = api.commands.get("cookie-auth").unwrap();
    assert_eq!(cookie.auth.as_deref(), Some("cookie_key"));

    let basic = api.commands.get("basic-auth").unwrap();
    assert_eq!(basic.auth.as_deref(), Some("basic"));

    let custom = api.commands.get("custom-auth").unwrap();
    assert_eq!(custom.auth.as_deref(), Some("custom_http"));

    let public = api.commands.get("public-endpoint").unwrap();
    assert!(public.auth.is_none());
}