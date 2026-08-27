use std::collections::HashMap;
use ycallr_core::ApiDefinition;

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

const FORM_BODY_YAML: &str = r#"
name: form-api
version: "1.0.0"
description: API with form body
base_url: https://api.test.com
commands:
  login:
    endpoint: /auth/login
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

const RAW_BODY_YAML: &str = r#"
name: raw-api
version: "1.0.0"
description: API with raw body
base_url: https://api.test.com
commands:
  send-xml:
    endpoint: /api/xml
    method: POST
    params:
      name:
        description: Name value
        type: string
        required: true
    body:
      raw: "<request><name>{name}</name></request>"
  send-text:
    endpoint: /api/text
    method: POST
    params:
      content:
        description: Text content
        type: string
        required: true
    body:
      raw: "Hello {content}, this is raw text"
"#;

const MULTIPART_BODY_YAML: &str = r#"
name: multipart-api
version: "1.0.0"
description: API with multipart body
base_url: https://api.test.com
commands:
  upload:
    endpoint: /upload
    method: POST
    params:
      desc:
        description: File description
        type: string
        required: true
    body:
      multipart:
        - name: description
          text: "{desc}"
        - name: attachment
          file: "/tmp/test.txt"
"#;

const EMPTY_BODY_YAML: &str = r#"
name: empty-api
version: "1.0.0"
base_url: https://api.test.com
commands:
  no-body:
    endpoint: /test
    method: GET
  json-only:
    endpoint: /json
    method: POST
    body:
      json:
        key: "value"
  form-only:
    endpoint: /form
    method: POST
    body:
      form:
        field: "data"
  raw-only:
    endpoint: /raw
    method: POST
    body:
      raw: "plain text"
  multipart-only:
    endpoint: /multipart
    method: POST
    body:
      multipart:
        - name: file
          text: "content"
"#;

const MIXED_BODY_YAML: &str = r#"
name: mixed-api
version: "1.0.0"
base_url: https://api.test.com
commands:
  complex:
    endpoint: /complex
    method: POST
    params:
      token:
        description: Auth token
        type: string
        required: true
    body:
      json:
        auth: "{token}"
        data:
          nested: true
      raw: "fallback {token}"
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
fn test_form_body_yaml_parsing() {
    let api = ycallr_core::yaml_parser::parse_yaml(FORM_BODY_YAML).unwrap();
    let cmd = api.commands.get("login").unwrap();
    let body = cmd.body.as_ref().unwrap();
    let form = body.form.as_ref().unwrap();

    assert_eq!(form.get("username").unwrap(), "{user}");
    assert_eq!(form.get("password").unwrap(), "secret123");
    assert_eq!(form.get("grant_type").unwrap(), "password");
    assert!(body.json.is_none());
    assert!(body.raw.is_none());
}

#[test]
fn test_form_body_protobuf_roundtrip() {
    let api = ycallr_core::yaml_parser::parse_yaml(FORM_BODY_YAML).unwrap();
    let proto_bytes = api.to_proto_bytes().unwrap();
    let restored = ApiDefinition::from_proto_bytes(&proto_bytes).unwrap();

    let cmd = restored.commands.get("login").unwrap();
    let body = cmd.body.as_ref().unwrap();
    let form = body.form.as_ref().unwrap();

    assert_eq!(form.get("username").unwrap(), "{user}");
    assert_eq!(form.get("password").unwrap(), "secret123");
    assert_eq!(form.get("grant_type").unwrap(), "password");
}

#[test]
fn test_form_body_template_resolution() {
    let api = ycallr_core::yaml_parser::parse_yaml(FORM_BODY_YAML).unwrap();
    let mut params = HashMap::new();
    params.insert("user".to_string(), "admin".to_string());

    let client = ycallr_core::YcallrClient::new(api).unwrap();
    let body_config = client
        .resolve_body(
            &client.api().commands.get("login").unwrap().body.as_ref().unwrap(),
            &params,
        )
        .unwrap();
    let form = body_config.form.as_ref().unwrap();

    assert_eq!(form.get("username").unwrap(), "admin");
    assert_eq!(form.get("password").unwrap(), "secret123");
}

#[test]
fn test_raw_body_yaml_parsing() {
    let api = ycallr_core::yaml_parser::parse_yaml(RAW_BODY_YAML).unwrap();

    let send_xml = api.commands.get("send-xml").unwrap();
    let body = send_xml.body.as_ref().unwrap();
    assert_eq!(body.raw.as_ref().unwrap(), "<request><name>{name}</name></request>");
    assert!(body.json.is_none());
    assert!(body.form.is_none());

    let send_text = api.commands.get("send-text").unwrap();
    let body = send_text.body.as_ref().unwrap();
    assert_eq!(body.raw.as_ref().unwrap(), "Hello {content}, this is raw text");
}

#[test]
fn test_raw_body_protobuf_roundtrip() {
    let api = ycallr_core::yaml_parser::parse_yaml(RAW_BODY_YAML).unwrap();
    let proto_bytes = api.to_proto_bytes().unwrap();
    let restored = ApiDefinition::from_proto_bytes(&proto_bytes).unwrap();

    let send_xml = restored.commands.get("send-xml").unwrap();
    let body = send_xml.body.as_ref().unwrap();
    assert_eq!(body.raw.as_ref().unwrap(), "<request><name>{name}</name></request>");

    let send_text = restored.commands.get("send-text").unwrap();
    let body = send_text.body.as_ref().unwrap();
    assert_eq!(body.raw.as_ref().unwrap(), "Hello {content}, this is raw text");
}

#[test]
fn test_raw_body_template_resolution() {
    let api = ycallr_core::yaml_parser::parse_yaml(RAW_BODY_YAML).unwrap();
    let mut params = HashMap::new();
    params.insert("name".to_string(), "test-user".to_string());

    let client = ycallr_core::YcallrClient::new(api).unwrap();
    let body_config = client
        .resolve_body(
            &client.api().commands.get("send-xml").unwrap().body.as_ref().unwrap(),
            &params,
        )
        .unwrap();

    assert_eq!(body_config.raw.as_ref().unwrap(), "<request><name>test-user</name></request>");
}

#[test]
fn test_multipart_body_yaml_parsing() {
    let api = ycallr_core::yaml_parser::parse_yaml(MULTIPART_BODY_YAML).unwrap();
    let cmd = api.commands.get("upload").unwrap();
    let body = cmd.body.as_ref().unwrap();
    let multipart = body.multipart.as_ref().unwrap();

    assert_eq!(multipart.len(), 2);
    assert_eq!(multipart[0].name, "description");
    assert_eq!(multipart[0].text.as_ref().unwrap(), "{desc}");
    assert!(multipart[0].file.is_none());
    assert_eq!(multipart[1].name, "attachment");
    assert_eq!(multipart[1].file.as_ref().unwrap(), "/tmp/test.txt");
    assert!(multipart[1].text.is_none());
}

#[test]
fn test_multipart_body_protobuf_roundtrip() {
    let api = ycallr_core::yaml_parser::parse_yaml(MULTIPART_BODY_YAML).unwrap();
    let proto_bytes = api.to_proto_bytes().unwrap();
    let restored = ApiDefinition::from_proto_bytes(&proto_bytes).unwrap();

    let cmd = restored.commands.get("upload").unwrap();
    let body = cmd.body.as_ref().unwrap();
    let multipart = body.multipart.as_ref().unwrap();

    assert_eq!(multipart.len(), 2);
    assert_eq!(multipart[0].name, "description");
    assert_eq!(multipart[0].text.as_ref().unwrap(), "{desc}");
    assert_eq!(multipart[1].name, "attachment");
    assert_eq!(multipart[1].file.as_ref().unwrap(), "/tmp/test.txt");
}

#[test]
fn test_multipart_body_template_resolution() {
    let api = ycallr_core::yaml_parser::parse_yaml(MULTIPART_BODY_YAML).unwrap();
    let mut params = HashMap::new();
    params.insert("desc".to_string(), "my file".to_string());

    let client = ycallr_core::YcallrClient::new(api).unwrap();
    let body_config = client
        .resolve_body(
            &client.api().commands.get("upload").unwrap().body.as_ref().unwrap(),
            &params,
        )
        .unwrap();
    let multipart = body_config.multipart.as_ref().unwrap();

    assert_eq!(multipart[0].text.as_ref().unwrap(), "my file");
    assert_eq!(multipart[1].file.as_ref().unwrap(), "/tmp/test.txt");
}

#[test]
fn test_body_type_detection() {
    let api = ycallr_core::yaml_parser::parse_yaml(EMPTY_BODY_YAML).unwrap();

    assert!(api.commands.get("no-body").unwrap().body.is_none());

    let json_cmd = api.commands.get("json-only").unwrap().body.as_ref().unwrap();
    assert!(json_cmd.json.is_some());
    assert!(json_cmd.form.is_none());
    assert!(json_cmd.raw.is_none());
    assert!(json_cmd.multipart.is_none());

    let form_cmd = api.commands.get("form-only").unwrap().body.as_ref().unwrap();
    assert!(form_cmd.json.is_none());
    assert!(form_cmd.form.is_some());
    assert!(form_cmd.raw.is_none());
    assert!(form_cmd.multipart.is_none());

    let raw_cmd = api.commands.get("raw-only").unwrap().body.as_ref().unwrap();
    assert!(raw_cmd.json.is_none());
    assert!(raw_cmd.form.is_none());
    assert!(raw_cmd.raw.is_some());
    assert!(raw_cmd.multipart.is_none());

    let mp_cmd = api.commands.get("multipart-only").unwrap().body.as_ref().unwrap();
    assert!(mp_cmd.json.is_none());
    assert!(mp_cmd.form.is_none());
    assert!(mp_cmd.raw.is_none());
    assert!(mp_cmd.multipart.is_some());
}

#[test]
fn test_multiple_body_types_in_one_config() {
    let api = ycallr_core::yaml_parser::parse_yaml(MIXED_BODY_YAML).unwrap();
    let body = api.commands.get("complex").unwrap().body.clone().unwrap();

    assert!(body.json.is_some());
    assert!(body.raw.is_some());

    let mut params = HashMap::new();
    params.insert("token".to_string(), "abc123".to_string());

    let client = ycallr_core::YcallrClient::new(api).unwrap();
    let resolved = client
        .resolve_body(&body, &params)
        .unwrap();

    assert_eq!(resolved.json.as_ref().unwrap()["auth"], "abc123");
    assert_eq!(resolved.raw.as_ref().unwrap(), "fallback abc123");
}

#[test]
fn test_body_protobuf_roundtrip_preserves_all_types() {
    let api = ycallr_core::yaml_parser::parse_yaml(MIXED_BODY_YAML).unwrap();
    let proto_bytes = api.to_proto_bytes().unwrap();
    let restored = ApiDefinition::from_proto_bytes(&proto_bytes).unwrap();

    let body = restored.commands.get("complex").unwrap().body.as_ref().unwrap();
    assert!(body.json.is_some());
    assert_eq!(body.json.as_ref().unwrap()["auth"], "{token}");
    assert!(body.raw.is_some());
    assert_eq!(body.raw.as_ref().unwrap(), "fallback {token}");
}

#[test]
fn test_empty_body_protobuf_roundtrip() {
    let api = ycallr_core::yaml_parser::parse_yaml(EMPTY_BODY_YAML).unwrap();
    let proto_bytes = api.to_proto_bytes().unwrap();
    let restored = ApiDefinition::from_proto_bytes(&proto_bytes).unwrap();

    assert!(restored.commands.get("no-body").unwrap().body.is_none());

    let json_cmd = restored.commands.get("json-only").unwrap().body.as_ref().unwrap();
    assert!(json_cmd.json.is_some());
    assert!(json_cmd.form.is_none());

    let form_cmd = restored.commands.get("form-only").unwrap().body.as_ref().unwrap();
    assert!(form_cmd.form.is_some());
    assert!(form_cmd.json.is_none());

    let raw_cmd = restored.commands.get("raw-only").unwrap().body.as_ref().unwrap();
    assert!(raw_cmd.raw.is_some());

    let mp_cmd = restored.commands.get("multipart-only").unwrap().body.as_ref().unwrap();
    assert!(mp_cmd.multipart.is_some());
}