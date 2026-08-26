use crate::error::{Result, YcallrError};
use crate::models::{
    ApiDefinition, BodyConfig, Command, EnvVar, HttpMethod, ParamType, Parameter, ResponseConfig,
    ResponseEntry,
};
use crate::proto;
use prost::Message;

pub struct Compiler;

impl Compiler {
    pub fn yaml_to_proto(api: &ApiDefinition) -> Result<Vec<u8>> {
        let proto_api = proto::ApiDefinition {
            name: api.name.clone(),
            version: api.version.clone(),
            description: api.description.clone(),
            base_url: api.base_url.clone(),
            commands: api
                .commands
                .iter()
                .map(|(k, v)| (k.clone(), Self::command_to_proto(v)))
                .collect(),
            env: api.env.iter().map(Self::env_to_proto).collect(),
        };

        Ok(proto_api.encode_to_vec())
    }

    pub fn proto_to_yaml(data: &[u8]) -> Result<ApiDefinition> {
        let proto_api =
            proto::ApiDefinition::decode(data).map_err(|e| YcallrError::Protobuf(e.to_string()))?;

        let mut commands = std::collections::HashMap::new();
        for (k, v) in proto_api.commands {
            commands.insert(k, Self::command_from_proto(&v)?);
        }

        let env = proto_api.env.iter().map(Self::env_from_proto).collect();

        Ok(ApiDefinition {
            name: proto_api.name,
            version: proto_api.version,
            description: proto_api.description,
            base_url: proto_api.base_url,
            env,
            commands,
        })
    }

    pub fn command_to_proto(cmd: &Command) -> proto::Command {
        let mut commands = std::collections::HashMap::new();
        if let Some(sub_commands) = &cmd.commands {
            for (k, v) in sub_commands {
                commands.insert(k.clone(), Self::command_to_proto(v));
            }
        }

        proto::Command {
            description: cmd.description.clone(),
            endpoint: cmd.endpoint.clone(),
            method: cmd.method.as_ref().map(|m| Self::method_to_proto(m) as i32),
            headers: cmd.headers.clone(),
            params: cmd
                .params
                .iter()
                .map(|(k, v)| (k.clone(), Self::param_to_proto(v)))
                .collect(),
            commands,
            body: cmd.body.as_ref().map(Self::body_to_proto),
            responses: cmd.responses.as_ref().map(Self::response_config_to_proto),
        }
    }

    pub fn command_from_proto(cmd: &proto::Command) -> Result<Command> {
        let mut params = std::collections::HashMap::new();
        for (k, v) in &cmd.params {
            params.insert(k.clone(), Self::param_from_proto(v)?);
        }

        let mut commands = std::collections::HashMap::new();
        for (k, v) in &cmd.commands {
            commands.insert(k.clone(), Self::command_from_proto(v)?);
        }

        Ok(Command {
            description: cmd.description.clone(),
            endpoint: cmd.endpoint.clone(),
            method: cmd.method.map(|m| Self::method_from_i32(m)),
            headers: cmd.headers.clone(),
            params,
            body: cmd.body.as_ref().map(Self::body_from_proto),
            responses: cmd.responses.as_ref().map(Self::response_config_from_proto),
            commands: if commands.is_empty() {
                None
            } else {
                Some(commands)
            },
        })
    }

    fn response_config_to_proto(config: &ResponseConfig) -> proto::ResponseConfig {
        proto::ResponseConfig {
            success: config.success.as_ref().map(Self::response_entry_to_proto),
            failure: config.failure.as_ref().map(Self::response_entry_to_proto),
            warn: config.warn.as_ref().map(Self::response_entry_to_proto),
            codes: config
                .codes
                .iter()
                .map(|(k, v)| (k.clone(), Self::response_entry_to_proto(v)))
                .collect(),
        }
    }

    fn response_config_from_proto(config: &proto::ResponseConfig) -> ResponseConfig {
        ResponseConfig {
            success: config.success.as_ref().map(Self::response_entry_from_proto),
            failure: config.failure.as_ref().map(Self::response_entry_from_proto),
            warn: config.warn.as_ref().map(Self::response_entry_from_proto),
            codes: config
                .codes
                .iter()
                .map(|(k, v)| (k.clone(), Self::response_entry_from_proto(v)))
                .collect(),
        }
    }

    fn response_entry_to_proto(entry: &ResponseEntry) -> proto::ResponseEntry {
        proto::ResponseEntry {
            message: entry.message.clone(),
        }
    }

    fn response_entry_from_proto(entry: &proto::ResponseEntry) -> ResponseEntry {
        ResponseEntry {
            message: entry.message.clone(),
        }
    }

    fn body_to_proto(body: &BodyConfig) -> proto::BodyConfig {
        proto::BodyConfig {
            json: body.json.as_ref().map(|v| v.to_string()),
        }
    }

    fn body_from_proto(body: &proto::BodyConfig) -> BodyConfig {
        BodyConfig {
            json: body
                .json
                .as_ref()
                .and_then(|s| serde_json::from_str(s).ok()),
        }
    }

    fn env_to_proto(env: &EnvVar) -> proto::EnvVar {
        proto::EnvVar {
            name: env.name.clone(),
            required: env.required,
        }
    }

    fn env_from_proto(env: &proto::EnvVar) -> EnvVar {
        EnvVar {
            name: env.name.clone(),
            required: env.required,
        }
    }

    fn method_to_proto(method: &HttpMethod) -> proto::HttpMethod {
        match method {
            HttpMethod::GET => proto::HttpMethod::Get,
            HttpMethod::POST => proto::HttpMethod::Post,
            HttpMethod::PUT => proto::HttpMethod::Put,
            HttpMethod::DELETE => proto::HttpMethod::Delete,
            HttpMethod::PATCH => proto::HttpMethod::Patch,
        }
    }

    fn method_from_i32(method: i32) -> HttpMethod {
        match method {
            0 => HttpMethod::GET,
            1 => HttpMethod::POST,
            2 => HttpMethod::PUT,
            3 => HttpMethod::DELETE,
            4 => HttpMethod::PATCH,
            _ => HttpMethod::GET,
        }
    }

    fn param_to_proto(param: &Parameter) -> proto::Parameter {
        proto::Parameter {
            description: param.description.clone(),
            r#type: Self::type_to_proto(&param.param_type) as i32,
            required: param.required,
        }
    }

    fn param_from_proto(param: &proto::Parameter) -> Result<Parameter> {
        Ok(Parameter {
            description: param.description.clone(),
            param_type: Self::type_from_i32(param.r#type),
            required: param.required,
        })
    }

    fn type_to_proto(t: &ParamType) -> proto::ParamType {
        match t {
            ParamType::String => proto::ParamType::String,
            ParamType::Number => proto::ParamType::Number,
            ParamType::Boolean => proto::ParamType::Boolean,
            ParamType::Array => proto::ParamType::Array,
        }
    }

    fn type_from_i32(t: i32) -> ParamType {
        match t {
            0 => ParamType::String,
            1 => ParamType::Number,
            2 => ParamType::Boolean,
            3 => ParamType::Array,
            _ => ParamType::String,
        }
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
    use std::collections::HashMap;

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
            env: vec![EnvVar {
                name: "GITHUB_TOKEN".to_string(),
                required: true,
            }],
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
    fn test_method_from_i32_fallback() {
        assert_eq!(Compiler::method_from_i32(0), HttpMethod::GET);
        assert_eq!(Compiler::method_from_i32(1), HttpMethod::POST);
        assert_eq!(Compiler::method_from_i32(2), HttpMethod::PUT);
        assert_eq!(Compiler::method_from_i32(3), HttpMethod::DELETE);
        assert_eq!(Compiler::method_from_i32(4), HttpMethod::PATCH);
        assert_eq!(Compiler::method_from_i32(99), HttpMethod::GET);
        assert_eq!(Compiler::method_from_i32(-1), HttpMethod::GET);
    }

    #[test]
    fn test_type_from_i32_fallback() {
        assert_eq!(Compiler::type_from_i32(0), ParamType::String);
        assert_eq!(Compiler::type_from_i32(1), ParamType::Number);
        assert_eq!(Compiler::type_from_i32(2), ParamType::Boolean);
        assert_eq!(Compiler::type_from_i32(3), ParamType::Array);
        assert_eq!(Compiler::type_from_i32(99), ParamType::String);
        assert_eq!(Compiler::type_from_i32(-1), ParamType::String);
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
}
