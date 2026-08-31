mod conversions;

use crate::error::{Result, YcallrError};
use crate::models::ApiDefinition;
use crate::proto;
use prost::Message;
use std::collections::HashMap;

use conversions::{
    auth_from_proto, auth_to_proto, command_from_proto, command_to_proto, env_from_proto,
    env_to_proto,
};

pub struct Compiler;

impl Compiler {
    pub fn yaml_to_proto(api: &ApiDefinition) -> Result<Vec<u8>> {
        api.validate_for_client()?;

        let proto_api = proto::ApiDefinition {
            name: api.name.clone(),
            version: api.version.clone(),
            description: api.description.clone(),
            base_url: api.base_url.clone(),
            commands: api
                .commands
                .iter()
                .map(|(k, v)| (k.clone(), command_to_proto(v)))
                .collect(),
            env: api.env.iter().map(env_to_proto).collect(),
            auth: api
                .auth
                .iter()
                .map(|(k, v)| (k.clone(), auth_to_proto(v)))
                .collect(),
        };

        Ok(proto_api.encode_to_vec())
    }

    pub fn proto_to_yaml(data: &[u8]) -> Result<ApiDefinition> {
        let proto_api =
            proto::ApiDefinition::decode(data).map_err(|e| YcallrError::Protobuf(e.to_string()))?;

        let mut commands = std::collections::HashMap::new();
        for (k, v) in proto_api.commands {
            commands.insert(k, command_from_proto(&v)?);
        }

        let env = proto_api.env.iter().map(env_from_proto).collect();

        let mut auth = HashMap::new();
        for (k, v) in proto_api.auth {
            auth.insert(k, auth_from_proto(&v)?);
        }

        let api = ApiDefinition {
            name: proto_api.name,
            version: proto_api.version,
            description: proto_api.description,
            base_url: proto_api.base_url,
            env,
            auth,
            commands,
        };
        api.validate()?;
        Ok(api)
    }
}

impl ApiDefinition {
    pub fn to_proto_bytes(&self) -> Result<Vec<u8>> {
        Compiler::yaml_to_proto(self)
    }

    pub fn from_proto_bytes(data: &[u8]) -> Result<Self> {
        Compiler::proto_to_yaml(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        AuthConfig, Command, EnvVar, HttpMethod, ParamType, Parameter, ResponseConfig,
        ResponseEntry,
    };
    use std::collections::HashMap;

    use super::conversions::{method_from_i32, type_from_i32};

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

        let mut headers = HashMap::new();
        headers.insert("Accept".to_string(), "application/json".to_string());

        commands.insert(
            "create-issue".to_string(),
            Command {
                description: Some("Create an issue".to_string()),
                endpoint: Some("/repos/{owner}/{repo}/issues".to_string()),
                method: Some(HttpMethod::POST),
                auth: None,
                headers,
                params,
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
            env: vec![EnvVar {
                name: "GITHUB_TOKEN".to_string(),
                required: true,
            }],
            auth: HashMap::new(),
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
            env: vec![
                EnvVar {
                    name: "GITHUB_TOKEN".to_string(),
                    required: true,
                },
                EnvVar {
                    name: "API_VERSION".to_string(),
                    required: false,
                },
            ],
            auth: HashMap::new(),
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

        let mut codes = HashMap::new();
        codes.insert(
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
                    codes,
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
        }
    }

    #[test]
    fn test_yaml_to_proto_and_back() {
        let api = create_test_api();
        let proto_bytes = Compiler::yaml_to_proto(&api).unwrap();
        let restored = Compiler::proto_to_yaml(&proto_bytes).unwrap();

        assert_eq!(api.name, restored.name);
        assert_eq!(api.version, restored.version);
        assert_eq!(api.base_url, restored.base_url);
        assert_eq!(api.commands.len(), restored.commands.len());
    }

    #[test]
    fn test_proto_preserves_commands() {
        let api = create_test_api();
        let proto_bytes = Compiler::yaml_to_proto(&api).unwrap();
        let restored = Compiler::proto_to_yaml(&proto_bytes).unwrap();

        let cmd = restored.commands.get("create-issue").unwrap();
        assert_eq!(cmd.method.as_ref().unwrap(), &HttpMethod::POST);
        assert!(cmd.headers.contains_key("Accept"));
    }

    #[test]
    fn test_method_from_i32_valid_values() {
        assert_eq!(method_from_i32(0).unwrap(), HttpMethod::GET);
        assert_eq!(method_from_i32(1).unwrap(), HttpMethod::POST);
        assert_eq!(method_from_i32(2).unwrap(), HttpMethod::PUT);
        assert_eq!(method_from_i32(3).unwrap(), HttpMethod::DELETE);
        assert_eq!(method_from_i32(4).unwrap(), HttpMethod::PATCH);
    }

    #[test]
    fn test_method_from_i32_invalid_errors() {
        assert!(method_from_i32(99).is_err());
        assert!(method_from_i32(-1).is_err());
    }

    #[test]
    fn test_type_from_i32_valid_values() {
        assert_eq!(type_from_i32(0).unwrap(), ParamType::String);
        assert_eq!(type_from_i32(1).unwrap(), ParamType::Number);
        assert_eq!(type_from_i32(2).unwrap(), ParamType::Boolean);
        assert_eq!(type_from_i32(3).unwrap(), ParamType::Array);
    }

    #[test]
    fn test_type_from_i32_invalid_errors() {
        assert!(type_from_i32(99).is_err());
        assert!(type_from_i32(-1).is_err());
    }

    #[test]
    fn test_api_key_location_from_i32_invalid_errors() {
        use super::conversions::api_key_location_from_i32;
        assert!(api_key_location_from_i32(99).is_err());
    }

    #[test]
    fn test_nested_api_proto_roundtrip() {
        let api = create_nested_api();
        let proto_bytes = Compiler::yaml_to_proto(&api).unwrap();
        let restored = Compiler::proto_to_yaml(&proto_bytes).unwrap();

        assert_eq!(api.name, restored.name);
        assert_eq!(api.commands.len(), restored.commands.len());

        let repos = restored.commands.get("repos").unwrap();
        assert!(repos.commands.is_some());
        assert_eq!(repos.endpoint.as_deref(), Some("/repos"));

        let issues = repos.commands.as_ref().unwrap().get("issues").unwrap();
        assert!(issues.commands.is_some());
        assert_eq!(issues.method.as_ref().unwrap(), &HttpMethod::GET);

        let create = issues.commands.as_ref().unwrap().get("create").unwrap();
        assert_eq!(create.method.as_ref().unwrap(), &HttpMethod::POST);
        assert_eq!(
            create.endpoint.as_deref(),
            Some("/repos/{owner}/{repo}/issues")
        );
    }

    #[test]
    fn test_nested_api_lookup_after_proto() {
        let api = create_nested_api();
        let proto_bytes = Compiler::yaml_to_proto(&api).unwrap();
        let restored = Compiler::proto_to_yaml(&proto_bytes).unwrap();

        let cmd = restored.get_command("repos.issues.create").unwrap();
        assert_eq!(cmd.method.as_ref().unwrap(), &HttpMethod::POST);
    }

    #[test]
    fn test_env_proto_roundtrip() {
        let api = create_env_api();
        let proto_bytes = Compiler::yaml_to_proto(&api).unwrap();
        let restored = Compiler::proto_to_yaml(&proto_bytes).unwrap();

        assert_eq!(restored.env.len(), 2);
        assert_eq!(restored.env[0].name, "GITHUB_TOKEN");
        assert!(restored.env[0].required);
        assert_eq!(restored.env[1].name, "API_VERSION");
        assert!(!restored.env[1].required);
    }

    #[test]
    fn test_env_proto_preserves_substitution() {
        let api = create_env_api();
        let proto_bytes = Compiler::yaml_to_proto(&api).unwrap();
        let restored = Compiler::proto_to_yaml(&proto_bytes).unwrap();

        let cmd = restored.commands.get("get-repo").unwrap();
        assert_eq!(
            cmd.headers.get("Authorization").unwrap(),
            "Bearer ${GITHUB_TOKEN}"
        );
    }

    #[test]
    fn test_description_proto_roundtrip() {
        let api = create_test_api();
        let proto_bytes = Compiler::yaml_to_proto(&api).unwrap();
        let restored = Compiler::proto_to_yaml(&proto_bytes).unwrap();

        let cmd = restored.commands.get("create-issue").unwrap();
        assert_eq!(cmd.description, Some("Create an issue".to_string()));
    }

    #[test]
    fn test_response_config_proto_roundtrip() {
        let api = create_response_api();
        let proto_bytes = Compiler::yaml_to_proto(&api).unwrap();
        let restored = Compiler::proto_to_yaml(&proto_bytes).unwrap();

        let cmd = restored.commands.get("get-repo").unwrap();
        let responses = cmd.responses.as_ref().unwrap();

        assert_eq!(
            responses.success.as_ref().unwrap().message,
            "Got repo {output.name}"
        );
        assert_eq!(
            responses.failure.as_ref().unwrap().message,
            "Failed to get repo"
        );
        assert!(responses.warn.is_none());
        assert_eq!(
            responses.codes.get("404").unwrap().message,
            "{input.owner} does not exist"
        );
    }

    fn create_auth_api() -> ApiDefinition {
        let mut auth = HashMap::new();
        auth.insert(
            "primary".to_string(),
            AuthConfig::bearer("yaml-token".to_string()),
        );
        auth.insert(
            "secondary".to_string(),
            AuthConfig::api_key("api-key".to_string(), "X-API-Key".to_string()),
        );

        let mut commands = HashMap::new();
        commands.insert(
            "get-repo".to_string(),
            Command {
                description: Some("Get repo".to_string()),
                endpoint: Some("/repos/{owner}/{repo}".to_string()),
                method: Some(HttpMethod::GET),
                auth: Some("primary".to_string()),
                headers: HashMap::new(),
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
            env: vec![],
            auth,
            commands,
        }
    }

    #[test]
    fn test_auth_proto_roundtrip() {
        let api = create_auth_api();
        let proto_bytes = Compiler::yaml_to_proto(&api).unwrap();
        let restored = Compiler::proto_to_yaml(&proto_bytes).unwrap();

        assert_eq!(api.auth.len(), restored.auth.len());
        assert!(restored.auth.contains_key("primary"));
        assert!(restored.auth.contains_key("secondary"));

        let primary = restored.auth.get("primary").unwrap();
        assert_eq!(primary, &AuthConfig::bearer("yaml-token".to_string()));

        let cmd = restored.commands.get("get-repo").unwrap();
        assert_eq!(cmd.auth.as_deref(), Some("primary"));
    }

    #[test]
    fn test_conversion_enum_errors_and_all_methods() {
        use super::conversions::{
            api_key_location_from_i32, api_key_location_to_proto, auth_from_proto,
            auth_to_proto, method_from_i32, method_to_proto, type_from_i32, type_to_proto,
        };
        use crate::models::{ApiKeyLocation, HttpMethod, ParamType};
        use crate::proto;

        assert_eq!(method_to_proto(&HttpMethod::PUT), proto::HttpMethod::Put);
        assert_eq!(method_to_proto(&HttpMethod::DELETE), proto::HttpMethod::Delete);
        assert_eq!(method_to_proto(&HttpMethod::PATCH), proto::HttpMethod::Patch);
        assert_eq!(type_to_proto(&ParamType::Boolean), proto::ParamType::Boolean);
        assert_eq!(type_to_proto(&ParamType::Array), proto::ParamType::Array);
        assert!(method_from_i32(99).is_err());
        assert!(type_from_i32(99).is_err());
        assert!(api_key_location_from_i32(99).is_err());
        assert_eq!(
            api_key_location_to_proto(&ApiKeyLocation::Cookie),
            proto::ApiKeyLocation::Cookie
        );

        let http_auth = auth_to_proto(&AuthConfig::http_custom("P ".to_string(), "t".to_string()));
        let restored = auth_from_proto(&http_auth).unwrap();
        assert!(matches!(restored, AuthConfig::Http { .. }));

        let missing = proto::AuthConfig { kind: None };
        assert!(auth_from_proto(&missing).is_err());
    }
}
