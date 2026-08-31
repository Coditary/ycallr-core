use std::collections::HashMap;

use crate::call_engine::{
    build_api_response, prepare_http_request, resolve_client_env, ClientContext, EnvMode,
    PreparedBody, PreparedHttpRequest,
};
use crate::models::{ApiDefinition, ApiKeyLocation, AuthConfig};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{future_to_promise, JsFuture};

#[wasm_bindgen]
pub struct YcallrApi {
    inner: ApiDefinition,
}

/// Compile YAML to protobuf bytes (install step). Use [`YcallrApi::from_proto`] at runtime.
#[wasm_bindgen(js_name = compileYaml)]
pub fn compile_yaml_profile(yaml: &str) -> std::result::Result<Vec<u8>, JsValue> {
    crate::profile_store::compile_yaml_str(yaml).map_err(|e| JsValue::from_str(&e.to_string()))
}

#[wasm_bindgen]
impl YcallrApi {
    /// Load an API from compiled protobuf bytes (primary runtime constructor).
    #[wasm_bindgen(js_name = fromProto)]
    pub fn from_proto(bytes: &[u8]) -> std::result::Result<YcallrApi, JsValue> {
        let api = crate::profile_store::load_from_proto_bytes(bytes)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        Ok(YcallrApi { inner: api })
    }

    /// Compile and load from YAML (convenience; prefer `compileYaml` + `fromProto` in production).
    #[wasm_bindgen(js_name = fromYaml)]
    pub fn from_yaml(yaml: &str) -> std::result::Result<YcallrApi, JsValue> {
        let bytes = compile_yaml_profile(yaml)?;
        YcallrApi::from_proto(&bytes)
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[wasm_bindgen(js_name = installProfile)]
    pub fn install_profile(name: &str) -> std::result::Result<(), JsValue> {
        crate::profile_store::install_profile(name)
            .map(|_| ())
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[wasm_bindgen(js_name = loadInstalled)]
    pub fn load_installed(name: &str) -> std::result::Result<YcallrApi, JsValue> {
        crate::profile_store::load_installed_profile(name)
            .map(|inner| YcallrApi { inner })
            .map_err(|e| JsValue::from_str(&e.to_string()))
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

    #[wasm_bindgen(js_name = commandBodyForm)]
    pub fn command_body_form(&self, name: &str) -> Option<String> {
        self.inner
            .get_command(name)
            .ok()
            .and_then(|cmd| cmd.body.as_ref())
            .and_then(|b| b.form.as_ref())
            .map(|m| serde_json::to_string(m).unwrap_or_default())
    }

    #[wasm_bindgen(js_name = commandBodyRaw)]
    pub fn command_body_raw(&self, name: &str) -> Option<String> {
        self.inner
            .get_command(name)
            .ok()
            .and_then(|cmd| cmd.body.as_ref())
            .and_then(|b| b.raw.as_ref())
            .cloned()
    }

    #[wasm_bindgen(js_name = commandBodyMultipart)]
    pub fn command_body_multipart(&self, name: &str) -> Option<String> {
        self.inner
            .get_command(name)
            .ok()
            .and_then(|cmd| cmd.body.as_ref())
            .and_then(|b| b.multipart.as_ref())
            .map(|m| serde_json::to_string(m).unwrap_or_default())
    }

    #[wasm_bindgen(js_name = commandBodyType)]
    pub fn command_body_type(&self, name: &str) -> Option<String> {
        self.inner
            .get_command(name)
            .ok()
            .and_then(|cmd| cmd.body.as_ref())
            .and_then(|b| b.active_body_kind().map(|k| k.to_string()))
    }

    pub fn to_json(&self) -> std::result::Result<String, JsValue> {
        serde_json::to_string_pretty(&self.inner).map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen(js_name = toProto)]
    pub fn to_proto(&self) -> std::result::Result<Vec<u8>, JsValue> {
        crate::compiler::Compiler::yaml_to_proto(&self.inner)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen(getter)]
    pub fn description(&self) -> String {
        self.inner.description.clone()
    }

    #[wasm_bindgen(js_name = commandEndpoint)]
    pub fn command_endpoint(&self, name: &str) -> Option<String> {
        self.inner
            .get_command(name)
            .ok()
            .and_then(|cmd| cmd.endpoint.clone())
    }

    #[wasm_bindgen(js_name = commandMethod)]
    pub fn command_method(&self, name: &str) -> Option<String> {
        self.inner
            .get_command(name)
            .ok()
            .and_then(|cmd| cmd.method.as_ref().map(|m| m.as_str().to_string()))
    }

    #[wasm_bindgen(js_name = commandDescription)]
    pub fn command_description(&self, name: &str) -> Option<String> {
        self.inner
            .get_command(name)
            .ok()
            .and_then(|cmd| cmd.description.clone())
    }

    #[wasm_bindgen(js_name = listCommands)]
    pub fn list_commands(&self) -> String {
        let names: Vec<&str> = self.inner.commands.keys().map(|s| s.as_str()).collect();
        serde_json::to_string(&names).unwrap_or_else(|_| "[]".to_string())
    }

    #[wasm_bindgen(js_name = listSubcommands)]
    pub fn list_subcommands(&self, path: &str) -> std::result::Result<String, JsValue> {
        self.inner
            .list_subcommands(path)
            .map(|names| serde_json::to_string(&names).unwrap_or_else(|_| "[]".to_string()))
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen(js_name = commandExists)]
    pub fn command_exists(&self, name: &str) -> bool {
        self.inner.get_command(name).is_ok()
    }
}

#[wasm_bindgen]
pub struct YcallrWasmClient {
    context: ClientContext,
}

#[wasm_bindgen]
impl YcallrWasmClient {
    #[wasm_bindgen(js_name = createClient)]
    pub fn create_client(
        api: &YcallrApi,
        env_json: Option<String>,
        auth_type: Option<String>,
        auth_data_json: Option<String>,
    ) -> std::result::Result<YcallrWasmClient, JsValue> {
        api.inner
            .validate_for_client()
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        let env_vars: HashMap<String, String> = if let Some(json) = env_json {
            serde_json::from_str(&json).map_err(|e| JsValue::from_str(&e.to_string()))?
        } else {
            HashMap::new()
        };

        let auth = parse_wasm_auth(auth_type, auth_data_json)?;

        let resolved_env = resolve_client_env(&api.inner, &EnvMode::Manual, &env_vars)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        Ok(YcallrWasmClient {
            context: ClientContext {
                api: api.inner.clone(),
                auth,
                auth_configs: api.inner.auth.clone(),
                env_vars: resolved_env,
            },
        })
    }

    pub fn call(
        &self,
        command: &str,
        params_json: &str,
        body_json: Option<String>,
    ) -> js_sys::Promise {
        let ctx = self.context.clone();
        let command = command.to_string();
        let params_json = params_json.to_string();
        let body_json = body_json;

        future_to_promise(async move {
            let params: HashMap<String, String> = serde_json::from_str(&params_json)
                .map_err(|e| JsValue::from_str(&e.to_string()))?;

            let body = if let Some(json) = body_json {
                Some(serde_json::from_str(&json).map_err(|e| JsValue::from_str(&e.to_string()))?)
            } else {
                None
            };

            let prepared = prepare_http_request(&ctx, &command, &params, body.as_ref())
                .map_err(|e| JsValue::from_str(&e.to_string()))?;

            let fetch_result = execute_fetch(&prepared)
                .await
                .map_err(|e| JsValue::from_str(&e))?;

            let api_response = build_api_response(
                fetch_result.status,
                fetch_result.headers,
                fetch_result.body_text,
                prepared.responses.as_ref(),
                ctx.api.errors.as_ref(),
                &prepared.params,
            );

            serde_json::to_string(&api_response)
                .map_err(|e| JsValue::from_str(&e.to_string()))
                .map(|s| JsValue::from_str(&s))
        })
    }
}

struct FetchResult {
    status: u16,
    headers: HashMap<String, String>,
    body_text: String,
}

async fn execute_fetch(prepared: &PreparedHttpRequest) -> Result<FetchResult, String> {
    use web_sys::{FormData, Headers, Request, RequestInit, RequestMode, Response};

    let window = web_sys::window().ok_or_else(|| "No global window object".to_string())?;

    let opts = RequestInit::new();
    opts.set_method(prepared.method.as_str());
    opts.set_mode(RequestMode::Cors);

    let headers = Headers::new().map_err(|_| "Failed to create request headers".to_string())?;
    for (key, value) in &prepared.headers {
        headers
            .set(key, value)
            .map_err(|_| format!("Failed to set header '{}'", key))?;
    }

    match &prepared.body {
        PreparedBody::None => {}
        PreparedBody::Json(json) => {
            if !prepared.headers.contains_key("Content-Type") {
                headers
                    .set("Content-Type", "application/json")
                    .map_err(|_| "Failed to set Content-Type header".to_string())?;
            }
            let body_str = serde_json::to_string(json).map_err(|e| e.to_string())?;
            let body_js = JsValue::from_str(&body_str);
            opts.set_body(&body_js);
        }
        PreparedBody::Form(form) => {
            let form_data = FormData::new().map_err(|_| "Failed to create FormData".to_string())?;
            for (key, value) in form {
                form_data
                    .append_with_str(key, value)
                    .map_err(|_| format!("Failed to append form field '{}'", key))?;
            }
            let form_js: JsValue = form_data.into();
            opts.set_body(&form_js);
        }
        PreparedBody::Raw { content_type, body } => {
            if !prepared.headers.contains_key("Content-Type") {
                headers
                    .set("Content-Type", content_type)
                    .map_err(|_| "Failed to set Content-Type header".to_string())?;
            }
            let body_js = JsValue::from_str(body);
            opts.set_body(&body_js);
        }
        #[cfg(not(target_arch = "wasm32"))]
        PreparedBody::MultipartNative(_) => {
            return Err("Multipart bodies are not supported in WASM".to_string());
        }
    }

    opts.set_headers(&headers);

    let request = Request::new_with_str_and_init(&prepared.url, &opts)
        .map_err(|_| "Failed to create fetch request".to_string())?;

    let response_value = JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|_| "Fetch request failed".to_string())?;

    let response: Response = response_value
        .dyn_into()
        .map_err(|_| "Fetch did not return a Response".to_string())?;

    let status = response.status();
    let mut response_headers = HashMap::new();
    if let Ok(content_type) = response.headers().get("content-type") {
        if let Some(content_type) = content_type {
            if !content_type.is_empty() {
                response_headers.insert("content-type".to_string(), content_type);
            }
        }
    }

    let body_value = JsFuture::from(
        response
            .text()
            .map_err(|_| "Failed to read response body")?,
    )
    .await
    .map_err(|_| "Failed to await response body".to_string())?;

    let body_text = body_value.as_string().unwrap_or_else(|| {
        body_value
            .as_f64()
            .map(|n| n.to_string())
            .unwrap_or_default()
    });

    Ok(FetchResult {
        status,
        headers: response_headers,
        body_text,
    })
}

fn parse_wasm_auth(
    auth_type: Option<String>,
    auth_data_json: Option<String>,
) -> std::result::Result<Option<AuthConfig>, JsValue> {
    match (auth_type, auth_data_json) {
        (None, None) => Ok(None),
        (Some(auth_type), Some(auth_data_json)) => {
            let auth_data: serde_json::Value = serde_json::from_str(&auth_data_json)
                .map_err(|e| JsValue::from_str(&e.to_string()))?;
            parse_runtime_auth(&auth_type, &auth_data)
                .map(Some)
                .map_err(|e| JsValue::from_str(&e))
        }
        _ => Err(JsValue::from_str(
            "auth_type and auth_data_json must both be provided for authenticated clients",
        )),
    }
}

fn parse_runtime_auth(
    auth_type: &str,
    auth_data: &serde_json::Value,
) -> Result<AuthConfig, String> {
    match auth_type {
        "bearer" => {
            let token = auth_data
                .get("token")
                .and_then(|v| v.as_str())
                .filter(|s| !s.trim().is_empty())
                .ok_or("bearer auth requires non-empty 'token'".to_string())?;
            Ok(AuthConfig::bearer(token.to_string()))
        }
        "api_key" => {
            let key = auth_data
                .get("key")
                .and_then(|v| v.as_str())
                .filter(|s| !s.trim().is_empty())
                .ok_or("api_key auth requires non-empty 'key'".to_string())?;
            let name = auth_data
                .get("name")
                .and_then(|v| v.as_str())
                .filter(|s| !s.trim().is_empty())
                .unwrap_or("X-API-Key");
            let in_str = auth_data
                .get("in")
                .and_then(|v| v.as_str())
                .unwrap_or("header");
            let in_ = match in_str {
                "header" => ApiKeyLocation::Header,
                "query" => ApiKeyLocation::Query,
                "cookie" => ApiKeyLocation::Cookie,
                other => {
                    return Err(format!(
                        "Unknown api_key location '{}': expected header, query, or cookie",
                        other
                    ));
                }
            };
            Ok(AuthConfig::api_key_in(
                key.to_string(),
                name.to_string(),
                in_,
            ))
        }
        "http_basic" => {
            let username = auth_data
                .get("username")
                .and_then(|v| v.as_str())
                .filter(|s| !s.trim().is_empty())
                .ok_or("http_basic auth requires non-empty 'username'".to_string())?;
            let password = auth_data
                .get("password")
                .and_then(|v| v.as_str())
                .filter(|s| !s.trim().is_empty())
                .ok_or("http_basic auth requires non-empty 'password'".to_string())?;
            Ok(AuthConfig::http_basic(
                username.to_string(),
                password.to_string(),
            ))
        }
        "http_custom" => {
            let prefix = auth_data
                .get("prefix")
                .and_then(|v| v.as_str())
                .filter(|s| !s.trim().is_empty())
                .ok_or("http_custom auth requires non-empty 'prefix'".to_string())?;
            let token = auth_data
                .get("token")
                .and_then(|v| v.as_str())
                .filter(|s| !s.trim().is_empty())
                .ok_or("http_custom auth requires non-empty 'token'".to_string())?;
            Ok(AuthConfig::http_custom(
                prefix.to_string(),
                token.to_string(),
            ))
        }
        other => Err(format!("Unknown auth type '{}'", other)),
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

    fn api_from_yaml(yaml: &str) -> YcallrApi {
        let bytes = compile_yaml_profile(yaml).unwrap();
        YcallrApi::from_proto(&bytes).unwrap()
    }

    #[wasm_bindgen_test]
    fn test_wasm_from_proto() {
        let api = api_from_yaml(VALID_YAML);
        assert_eq!(api.name(), "test-api");
    }

    #[wasm_bindgen_test]
    fn test_wasm_compile_invalid_yaml() {
        let result = compile_yaml_profile("not valid yaml {{{");
        assert!(result.is_err());
    }

    #[wasm_bindgen_test]
    fn test_wasm_compile_empty_yaml() {
        let result = compile_yaml_profile("");
        assert!(result.is_err());
    }

    #[wasm_bindgen_test]
    fn test_wasm_getters() {
        let api = api_from_yaml(VALID_YAML);
        assert_eq!(api.name(), "test-api");
        assert_eq!(api.version(), "1.0.0");
        assert_eq!(api.base_url(), "https://api.test.com");
        assert_eq!(api.description(), "Test API for WASM");
        assert_eq!(api.command_endpoint("get-item").unwrap(), "/items/{id}");
        assert_eq!(api.command_method("get-item").unwrap(), "GET");
    }

    #[wasm_bindgen_test]
    fn test_wasm_list_commands_and_subcommands() {
        let api = api_from_yaml(NESTED_YAML);
        let top = api.list_commands();
        assert!(top.contains("repos"));
        let issues = api.list_subcommands("repos").unwrap();
        assert!(issues.contains("issues"));
        let create = api.list_subcommands("repos.issues").unwrap();
        assert!(create.contains("create"));
    }

    #[wasm_bindgen_test]
    fn test_wasm_to_json() {
        let api = api_from_yaml(VALID_YAML);
        let json = api.to_json().unwrap();
        assert!(json.contains("test-api"));
        assert!(json.contains("https://api.test.com"));
    }

    #[wasm_bindgen_test]
    fn test_wasm_to_proto() {
        let api = api_from_yaml(VALID_YAML);
        let proto = api.to_proto().unwrap();
        assert!(!proto.is_empty());
    }

    #[wasm_bindgen_test]
    fn test_wasm_command_exists() {
        let api = api_from_yaml(VALID_YAML);
        assert!(api.command_exists("get-item"));
        assert!(api.command_exists("create-item"));
        assert!(!api.command_exists("nonexistent"));
    }

    #[wasm_bindgen_test]
    fn test_wasm_nested_command_exists() {
        let api = api_from_yaml(NESTED_YAML);
        assert!(api.command_exists("repos"));
        assert!(api.command_exists("repos.issues"));
        assert!(api.command_exists("repos.issues.create"));
        assert!(api.command_exists("repos.issues.list"));
        assert!(!api.command_exists("repos.issues.nonexistent"));
        assert!(!api.command_exists("repos.nonexistent"));
    }

    #[wasm_bindgen_test]
    fn test_wasm_nested_yaml_to_proto_roundtrip() {
        let api = api_from_yaml(NESTED_YAML);
        let proto = api.to_proto().unwrap();
        assert!(!proto.is_empty());
        let json = api.to_json().unwrap();
        assert!(json.contains("repos"));
        assert!(json.contains("issues"));
        assert!(json.contains("create"));
    }

    #[wasm_bindgen_test]
    fn test_wasm_env_count() {
        let api = api_from_yaml(ENV_YAML);
        assert_eq!(api.env_count(), 2);
    }

    #[wasm_bindgen_test]
    fn test_wasm_env_name() {
        let api = api_from_yaml(ENV_YAML);
        assert_eq!(api.env_name(0), Some("GITHUB_TOKEN".to_string()));
        assert_eq!(api.env_name(1), Some("GITHUB_URL".to_string()));
        assert_eq!(api.env_name(99), None);
    }

    #[wasm_bindgen_test]
    fn test_wasm_env_required() {
        let api = api_from_yaml(ENV_YAML);
        assert_eq!(api.env_required(0), Some(true));
        assert_eq!(api.env_required(1), Some(false));
        assert_eq!(api.env_required(99), None);
    }

    #[wasm_bindgen_test]
    fn test_wasm_env_yaml_contains_substitution() {
        let api = api_from_yaml(ENV_YAML);
        let json = api.to_json().unwrap();
        assert!(json.contains("${GITHUB_TOKEN}"));
    }

    #[wasm_bindgen_test]
    fn test_wasm_command_has_responses() {
        let api = api_from_yaml(RESPONSE_YAML);
        assert!(api.command_has_responses("create-item"));
        assert!(!api.command_has_responses("nonexistent"));
    }

    #[wasm_bindgen_test]
    fn test_wasm_command_success_message() {
        let api = api_from_yaml(RESPONSE_YAML);
        assert_eq!(
            api.command_success_message("create-item"),
            Some("Created {output.name}".to_string())
        );
    }

    #[wasm_bindgen_test]
    fn test_wasm_command_failure_message() {
        let api = api_from_yaml(RESPONSE_YAML);
        assert_eq!(
            api.command_failure_message("create-item"),
            Some("Failed to create item".to_string())
        );
    }

    #[wasm_bindgen_test]
    fn test_wasm_response_yaml_in_json() {
        let api = api_from_yaml(RESPONSE_YAML);
        let json = api.to_json().unwrap();
        assert!(json.contains("Created {output.name}"));
        assert!(json.contains("Failed to create item"));
    }

    #[wasm_bindgen_test]
    fn test_wasm_command_has_body() {
        let api = api_from_yaml(BODY_YAML);
        assert!(api.command_has_body("create-issue"));
        assert!(!api.command_has_body("nonexistent"));
    }

    #[wasm_bindgen_test]
    fn test_wasm_command_body_json() {
        let api = api_from_yaml(BODY_YAML);
        let body = api.command_body_json("create-issue").unwrap();
        assert!(body.contains("owner_id"));
        assert!(body.contains("{owner}"));
        assert!(body.contains("issue_title"));
        assert!(body.contains("{title}"));
    }

    #[wasm_bindgen_test]
    fn test_wasm_body_yaml_in_json() {
        let api = api_from_yaml(BODY_YAML);
        let json = api.to_json().unwrap();
        assert!(json.contains("owner_id"));
        assert!(json.contains("{owner}"));
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod native_tests {
    use super::*;

    #[test]
    fn test_runtime_auth_parsers_native() {
        assert!(parse_wasm_auth(None, None).unwrap().is_none());

        let bearer = parse_runtime_auth("bearer", &serde_json::json!({"token": "t"})).unwrap();
        assert!(matches!(bearer, AuthConfig::Bearer { .. }));

        let api_key = parse_runtime_auth(
            "api_key",
            &serde_json::json!({"key": "k", "name": "H", "in": "cookie"}),
        )
        .unwrap();
        assert!(matches!(api_key, AuthConfig::ApiKey { .. }));

        assert!(parse_runtime_auth("unknown", &serde_json::json!({})).is_err());
        assert!(parse_runtime_auth("bearer", &serde_json::json!({"token": ""})).is_err());
    }
}
