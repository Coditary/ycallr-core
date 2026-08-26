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
}
