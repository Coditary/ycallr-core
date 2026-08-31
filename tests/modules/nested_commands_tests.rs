use std::collections::HashMap;
use ycallr_core::{HttpMethod, YcallrError};

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

const DETAILS_YAML: &str = r#"
name: details-api
version: "1.0.0"
base_url: https://api.test.com
auth:
  primary:
    type: bearer
    token: ${TOKEN}
commands:
  create-item:
    description: Create an item
    endpoint: /items
    method: POST
    auth: primary
    headers:
      X-App: myapp
    params:
      title:
        description: Title
        type: string
        required: true
    body:
      json:
        title: "{title}"
    responses:
      success:
        message: "Created {output.id}"
      "409":
        message: "Conflict"
env:
  - name: TOKEN
    required: true
"#;

const USERS_GET_YAML: &str = r#"
name: github
version: "1.0.0"
base_url: PLACEHOLDER
commands:
  users:
    endpoint: /users
    method: GET
    commands:
      get:
        endpoint: /users/{username}
        method: GET
        params:
          username:
            description: Username
            type: string
            required: true
"#;

#[test]
fn test_command_details_not_found() {
    let api = ycallr_core::yaml_parser::parse_yaml(NESTED_YAML).unwrap();
    let err = api.command_details("repos.issues.missing").unwrap_err();
    assert!(matches!(err, YcallrError::CommandNotFound(_)));
    assert!(err.to_string().contains("missing"));
}

#[test]
fn test_list_subcommands_not_found() {
    let api = ycallr_core::yaml_parser::parse_yaml(NESTED_YAML).unwrap();
    assert!(api.list_subcommands("users.missing").is_err());
}

#[test]
fn test_get_command_past_leaf_segment() {
    let api = ycallr_core::yaml_parser::parse_yaml(NESTED_YAML).unwrap();
    let err = api.get_command("repos.issues.create.extra").unwrap_err();
    assert!(err.to_string().contains("extra has no sub-commands"));
}

#[test]
fn test_nested_yaml_command_details_metadata() {
    let api = ycallr_core::yaml_parser::parse_yaml(DETAILS_YAML).unwrap();
    let details = api.command_details("create-item").unwrap();

    assert_eq!(details.path, "create-item");
    assert_eq!(details.description.as_deref(), Some("Create an item"));
    assert_eq!(details.auth.as_deref(), Some("primary"));
    assert_eq!(
        details.headers.get("X-App").map(String::as_str),
        Some("myapp")
    );
    assert!(details.has_body);
    assert!(details.has_responses);
    assert_eq!(details.params.len(), 1);
    assert!(details.subcommands.is_empty());
    assert!(details.is_callable);
    assert!(!details.is_branch);
}

#[test]
fn test_users_get_details_from_nested_yaml() {
    let api = ycallr_core::yaml_parser::parse_yaml(NESTED_YAML).unwrap();
    let details = api.command_details("users.get").unwrap();

    assert_eq!(details.path, "users.get");
    assert_eq!(details.endpoint.as_deref(), Some("/users/{username}"));
    assert_eq!(details.method.as_ref(), Some(&HttpMethod::GET));
    assert!(details.is_leaf);
    assert!(!details.is_branch);
    assert!(details.subcommands.is_empty());
}

#[cfg(feature = "protobuf")]
#[test]
fn test_command_details_stable_after_proto_roundtrip() {
    let api = ycallr_core::yaml_parser::parse_yaml(NESTED_YAML).unwrap();
    let before = api.command_details("repos.issues.create").unwrap();

    let bytes = api.to_proto_bytes().unwrap();
    let restored = ycallr_core::ApiDefinition::from_proto_bytes(&bytes).unwrap();
    let after = restored.command_details("repos.issues.create").unwrap();

    assert_eq!(after.path, before.path);
    assert_eq!(after.endpoint, before.endpoint);
    assert_eq!(after.method, before.method);
    assert_eq!(after.is_callable, before.is_callable);
    assert_eq!(after.subcommands, before.subcommands);
}

#[cfg(all(not(target_arch = "wasm32"), feature = "client"))]
mod client_tests {
    use super::NESTED_YAML;
    use std::collections::HashMap;
    use ycallr_core::YcallrClient;

    #[test]
    fn test_nested_call_missing_required_params() {
        let api = ycallr_core::yaml_parser::parse_yaml(NESTED_YAML).unwrap();
        let client = YcallrClient::new(api).unwrap();

        let result = client.call("repos.issues", &HashMap::new(), None);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("path parameter"));
    }

    #[test]
    fn test_nested_call_command_not_found() {
        let api = ycallr_core::yaml_parser::parse_yaml(NESTED_YAML).unwrap();
        let client = YcallrClient::new(api).unwrap();

        let result = client.call("repos.issues.missing", &HashMap::new(), None);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("missing"));
    }

    #[test]
    fn test_nested_users_get_http() {
        let mut server = mockito::Server::new();

        let mock = server
            .mock("GET", "/users/octocat")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"login": "octocat"}"#)
            .create();

        let yaml = super::USERS_GET_YAML.replace("PLACEHOLDER", "https://api.test.com");
        let mut api = ycallr_core::yaml_parser::parse_yaml(&yaml).unwrap();
        api.base_url = server.url();
        let client = YcallrClient::new(api).unwrap();
        let params = HashMap::from([("username".to_string(), "octocat".to_string())]);
        let response = client.call("users.get", &params, None).unwrap();

        mock.assert();
        assert_eq!(response.status, 200);
        assert_eq!(response.body["login"], "octocat");
    }

    #[test]
    fn test_nested_hybrid_parent_does_not_inherit_child_params() {
        let api = ycallr_core::yaml_parser::parse_yaml(NESTED_YAML).unwrap();
        let client = YcallrClient::new(api).unwrap();

        let repos = client.command_details("repos").unwrap();
        let issues = client.command_details("repos.issues").unwrap();

        assert!(repos.params.is_empty());
        assert!(issues.params.is_empty());
    }
}
