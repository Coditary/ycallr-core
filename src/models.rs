use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ApiDefinition {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: String,
    pub base_url: String,
    pub commands: HashMap<String, Command>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Command {
    pub endpoint: String,
    pub method: HttpMethod,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    #[serde(default)]
    pub params: HashMap<String, Parameter>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HttpMethod {
    GET,
    POST,
    PUT,
    DELETE,
    PATCH,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Parameter {
    pub description: String,
    #[serde(rename = "type")]
    pub param_type: ParamType,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ParamType {
    #[serde(rename = "string")]
    String,
    #[serde(rename = "number")]
    Number,
    #[serde(rename = "boolean")]
    Boolean,
    #[serde(rename = "array")]
    Array,
}

impl ApiDefinition {
    pub fn validate(&self) -> crate::Result<()> {
        if self.name.is_empty() {
            return Err(crate::YcallrError::InvalidDefinition(
                "API name cannot be empty".into(),
            ));
        }
        if !self.name.chars().all(|c| c.is_alphanumeric() || c == '-') {
            return Err(crate::YcallrError::InvalidDefinition(
                "API name must be alphanumeric or dash".into(),
            ));
        }
        if self.base_url.is_empty() {
            return Err(crate::YcallrError::InvalidDefinition(
                "Base URL cannot be empty".into(),
            ));
        }
        Ok(())
    }

    pub fn get_command(&self, name: &str) -> crate::Result<&Command> {
        self.commands
            .get(name)
            .ok_or_else(|| crate::YcallrError::CommandNotFound(name.into()))
    }
}

impl Command {
    pub fn resolve_endpoint(&self, params: &HashMap<String, String>) -> crate::Result<String> {
        let mut endpoint = self.endpoint.clone();
        for (key, value) in params {
            endpoint = endpoint.replace(&format!("{{{}}}", key), value);
        }

        let unresolved: Vec<_> = endpoint.matches('{').collect();
        if !unresolved.is_empty() {
            return Err(crate::YcallrError::ParamValidation(
                format!("Unresolved parameters in endpoint: {}", endpoint),
            ));
        }

        Ok(endpoint)
    }
}

impl HttpMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            HttpMethod::GET => "GET",
            HttpMethod::POST => "POST",
            HttpMethod::PUT => "PUT",
            HttpMethod::DELETE => "DELETE",
            HttpMethod::PATCH => "PATCH",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        headers.insert(
            "Accept".to_string(),
            "application/vnd.github.v3+json".to_string(),
        );

        commands.insert(
            "create-issue".to_string(),
            Command {
                endpoint: "/repos/{owner}/{repo}/issues".to_string(),
                method: HttpMethod::POST,
                headers,
                params,
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
    fn test_api_definition_validate_valid() {
        let api = create_test_api();
        assert!(api.validate().is_ok());
    }

    #[test]
    fn test_api_definition_validate_empty_name() {
        let mut api = create_test_api();
        api.name = "".to_string();
        assert!(api.validate().is_err());
    }

    #[test]
    fn test_api_definition_validate_invalid_name() {
        let mut api = create_test_api();
        api.name = "invalid name!".to_string();
        assert!(api.validate().is_err());
    }

    #[test]
    fn test_api_definition_validate_empty_base_url() {
        let mut api = create_test_api();
        api.base_url = "".to_string();
        assert!(api.validate().is_err());
    }

    #[test]
    fn test_get_command_exists() {
        let api = create_test_api();
        let cmd = api.get_command("create-issue");
        assert!(cmd.is_ok());
        assert_eq!(cmd.unwrap().method, HttpMethod::POST);
    }

    #[test]
    fn test_get_command_not_exists() {
        let api = create_test_api();
        let cmd = api.get_command("nonexistent");
        assert!(cmd.is_err());
    }

    #[test]
    fn test_resolve_endpoint() {
        let cmd = Command {
            endpoint: "/repos/{owner}/{repo}/issues".to_string(),
            method: HttpMethod::POST,
            headers: HashMap::new(),
            params: HashMap::new(),
        };

        let mut params = HashMap::new();
        params.insert("owner".to_string(), "rust-lang".to_string());
        params.insert("repo".to_string(), "rust".to_string());

        let resolved = cmd.resolve_endpoint(&params).unwrap();
        assert_eq!(resolved, "/repos/rust-lang/rust/issues");
    }

    #[test]
    fn test_resolve_endpoint_unresolved() {
        let cmd = Command {
            endpoint: "/repos/{owner}/{repo}/issues".to_string(),
            method: HttpMethod::POST,
            headers: HashMap::new(),
            params: HashMap::new(),
        };

        let params = HashMap::new();
        let resolved = cmd.resolve_endpoint(&params);
        assert!(resolved.is_err());
    }

    #[test]
    fn test_http_method_as_str() {
        assert_eq!(HttpMethod::GET.as_str(), "GET");
        assert_eq!(HttpMethod::POST.as_str(), "POST");
        assert_eq!(HttpMethod::PUT.as_str(), "PUT");
        assert_eq!(HttpMethod::DELETE.as_str(), "DELETE");
        assert_eq!(HttpMethod::PATCH.as_str(), "PATCH");
    }
}
