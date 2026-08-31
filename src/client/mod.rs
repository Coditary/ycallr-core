mod builder;
mod request;
mod types;

pub use crate::call_engine::templates;
pub use types::*;

use crate::error::Result;
use crate::models::ApiDefinition;
use std::collections::HashMap;

use builder::YcallrClientBuilder;

#[derive(Debug)]
pub struct YcallrClient {
    pub(crate) api: ApiDefinition,
    pub(crate) http_client: reqwest::blocking::Client,
    pub(crate) auth: Option<AuthConfig>,
    pub(crate) auth_configs: HashMap<String, AuthConfig>,
    pub(crate) env_mode: EnvMode,
    pub(crate) env_vars: HashMap<String, String>,
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

    pub fn set_env(&mut self, key: &str, value: &str) -> Result<()> {
        builder::validate_declared_env_key(&self.api, key)?;

        if let Some(env_var) = self.api.env.iter().find(|e| e.name == key) {
            if env_var.required && value.trim().is_empty() {
                return Err(crate::YcallrError::EnvVar(format!(
                    "Required environment variable '{}' cannot be empty",
                    key
                )));
            }
        }

        self.env_vars.insert(key.to_string(), value.to_string());
        Ok(())
    }

    pub fn get_env(&self, key: &str) -> Option<&str> {
        self.env_vars.get(key).map(|s| s.as_str())
    }

    pub fn resolve_env_vars(&self, text: &str) -> Result<String> {
        templates::resolve_env_vars(text, &self.env_vars)
    }

    pub fn resolve_body(
        &self,
        body_config: &crate::models::BodyConfig,
        params: &HashMap<String, String>,
    ) -> Result<crate::models::BodyConfig> {
        templates::resolve_body(body_config, params)
    }

    pub fn resolve_json_templates(
        &self,
        value: &serde_json::Value,
        params: &HashMap<String, String>,
    ) -> Result<serde_json::Value> {
        templates::resolve_json_templates(value, params)
    }

    pub fn call(
        &self,
        command: &str,
        params: &HashMap<String, String>,
        body: Option<&serde_json::Value>,
    ) -> Result<ApiResponse> {
        request::call(self, command, params, body)
    }

    pub fn validate_params(
        &self,
        command: &str,
        params: &HashMap<String, String>,
        body: Option<&serde_json::Value>,
    ) -> Result<()> {
        let cmd = self.api.get_command(command)?;
        cmd.validate_params(params, body)
    }

    pub fn api(&self) -> &ApiDefinition {
        &self.api
    }

    pub fn get_auth_config(&self, name: &str) -> Option<&AuthConfig> {
        self.auth_configs.get(name)
    }

    pub fn command_details(&self, command: &str) -> Result<crate::models::CommandDetails> {
        self.api.command_details(command)
    }

    pub fn list_subcommands(&self, command: &str) -> Result<Vec<String>> {
        self.api.list_subcommands(command)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::{AuthConfig, EnvMode};
    use crate::models::ApiKeyLocation;
    use crate::models::{Command, ParamType, Parameter, ResponseConfig, ResponseEntry};
    use crate::HttpMethod;

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
                auth: None,
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
                auth: None,
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
            auth: HashMap::new(),
            commands,
            errors: None,
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
                auth: None,
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
            auth: HashMap::new(),
            commands,
            errors: None,
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
                auth: None,
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
                auth: None,
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
                auth: None,
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
                auth: None,
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
            auth: HashMap::new(),
            commands,
            errors: None,
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
                auth: None,
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
            auth: HashMap::new(),
            commands,
            errors: None,
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
        let client = YcallrClient::with_auth(api, AuthConfig::bearer("test-token".to_string()));
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
        let bearer = AuthConfig::bearer("token".to_string());
        let api_key = AuthConfig::api_key("key123".to_string(), "X-API-Key".to_string());

        match bearer {
            AuthConfig::Bearer { token } => assert_eq!(token, "token"),
            _ => panic!("Expected Bearer"),
        }

        match api_key {
            AuthConfig::ApiKey { key, name, in_ } => {
                assert_eq!(key, "key123");
                assert_eq!(name, "X-API-Key");
                assert_eq!(in_, ApiKeyLocation::Header);
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
    fn test_command_details_via_client() {
        let api = create_nested_api();
        let client = YcallrClient::new(api).unwrap();

        let details = client.command_details("repos.issues.list").unwrap();
        assert_eq!(details.path, "repos.issues.list");
        assert_eq!(details.method.as_ref(), Some(&HttpMethod::GET));
        assert!(details.is_callable);
        assert!(details.subcommands.is_empty());

        let subcommands = client.list_subcommands("repos.issues").unwrap();
        assert_eq!(subcommands, vec!["create".to_string(), "list".to_string()]);
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
    fn test_env_required_empty_string_errors() {
        let api = create_env_api();
        let result = YcallrClient::builder(api)
            .env_mode(EnvMode::Manual)
            .env("GITHUB_TOKEN", "")
            .build();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot be empty"));
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
        let client = client.unwrap();
        assert_eq!(client.get_env("OPTIONAL_VAR"), Some(""));
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

        let result = client.resolve_env_vars("${UNKNOWN}");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Unknown environment variable 'UNKNOWN'"));
    }

    #[test]
    fn test_env_undeclared_builder_var_errors() {
        let api = create_test_api();
        let result = YcallrClient::builder(api)
            .env_mode(EnvMode::Manual)
            .env("NOT_IN_PROFILE", "value")
            .build();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("not declared in the API profile"));
    }

    #[test]
    fn test_builder_pattern() {
        let mut api = create_test_api();
        api.env = vec![crate::models::EnvVar {
            name: "KEY".to_string(),
            required: false,
        }];
        let client = YcallrClient::builder(api)
            .auth(AuthConfig::bearer("token".to_string()))
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
        let api = create_env_api();
        let mut client = YcallrClient::builder(api)
            .env_mode(EnvMode::Manual)
            .env("GITHUB_TOKEN", "ghp_initial")
            .build()
            .unwrap();
        client.set_env("GITHUB_TOKEN", "ghp_updated").unwrap();
        assert_eq!(client.get_env("GITHUB_TOKEN"), Some("ghp_updated"));
    }

    #[test]
    fn test_set_env_undeclared_key_errors() {
        let api = create_test_api();
        let mut client = YcallrClient::new(api).unwrap();
        let result = client.set_env("SECRET", "value");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not declared"));
    }

    #[test]
    fn test_set_env_required_empty_errors() {
        let api = create_env_api();
        let mut client = YcallrClient::builder(api)
            .env_mode(EnvMode::Manual)
            .env("GITHUB_TOKEN", "ghp_initial")
            .build()
            .unwrap();
        let result = client.set_env("GITHUB_TOKEN", "");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot be empty"));
    }

    #[test]
    fn test_env_mode_default() {
        let api = create_test_api();
        let client = YcallrClient::new(api).unwrap();
        assert_eq!(client.env_mode(), &EnvMode::Auto);
    }

    #[test]
    fn test_client_build_rejects_invalid_command_structure() {
        let mut commands = HashMap::new();
        commands.insert(
            "broken".to_string(),
            Command {
                description: None,
                endpoint: Some("/broken".to_string()),
                method: None,
                auth: None,
                headers: HashMap::new(),
                params: HashMap::new(),
                body: None,
                responses: None,
                commands: None,
            },
        );

        let api = ApiDefinition {
            name: "test".to_string(),
            version: "1.0.0".to_string(),
            description: "Test".to_string(),
            base_url: "http://127.0.0.1:8080".to_string(),
            env: vec![],
            auth: HashMap::new(),
            commands,
            errors: None,
        };

        let result = YcallrClient::new(api);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("endpoint and method"));
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
    use crate::HttpMethod;

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
                auth: None,
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
            auth: HashMap::new(),
            commands,
            errors: None,
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
