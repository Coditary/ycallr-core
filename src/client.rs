use crate::error::{Result, YcallrError};
use crate::models::{ApiDefinition, HttpMethod};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum AuthConfig {
    Bearer(String),
    ApiKey { key: String, header: String },
}

#[derive(Debug, Clone, PartialEq)]
pub enum EnvMode {
    Auto,
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    pub status: u16,
    pub message: String,
    pub body: Option<serde_json::Value>,
}

pub struct YcallrClientBuilder {
    api: ApiDefinition,
    auth: Option<AuthConfig>,
    env_mode: EnvMode,
    env_vars: HashMap<String, String>,
}

impl YcallrClientBuilder {
    pub fn new(api: ApiDefinition) -> Self {
        Self {
            api,
            auth: None,
            env_mode: EnvMode::Auto,
            env_vars: HashMap::new(),
        }
    }

    pub fn auth(mut self, auth: AuthConfig) -> Self {
        self.auth = Some(auth);
        self
    }

    pub fn env_mode(mut self, mode: EnvMode) -> Self {
        self.env_mode = mode;
        self
    }

    pub fn env(mut self, key: &str, value: &str) -> Self {
        self.env_vars.insert(key.to_string(), value.to_string());
        self
    }

    pub fn envs(mut self, vars: HashMap<String, String>) -> Self {
        self.env_vars.extend(vars);
        self
    }

    pub fn build(self) -> Result<YcallrClient> {
        let mut resolved_env = HashMap::new();

        for env_var in &self.api.env {
            match self.env_mode {
                EnvMode::Auto => {
                    if let Ok(val) = std::env::var(&env_var.name) {
                        resolved_env.insert(env_var.name.clone(), val);
                    } else if let Some(val) = self.env_vars.get(&env_var.name) {
                        resolved_env.insert(env_var.name.clone(), val.clone());
                    }
                }
                EnvMode::Manual => {
                    if let Some(val) = self.env_vars.get(&env_var.name) {
                        resolved_env.insert(env_var.name.clone(), val.clone());
                    }
                }
            }
        }

        for (key, value) in &self.env_vars {
            resolved_env
                .entry(key.clone())
                .or_insert_with(|| value.clone());
        }

        for env_var in &self.api.env {
            if env_var.required && !resolved_env.contains_key(&env_var.name) {
                return Err(YcallrError::EnvVar(format!(
                    "Required environment variable '{}' is not set",
                    env_var.name
                )));
            }
        }

        let http_client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| YcallrError::HttpClient(e.to_string()))?;

        Ok(YcallrClient {
            api: self.api,
            http_client,
            auth: self.auth,
            env_mode: self.env_mode,
            env_vars: resolved_env,
        })
    }
}

#[derive(Debug)]
pub struct YcallrClient {
    api: ApiDefinition,
    http_client: reqwest::blocking::Client,
    auth: Option<AuthConfig>,
    env_mode: EnvMode,
    env_vars: HashMap<String, String>,
}

impl YcallrClient {
    pub fn new(api: ApiDefinition) -> Result<Self> {
        Self::builder(api).build()
    }

    pub fn with_auth(api: ApiDefinition, auth: AuthConfig) -> Result<Self> {
        Self::builder(api).auth(auth).build()
    }

    pub fn builder(api: ApiDefinition) -> YcallrClientBuilder {
        YcallrClientBuilder::new(api)
    }

    pub fn env_mode(&self) -> &EnvMode {
        &self.env_mode
    }

    pub fn set_env(&mut self, key: &str, value: &str) {
        self.env_vars.insert(key.to_string(), value.to_string());
    }

    pub fn get_env(&self, key: &str) -> Option<&str> {
        self.env_vars.get(key).map(|s| s.as_str())
    }

    fn resolve_env_vars(&self, text: &str) -> Result<String> {
        let re = Regex::new(r"\$\{([^}]+)\}").unwrap();
        let mut result = text.to_string();

        for cap in re.captures_iter(text) {
            let var_name = &cap[1];
            let replacement = self
                .env_vars
                .get(var_name)
                .map(|s| s.as_str())
                .unwrap_or("");
            result = result.replace(&cap[0], replacement);
        }

        Ok(result)
    }

    fn resolve_response_template(
        template: &str,
        params: &HashMap<String, String>,
        body: &serde_json::Value,
    ) -> String {
        let re = Regex::new(r"\{(input|output)\.([^}]+)\}").unwrap();
        let mut result = template.to_string();

        for cap in re.captures_iter(template) {
            let prefix = &cap[1];
            let field = &cap[2];

            let replacement = match prefix {
                "input" => params.get(field).map(|s| s.as_str()).unwrap_or(""),
                "output" => {
                    if let Some(val) = body.get(field) {
                        match val {
                            serde_json::Value::String(s) => s.as_str(),
                            other => {
                                let s = other.to_string();
                                return result.replace(&cap[0], &s);
                            }
                        }
                    } else {
                        ""
                    }
                }
                _ => "",
            };

            result = result.replace(&cap[0], replacement);
        }

        result
    }

    fn resolve_body_templates(
        &self,
        body_config: &crate::models::BodyConfig,
        params: &HashMap<String, String>,
    ) -> Result<serde_json::Value> {
        if let Some(json) = &body_config.json {
            let resolved = self.resolve_json_templates(json, params)?;
            Ok(resolved)
        } else {
            Ok(serde_json::Value::Null)
        }
    }

    pub fn resolve_json_templates(
        &self,
        value: &serde_json::Value,
        params: &HashMap<String, String>,
    ) -> Result<serde_json::Value> {
        match value {
            serde_json::Value::String(s) => {
                let mut resolved = s.clone();
                for (key, val) in params {
                    resolved = resolved.replace(&format!("{{{}}}", key), val);
                }
                Ok(serde_json::Value::String(resolved))
            }
            serde_json::Value::Array(arr) => {
                let mut resolved = Vec::new();
                for item in arr {
                    resolved.push(self.resolve_json_templates(item, params)?);
                }
                Ok(serde_json::Value::Array(resolved))
            }
            serde_json::Value::Object(map) => {
                let mut resolved = serde_json::Map::new();
                for (k, v) in map {
                    resolved.insert(k.clone(), self.resolve_json_templates(v, params)?);
                }
                Ok(serde_json::Value::Object(resolved))
            }
            other => Ok(other.clone()),
        }
    }

    pub fn call(
        &self,
        command: &str,
        params: &HashMap<String, String>,
        body: Option<&serde_json::Value>,
    ) -> Result<ApiResponse> {
        let cmd = self.api.get_command(command)?;

        let endpoint = cmd.resolve_endpoint(params)?;
        let resolved_endpoint = self.resolve_env_vars(&endpoint)?;
        let url = format!(
            "{}{}",
            self.api.base_url.trim_end_matches('/'),
            resolved_endpoint
        );

        let method = cmd
            .method
            .as_ref()
            .ok_or_else(|| YcallrError::ParamValidation("Command has no method".into()))?;

        let mut request = match method {
            HttpMethod::GET => self.http_client.get(&url),
            HttpMethod::POST => self.http_client.post(&url),
            HttpMethod::PUT => self.http_client.put(&url),
            HttpMethod::DELETE => self.http_client.delete(&url),
            HttpMethod::PATCH => self.http_client.patch(&url),
        };

        for (key, value) in &cmd.headers {
            let resolved_value = self.resolve_env_vars(value)?;
            request = request.header(key.as_str(), resolved_value.as_str());
        }

        if let Some(auth) = &self.auth {
            match auth {
                AuthConfig::Bearer(token) => {
                    request = request.bearer_auth(token);
                }
                AuthConfig::ApiKey { key, header } => {
                    request = request.header(header.as_str(), key.as_str());
                }
            }
        }

        let final_body = if let Some(caller_body) = body {
            Some(caller_body.clone())
        } else if let Some(body_config) = &cmd.body {
            let resolved_body = self.resolve_body_templates(body_config, params)?;
            if resolved_body.is_null() {
                None
            } else {
                Some(resolved_body)
            }
        } else {
            None
        };

        if let Some(body) = &final_body {
            request = request.json(body);
        }

        let response = request
            .send()
            .map_err(|e| YcallrError::HttpClient(e.to_string()))?;

        let status = response.status().as_u16();

        let headers: HashMap<String, String> = response
            .headers()
            .iter()
            .filter_map(|(k, v)| v.to_str().ok().map(|val| (k.to_string(), val.to_string())))
            .collect();

        let body_text = response
            .text()
            .map_err(|e| YcallrError::HttpClient(e.to_string()))?;

        let body_json: serde_json::Value = serde_json::from_str(&body_text)
            .unwrap_or_else(|_| serde_json::Value::String(body_text));

        let message = if let Some(responses) = &cmd.responses {
            if let Some(entry) = responses.get_entry_for_status(status) {
                Some(Self::resolve_response_template(
                    &entry.message,
                    params,
                    &body_json,
                ))
            } else {
                None
            }
        } else {
            None
        };

        Ok(ApiResponse {
            status,
            headers,
            body: body_json,
            message,
        })
    }

    pub fn api(&self) -> &ApiDefinition {
        &self.api
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Command, ParamType, Parameter, ResponseConfig, ResponseEntry};

    fn create_test_api() -> ApiDefinition {
        let mut commands = HashMap::new();
        let mut params = HashMap::new();

        params.insert(
            "owner".to_string(),
            Parameter {
                description: "Repository owner".to_string(),
                param_type: ParamType::String,
                required: true,
            },
        );

        params.insert(
            "repo".to_string(),
            Parameter {
                description: "Repository name".to_string(),
                param_type: ParamType::String,
                required: true,
            },
        );

        let mut headers = HashMap::new();
        headers.insert("Accept".to_string(), "application/json".to_string());

        commands.insert(
            "get-repo".to_string(),
            Command {
                description: Some("Get a repository".to_string()),
                endpoint: Some("/repos/{owner}/{repo}".to_string()),
                method: Some(HttpMethod::GET),
                headers,
                params,
                body: None,
                responses: None,
                commands: None,
            },
        );

        let mut create_params = HashMap::new();
        create_params.insert(
            "title".to_string(),
            Parameter {
                description: "Issue title".to_string(),
                param_type: ParamType::String,
                required: true,
            },
        );

        let mut create_headers = HashMap::new();
        create_headers.insert("Content-Type".to_string(), "application/json".to_string());

        commands.insert(
            "create-issue".to_string(),
            Command {
                description: Some("Create an issue".to_string()),
                endpoint: Some("/repos/{owner}/{repo}/issues".to_string()),
                method: Some(HttpMethod::POST),
                headers: create_headers,
                params: create_params,
                body: None,
                responses: None,
                commands: None,
            },
        );

        ApiDefinition {
            name: "github".to_string(),
            version: "1.0.0".to_string(),
            description: "GitHub API".to_string(),
            base_url: "https://api.github.com".to_string(),
            env: vec![],
            commands,
        }
    }

    fn create_response_api() -> ApiDefinition {
        let mut commands = HashMap::new();
        let mut params = HashMap::new();

        params.insert(
            "owner".to_string(),
            Parameter {
                description: "Repository owner".to_string(),
                param_type: ParamType::String,
                required: true,
            },
        );

        params.insert(
            "repo".to_string(),
            Parameter {
                description: "Repository name".to_string(),
                param_type: ParamType::String,
                required: true,
            },
        );

        let mut responses = HashMap::new();
        responses.insert(
            "404".to_string(),
            ResponseEntry {
                message: "{input.owner} does not exist".to_string(),
            },
        );

        commands.insert(
            "get-repo".to_string(),
            Command {
                description: Some("Get a repository".to_string()),
                endpoint: Some("/repos/{owner}/{repo}".to_string()),
                method: Some(HttpMethod::GET),
                headers: HashMap::new(),
                params,
                body: None,
                responses: Some(ResponseConfig {
                    success: Some(ResponseEntry {
                        message: "Got repo {output.name}".to_string(),
                    }),
                    failure: Some(ResponseEntry {
                        message: "Failed to get repo".to_string(),
                    }),
                    warn: None,
                    codes: responses,
                }),
                commands: None,
            },
        );

        ApiDefinition {
            name: "github".to_string(),
            version: "1.0.0".to_string(),
            description: "GitHub API".to_string(),
            base_url: "https://api.github.com".to_string(),
            env: vec![],
            commands,
        }
    }

    fn create_nested_api() -> ApiDefinition {
        let mut commands = HashMap::new();

        let mut repos_commands = HashMap::new();

        let mut issues_commands = HashMap::new();
        issues_commands.insert(
            "create".to_string(),
            Command {
                description: Some("Create an issue".to_string()),
                endpoint: Some("/repos/{owner}/{repo}/issues".to_string()),
                method: Some(HttpMethod::POST),
                headers: HashMap::new(),
                params: HashMap::new(),
                body: None,
                responses: None,
                commands: None,
            },
        );
        issues_commands.insert(
            "list".to_string(),
            Command {
                description: Some("List issues".to_string()),
                endpoint: Some("/repos/{owner}/{repo}/issues".to_string()),
                method: Some(HttpMethod::GET),
                headers: HashMap::new(),
                params: HashMap::new(),
                body: None,
                responses: None,
                commands: None,
            },
        );

        repos_commands.insert(
            "issues".to_string(),
            Command {
                description: Some("Issues operations".to_string()),
                endpoint: Some("/repos/{owner}/{repo}/issues".to_string()),
                method: Some(HttpMethod::GET),
                headers: HashMap::new(),
                params: HashMap::new(),
                body: None,
                responses: None,
                commands: Some(issues_commands),
            },
        );

        commands.insert(
            "repos".to_string(),
            Command {
                description: Some("Repository operations".to_string()),
                endpoint: Some("/repos".to_string()),
                method: Some(HttpMethod::GET),
                headers: HashMap::new(),
                params: HashMap::new(),
                body: None,
                responses: None,
                commands: Some(repos_commands),
            },
        );

        ApiDefinition {
            name: "github".to_string(),
            version: "1.0.0".to_string(),
            description: "GitHub API".to_string(),
            base_url: "https://api.github.com".to_string(),
            env: vec![],
            commands,
        }
    }

    fn create_env_api() -> ApiDefinition {
        let mut commands = HashMap::new();

        let mut headers = HashMap::new();
        headers.insert(
            "Authorization".to_string(),
            "Bearer ${GITHUB_TOKEN}".to_string(),
        );

        commands.insert(
            "get-repo".to_string(),
            Command {
                description: Some("Get a repository".to_string()),
                endpoint: Some("/repos/{owner}/{repo}".to_string()),
                method: Some(HttpMethod::GET),
                headers,
                params: HashMap::new(),
                body: None,
                responses: None,
                commands: None,
            },
        );

        ApiDefinition {
            name: "github".to_string(),
            version: "1.0.0".to_string(),
            description: "GitHub API".to_string(),
            base_url: "https://api.github.com".to_string(),
            env: vec![crate::models::EnvVar {
                name: "GITHUB_TOKEN".to_string(),
                required: true,
            }],
            commands,
        }
    }

    #[test]
    fn test_client_new() {
        let api = create_test_api();
        let client = YcallrClient::new(api);
        assert!(client.is_ok());
    }

    #[test]
    fn test_client_with_auth() {
        let api = create_test_api();
        let client = YcallrClient::with_auth(api, AuthConfig::Bearer("test-token".to_string()));
        assert!(client.is_ok());
        let client = client.unwrap();
        assert!(client.auth.is_some());
    }

    #[test]
    fn test_call_command_not_found() {
        let api = create_test_api();
        let client = YcallrClient::new(api).unwrap();
        let params = HashMap::new();
        let result = client.call("nonexistent", &params, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_auth_types() {
        let bearer = AuthConfig::Bearer("token".to_string());
        let api_key = AuthConfig::ApiKey {
            key: "key123".to_string(),
            header: "X-API-Key".to_string(),
        };

        match bearer {
            AuthConfig::Bearer(t) => assert_eq!(t, "token"),
            _ => panic!("Expected Bearer"),
        }

        match api_key {
            AuthConfig::ApiKey { key, header } => {
                assert_eq!(key, "key123");
                assert_eq!(header, "X-API-Key");
            }
            _ => panic!("Expected ApiKey"),
        }
    }

    #[test]
    fn test_api_response_structure() {
        let response = ApiResponse {
            status: 200,
            headers: HashMap::new(),
            body: serde_json::json!({"key": "value"}),
            message: None,
        };

        assert_eq!(response.status, 200);
        assert_eq!(response.body["key"], "value");
        assert!(response.message.is_none());
    }

    #[test]
    fn test_api_response_with_message() {
        let response = ApiResponse {
            status: 200,
            headers: HashMap::new(),
            body: serde_json::json!({"name": "rust"}),
            message: Some("Got repo rust".to_string()),
        };

        assert_eq!(response.status, 200);
        assert_eq!(response.message.unwrap(), "Got repo rust");
    }

    #[test]
    fn test_nested_command_lookup() {
        let api = create_nested_api();
        let client = YcallrClient::new(api).unwrap();
        let cmd = client.api.get_command("repos.issues.create").unwrap();
        assert_eq!(cmd.method.as_ref().unwrap(), &HttpMethod::POST);
    }

    #[test]
    fn test_nested_command_not_found() {
        let api = create_nested_api();
        let client = YcallrClient::new(api).unwrap();
        let params: HashMap<String, String> = HashMap::new();
        let result = client.call("repos.issues.nonexistent", &params, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_nested_endpoint_resolution() {
        let api = create_nested_api();
        let client = YcallrClient::new(api).unwrap();
        let mut params = HashMap::new();
        params.insert("owner".to_string(), "rust-lang".to_string());
        params.insert("repo".to_string(), "rust".to_string());
        let cmd = client.api.get_command("repos.issues.create").unwrap();
        let endpoint = cmd.resolve_endpoint(&params).unwrap();
        assert_eq!(endpoint, "/repos/rust-lang/rust/issues");
    }

    #[test]
    fn test_env_required_missing() {
        let api = create_env_api();
        let result = YcallrClient::builder(api).env_mode(EnvMode::Manual).build();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("GITHUB_TOKEN"));
    }

    #[test]
    fn test_env_required_set_manual() {
        let api = create_env_api();
        let client = YcallrClient::builder(api)
            .env_mode(EnvMode::Manual)
            .env("GITHUB_TOKEN", "ghp_test123")
            .build();
        assert!(client.is_ok());
        let client = client.unwrap();
        assert_eq!(client.get_env("GITHUB_TOKEN"), Some("ghp_test123"));
    }

    #[test]
    fn test_env_not_required_missing() {
        let mut api = create_test_api();
        api.env = vec![crate::models::EnvVar {
            name: "OPTIONAL_VAR".to_string(),
            required: false,
        }];
        let client = YcallrClient::builder(api).env_mode(EnvMode::Manual).build();
        assert!(client.is_ok());
    }

    #[test]
    fn test_env_resolve_in_headers() {
        let api = create_env_api();
        let client = YcallrClient::builder(api)
            .env_mode(EnvMode::Manual)
            .env("GITHUB_TOKEN", "ghp_test123")
            .build()
            .unwrap();

        let resolved = client.resolve_env_vars("Bearer ${GITHUB_TOKEN}").unwrap();
        assert_eq!(resolved, "Bearer ghp_test123");
    }

    #[test]
    fn test_env_resolve_multiple_vars() {
        let api = create_env_api();
        let client = YcallrClient::builder(api)
            .env_mode(EnvMode::Manual)
            .env("GITHUB_TOKEN", "ghp_test123")
            .build()
            .unwrap();

        let resolved = client
            .resolve_env_vars("${GITHUB_TOKEN} and ${GITHUB_TOKEN}")
            .unwrap();
        assert_eq!(resolved, "ghp_test123 and ghp_test123");
    }

    #[test]
    fn test_env_resolve_unknown_var() {
        let api = create_env_api();
        let client = YcallrClient::builder(api)
            .env_mode(EnvMode::Manual)
            .env("GITHUB_TOKEN", "ghp_test123")
            .build()
            .unwrap();

        let resolved = client.resolve_env_vars("${UNKNOWN}").unwrap();
        assert_eq!(resolved, "");
    }

    #[test]
    fn test_builder_pattern() {
        let api = create_test_api();
        let client = YcallrClient::builder(api)
            .auth(AuthConfig::Bearer("token".to_string()))
            .env_mode(EnvMode::Manual)
            .env("KEY", "value")
            .build()
            .unwrap();

        assert!(client.auth.is_some());
        assert_eq!(client.env_mode(), &EnvMode::Manual);
        assert_eq!(client.get_env("KEY"), Some("value"));
    }

    #[test]
    fn test_set_env_after_creation() {
        let api = create_test_api();
        let mut client = YcallrClient::new(api).unwrap();
        client.set_env("NEW_KEY", "new_value");
        assert_eq!(client.get_env("NEW_KEY"), Some("new_value"));
    }

    #[test]
    fn test_env_mode_default() {
        let api = create_test_api();
        let client = YcallrClient::new(api).unwrap();
        assert_eq!(client.env_mode(), &EnvMode::Auto);
    }

    #[test]
    fn test_resolve_response_template_input() {
        let mut params = HashMap::new();
        params.insert("owner".to_string(), "rust-lang".to_string());
        let body = serde_json::json!({});

        let result =
            YcallrClient::resolve_response_template("{input.owner} not found", &params, &body);
        assert_eq!(result, "rust-lang not found");
    }

    #[test]
    fn test_resolve_response_template_output() {
        let params = HashMap::new();
        let body = serde_json::json!({"name": "rust", "stars": 90000});

        let result = YcallrClient::resolve_response_template(
            "Got repo {output.name} with {output.stars} stars",
            &params,
            &body,
        );
        assert_eq!(result, "Got repo rust with 90000 stars");
    }

    #[test]
    fn test_resolve_response_template_mixed() {
        let mut params = HashMap::new();
        params.insert("owner".to_string(), "rust-lang".to_string());
        let body = serde_json::json!({"name": "rust"});

        let result =
            YcallrClient::resolve_response_template("{input.owner}/{output.name}", &params, &body);
        assert_eq!(result, "rust-lang/rust");
    }

    #[test]
    fn test_resolve_response_template_missing_field() {
        let params = HashMap::new();
        let body = serde_json::json!({"name": "rust"});

        let result = YcallrClient::resolve_response_template("{output.missing}", &params, &body);
        assert_eq!(result, "");
    }

    #[test]
    fn test_response_api_parsing() {
        let api = create_response_api();
        let cmd = api.commands.get("get-repo").unwrap();
        let responses = cmd.responses.as_ref().unwrap();
        assert_eq!(
            responses.success.as_ref().unwrap().message,
            "Got repo {output.name}"
        );
        assert_eq!(
            responses.failure.as_ref().unwrap().message,
            "Failed to get repo"
        );
        assert_eq!(
            responses.codes.get("404").unwrap().message,
            "{input.owner} does not exist"
        );
    }
}

#[cfg(test)]
#[cfg(feature = "test-utils")]
mod client_integration_tests {
    use super::*;
    use crate::models::{Command, ResponseConfig, ResponseEntry};
    use crate::test_utils::{make_params, response_ok};

    fn create_response_test_api(base_url: &str) -> ApiDefinition {
        let mut commands = HashMap::new();
        let mut params = HashMap::new();

        params.insert(
            "owner".to_string(),
            crate::models::Parameter {
                description: "Repository owner".to_string(),
                param_type: crate::models::ParamType::String,
                required: true,
            },
        );

        params.insert(
            "repo".to_string(),
            crate::models::Parameter {
                description: "Repository name".to_string(),
                param_type: crate::models::ParamType::String,
                required: true,
            },
        );

        commands.insert(
            "get-repo".to_string(),
            Command {
                description: Some("Get a repository".to_string()),
                endpoint: Some("/repos/{owner}/{repo}".to_string()),
                method: Some(HttpMethod::GET),
                headers: HashMap::new(),
                params,
                body: None,
                responses: Some(ResponseConfig {
                    success: Some(ResponseEntry {
                        message: "Got repo {output.name}".to_string(),
                    }),
                    failure: Some(ResponseEntry {
                        message: "Failed to get repo".to_string(),
                    }),
                    warn: None,
                    codes: std::collections::HashMap::new(),
                }),
                commands: None,
            },
        );

        ApiDefinition {
            name: "github".to_string(),
            version: "1.0.0".to_string(),
            description: "GitHub API".to_string(),
            base_url: base_url.to_string(),
            env: vec![],
            commands,
        }
    }

    #[test]
    fn test_mock_response_success() {
        let mut mock = crate::test_utils::MockApiClient::new();
        mock.expect("get-repo", response_ok(serde_json::json!({"name": "rust"})));

        let params = make_params(&[("owner", "rust-lang"), ("repo", "rust")]);
        let response = mock.call("get-repo", &params, None).unwrap();

        assert_eq!(response.status, 200);
        assert_eq!(response.body["name"], "rust");
    }

    #[test]
    fn test_mock_response_with_message() {
        let mut server = mockito::Server::new();

        let mock = server
            .mock("GET", "/repos/rust-lang/rust")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"name": "rust"}"#)
            .create();

        let api = create_response_test_api(&server.url());
        let client = YcallrClient::new(api).unwrap();

        let mut params = HashMap::new();
        params.insert("owner".to_string(), "rust-lang".to_string());
        params.insert("repo".to_string(), "rust".to_string());

        let response = client.call("get-repo", &params, None).unwrap();

        mock.assert();
        assert_eq!(response.status, 200);
        assert_eq!(response.message.unwrap(), "Got repo rust");
    }

    #[test]
    fn test_mock_response_failure_message() {
        let mut server = mockito::Server::new();

        let mock = server
            .mock("GET", "/repos/rust-lang/missing")
            .with_status(404)
            .with_header("content-type", "application/json")
            .with_body(r#"{"message": "Not Found"}"#)
            .create();

        let api = create_response_test_api(&server.url());
        let client = YcallrClient::new(api).unwrap();

        let mut params = HashMap::new();
        params.insert("owner".to_string(), "rust-lang".to_string());
        params.insert("repo".to_string(), "missing".to_string());

        let response = client.call("get-repo", &params, None).unwrap();

        mock.assert();
        assert_eq!(response.status, 404);
        assert_eq!(response.message.unwrap(), "Failed to get repo");
    }
}
