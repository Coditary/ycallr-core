use crate::models::ApiDefinition;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct YcallrApi {
    inner: ApiDefinition,
}

#[wasm_bindgen]
impl YcallrApi {
    #[wasm_bindgen(constructor)]
    pub fn new(yaml: &str) -> std::result::Result<YcallrApi, JsValue> {
        let api =
            crate::yaml_parser::parse_yaml(yaml).map_err(|e| JsValue::from_str(&e.to_string()))?;
        Ok(YcallrApi { inner: api })
    }

    #[wasm_bindgen(getter)]
    pub fn name(&self) -> String {
        self.inner.name.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn version(&self) -> String {
        self.inner.version.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn base_url(&self) -> String {
        self.inner.base_url.clone()
    }

    #[wasm_bindgen(js_name = envCount)]
    pub fn env_count(&self) -> usize {
        self.inner.env.len()
    }

    #[wasm_bindgen(js_name = envName)]
    pub fn env_name(&self, index: usize) -> Option<String> {
        self.inner.env.get(index).map(|e| e.name.clone())
    }

    #[wasm_bindgen(js_name = envRequired)]
    pub fn env_required(&self, index: usize) -> Option<bool> {
        self.inner.env.get(index).map(|e| e.required)
    }

    #[wasm_bindgen(js_name = commandHasResponses)]
    pub fn command_has_responses(&self, name: &str) -> bool {
        self.inner
            .get_command(name)
            .ok()
            .and_then(|cmd| cmd.responses.as_ref())
            .is_some()
    }

    #[wasm_bindgen(js_name = commandSuccessMessage)]
    pub fn command_success_message(&self, name: &str) -> Option<String> {
        self.inner
            .get_command(name)
            .ok()
            .and_then(|cmd| cmd.responses.as_ref())
            .and_then(|r| r.success.as_ref())
            .map(|e| e.message.clone())
    }

    #[wasm_bindgen(js_name = commandFailureMessage)]
    pub fn command_failure_message(&self, name: &str) -> Option<String> {
        self.inner
            .get_command(name)
            .ok()
            .and_then(|cmd| cmd.responses.as_ref())
            .and_then(|r| r.failure.as_ref())
            .map(|e| e.message.clone())
    }

    #[wasm_bindgen(js_name = commandHasBody)]
    pub fn command_has_body(&self, name: &str) -> bool {
        self.inner
            .get_command(name)
            .ok()
            .and_then(|cmd| cmd.body.as_ref())
            .is_some()
    }

    #[wasm_bindgen(js_name = commandBodyJson)]
    pub fn command_body_json(&self, name: &str) -> Option<String> {
        self.inner
            .get_command(name)
            .ok()
            .and_then(|cmd| cmd.body.as_ref())
            .and_then(|b| b.json.as_ref())
            .map(|v| v.to_string())
    }

    pub fn to_json(&self) -> std::result::Result<String, JsValue> {
        serde_json::to_string_pretty(&self.inner).map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen(js_name = toProto)]
    pub fn to_proto(&self) -> std::result::Result<Vec<u8>, JsValue> {
        crate::compiler::Compiler::yaml_to_proto(&self.inner)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen(js_name = commandExists)]
    pub fn command_exists(&self, name: &str) -> bool {
        self.inner.get_command(name).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    const VALID_YAML: &str = r#"
name: test-api
version: "1.0.0"
description: Test API for WASM
base_url: https://api.test.com
commands:
  get-item:
    endpoint: /items/{id}
    method: GET
    params:
      id:
        description: Item ID
        type: string
        required: true
  create-item:
    endpoint: /items
    method: POST
    params:
      name:
        description: Item name
        type: string
        required: true
"#;

    const NESTED_YAML: &str = r#"
name: nested-api
version: "1.0.0"
description: Nested API for WASM
base_url: https://api.test.com
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
"#;

    const ENV_YAML: &str = r#"
name: env-api
version: "1.0.0"
description: API with env vars
base_url: https://api.test.com
env:
  - name: GITHUB_TOKEN
    required: true
  - name: GITHUB_URL
    required: false
commands:
  get-item:
    endpoint: /items/{id}
    method: GET
    headers:
      Authorization: "Bearer ${GITHUB_TOKEN}"
    params:
      id:
        description: Item ID
        type: string
        required: true
"#;

    const RESPONSE_YAML: &str = r#"
name: response-api
version: "1.0.0"
description: API with responses
base_url: https://api.test.com
commands:
  create-item:
    endpoint: /items
    method: POST
    responses:
      success:
        message: "Created {output.name}"
      failure:
        message: "Failed to create item"
      404:
        message: "{input.owner} not found"
"#;

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
        issue_title: "{title}"
"#;

    #[wasm_bindgen_test]
    fn test_wasm_new() {
        let api = YcallrApi::new(VALID_YAML).unwrap();
        assert_eq!(api.name(), "test-api");
    }

    #[wasm_bindgen_test]
    fn test_wasm_invalid_yaml() {
        let result = YcallrApi::new("not valid yaml {{{");
        assert!(result.is_err());
    }

    #[wasm_bindgen_test]
    fn test_wasm_empty_yaml() {
        let result = YcallrApi::new("");
        assert!(result.is_err());
    }

    #[wasm_bindgen_test]
    fn test_wasm_getters() {
        let api = YcallrApi::new(VALID_YAML).unwrap();
        assert_eq!(api.name(), "test-api");
        assert_eq!(api.version(), "1.0.0");
        assert_eq!(api.base_url(), "https://api.test.com");
    }

    #[wasm_bindgen_test]
    fn test_wasm_to_json() {
        let api = YcallrApi::new(VALID_YAML).unwrap();
        let json = api.to_json().unwrap();
        assert!(json.contains("test-api"));
        assert!(json.contains("https://api.test.com"));
    }

    #[wasm_bindgen_test]
    fn test_wasm_to_proto() {
        let api = YcallrApi::new(VALID_YAML).unwrap();
        let proto = api.to_proto().unwrap();
        assert!(!proto.is_empty());
    }

    #[wasm_bindgen_test]
    fn test_wasm_command_exists() {
        let api = YcallrApi::new(VALID_YAML).unwrap();
        assert!(api.command_exists("get-item"));
        assert!(api.command_exists("create-item"));
        assert!(!api.command_exists("nonexistent"));
    }

    #[wasm_bindgen_test]
    fn test_wasm_nested_command_exists() {
        let api = YcallrApi::new(NESTED_YAML).unwrap();
        assert!(api.command_exists("repos"));
        assert!(api.command_exists("repos.issues"));
        assert!(api.command_exists("repos.issues.create"));
        assert!(api.command_exists("repos.issues.list"));
        assert!(!api.command_exists("repos.issues.nonexistent"));
        assert!(!api.command_exists("repos.nonexistent"));
    }

    #[wasm_bindgen_test]
    fn test_wasm_nested_yaml_to_proto_roundtrip() {
        let api = YcallrApi::new(NESTED_YAML).unwrap();
        let proto = api.to_proto().unwrap();
        assert!(!proto.is_empty());
        let json = api.to_json().unwrap();
        assert!(json.contains("repos"));
        assert!(json.contains("issues"));
        assert!(json.contains("create"));
    }

    #[wasm_bindgen_test]
    fn test_wasm_env_count() {
        let api = YcallrApi::new(ENV_YAML).unwrap();
        assert_eq!(api.env_count(), 2);
    }

    #[wasm_bindgen_test]
    fn test_wasm_env_name() {
        let api = YcallrApi::new(ENV_YAML).unwrap();
        assert_eq!(api.env_name(0), Some("GITHUB_TOKEN".to_string()));
        assert_eq!(api.env_name(1), Some("GITHUB_URL".to_string()));
        assert_eq!(api.env_name(99), None);
    }

    #[wasm_bindgen_test]
    fn test_wasm_env_required() {
        let api = YcallrApi::new(ENV_YAML).unwrap();
        assert_eq!(api.env_required(0), Some(true));
        assert_eq!(api.env_required(1), Some(false));
        assert_eq!(api.env_required(99), None);
    }

    #[wasm_bindgen_test]
    fn test_wasm_env_yaml_contains_substitution() {
        let api = YcallrApi::new(ENV_YAML).unwrap();
        let json = api.to_json().unwrap();
        assert!(json.contains("${GITHUB_TOKEN}"));
    }

    #[wasm_bindgen_test]
    fn test_wasm_command_has_responses() {
        let api = YcallrApi::new(RESPONSE_YAML).unwrap();
        assert!(api.command_has_responses("create-item"));
        assert!(!api.command_has_responses("nonexistent"));
    }

    #[wasm_bindgen_test]
    fn test_wasm_command_success_message() {
        let api = YcallrApi::new(RESPONSE_YAML).unwrap();
        assert_eq!(
            api.command_success_message("create-item"),
            Some("Created {output.name}".to_string())
        );
    }

    #[wasm_bindgen_test]
    fn test_wasm_command_failure_message() {
        let api = YcallrApi::new(RESPONSE_YAML).unwrap();
        assert_eq!(
            api.command_failure_message("create-item"),
            Some("Failed to create item".to_string())
        );
    }

    #[wasm_bindgen_test]
    fn test_wasm_response_yaml_in_json() {
        let api = YcallrApi::new(RESPONSE_YAML).unwrap();
        let json = api.to_json().unwrap();
        assert!(json.contains("Created {output.name}"));
        assert!(json.contains("Failed to create item"));
    }

    #[wasm_bindgen_test]
    fn test_wasm_command_has_body() {
        let api = YcallrApi::new(BODY_YAML).unwrap();
        assert!(api.command_has_body("create-issue"));
        assert!(!api.command_has_body("nonexistent"));
    }

    #[wasm_bindgen_test]
    fn test_wasm_command_body_json() {
        let api = YcallrApi::new(BODY_YAML).unwrap();
        let body = api.command_body_json("create-issue").unwrap();
        assert!(body.contains("owner_id"));
        assert!(body.contains("{owner}"));
        assert!(body.contains("issue_title"));
        assert!(body.contains("{title}"));
    }

    #[wasm_bindgen_test]
    fn test_wasm_body_yaml_in_json() {
        let api = YcallrApi::new(BODY_YAML).unwrap();
        let json = api.to_json().unwrap();
        assert!(json.contains("owner_id"));
        assert!(json.contains("{owner}"));
    }
}
