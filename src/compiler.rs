use prost::Message;
use crate::error::{Result, YcallrError};
use crate::models::{ApiDefinition, Command, HttpMethod, ParamType, Parameter};
use crate::proto;

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
        };

        Ok(proto_api.encode_to_vec())
    }

    pub fn proto_to_yaml(data: &[u8]) -> Result<ApiDefinition> {
        let proto_api = proto::ApiDefinition::decode(data)
            .map_err(|e| YcallrError::Protobuf(e.to_string()))?;

        let mut commands = std::collections::HashMap::new();
        for (k, v) in proto_api.commands {
            commands.insert(k, Self::command_from_proto(&v)?);
        }

        Ok(ApiDefinition {
            name: proto_api.name,
            version: proto_api.version,
            description: proto_api.description,
            base_url: proto_api.base_url,
            commands,
        })
    }

    fn command_to_proto(cmd: &Command) -> proto::Command {
        proto::Command {
            endpoint: cmd.endpoint.clone(),
            method: Self::method_to_proto(&cmd.method) as i32,
            headers: cmd.headers.clone(),
            params: cmd
                .params
                .iter()
                .map(|(k, v)| (k.clone(), Self::param_to_proto(v)))
                .collect(),
        }
    }

    fn command_from_proto(cmd: &proto::Command) -> Result<Command> {
        let mut params = std::collections::HashMap::new();
        for (k, v) in &cmd.params {
            params.insert(k.clone(), Self::param_from_proto(v)?);
        }

        Ok(Command {
            endpoint: cmd.endpoint.clone(),
            method: Self::method_from_i32(cmd.method),
            headers: cmd.headers.clone(),
            params,
        })
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
        assert_eq!(cmd.method, HttpMethod::POST);
        assert!(cmd.headers.contains_key("Accept"));
    }
}
