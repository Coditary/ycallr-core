use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::error::{Result, YcallrError};
use crate::models::{ApiDefinition, HttpMethod};

#[derive(Debug, Clone)]
pub enum AuthConfig {
    Bearer(String),
    ApiKey { key: String, header: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    pub status: u16,
    pub message: String,
    pub body: Option<serde_json::Value>,
}

pub struct YcallrClient {
    api: ApiDefinition,
    http_client: reqwest::blocking::Client,
    auth: Option<AuthConfig>,
}

impl YcallrClient {
    pub fn new(api: ApiDefinition) -> Result<Self> {
        let http_client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| YcallrError::HttpClient(e.to_string()))?;

        Ok(Self {
            api,
            http_client,
            auth: None,
        })
    }

    pub fn with_auth(api: ApiDefinition, auth: AuthConfig) -> Result<Self> {
        let mut client = Self::new(api)?;
        client.auth = Some(auth);
        Ok(client)
    }

    pub fn call(
        &self,
        command: &str,
        params: &HashMap<String, String>,
        body: Option<&serde_json::Value>,
    ) -> Result<ApiResponse> {
        let cmd = self.api.get_command(command)?;

        let endpoint = cmd.resolve_endpoint(params)?;
        let url = format!("{}{}", self.api.base_url.trim_end_matches('/'), endpoint);

        let mut request = match cmd.method {
            HttpMethod::GET => self.http_client.get(&url),
            HttpMethod::POST => self.http_client.post(&url),
            HttpMethod::PUT => self.http_client.put(&url),
            HttpMethod::DELETE => self.http_client.delete(&url),
            HttpMethod::PATCH => self.http_client.patch(&url),
        };

        for (key, value) in &cmd.headers {
            request = request.header(key.as_str(), value.as_str());
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

        if let Some(body) = body {
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

        Ok(ApiResponse {
            status,
            headers,
            body: body_json,
        })
    }

    pub fn api(&self) -> &ApiDefinition {
        &self.api
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Command, HttpMethod, Parameter, ParamType};

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
                endpoint: "/repos/{owner}/{repo}".to_string(),
                method: HttpMethod::GET,
                headers,
                params,
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
                endpoint: "/repos/{owner}/{repo}/issues".to_string(),
                method: HttpMethod::POST,
                headers: create_headers,
                params: create_params,
            },
        );

        ApiDefinition {
            name: "github".to_string(),
            version: "1.0.0".to_string(),
            description: "GitHub API".to_string(),
            base_url: "https://api.github.com".to_string(),
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
        let client = YcallrClient::with_auth(
            api,
            AuthConfig::Bearer("test-token".to_string()),
        );
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
        };

        assert_eq!(response.status, 200);
        assert_eq!(response.body["key"], "value");
    }
}
