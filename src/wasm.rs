use wasm_bindgen::prelude::*;
use crate::models::ApiDefinition;

#[wasm_bindgen]
pub struct YcallrApi {
    inner: ApiDefinition,
}

#[wasm_bindgen]
impl YcallrApi {
    #[wasm_bindgen(constructor)]
    pub fn new(yaml: &str) -> std::result::Result<YcallrApi, JsValue> {
        let api = crate::yaml_parser::parse_yaml(yaml)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
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
        serde_json::to_string_pretty(&self.inner)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen(js_name = toProto)]
    pub fn to_proto(&self) -> std::result::Result<Vec<u8>, JsValue> {
        crate::compiler::Compiler::yaml_to_proto(&self.inner)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen(js_name = commandExists)]
    pub fn command_exists(&self, name: &str) -> bool {
        self.inner.commands.contains_key(name)
    }
}
