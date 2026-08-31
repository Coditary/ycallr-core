use crate::error::{Result, YcallrError};
use crate::models::{
    ApiErrorConfig, ApiKeyLocation, AuthConfig, BodyConfig, Command, EnvVar, HttpMethod, ParamType,
    Parameter, ResponseConfig, ResponseEntry,
};
use crate::proto;

pub fn command_to_proto(cmd: &Command) -> proto::Command {
    let mut commands = std::collections::HashMap::new();
    if let Some(sub_commands) = &cmd.commands {
        for (k, v) in sub_commands {
            commands.insert(k.clone(), command_to_proto(v));
        }
    }

    proto::Command {
        description: cmd.description.clone(),
        endpoint: cmd.endpoint.clone(),
        method: cmd.method.as_ref().map(|m| method_to_proto(m) as i32),
        headers: cmd.headers.clone(),
        params: cmd
            .params
            .iter()
            .map(|(k, v)| (k.clone(), param_to_proto(v)))
            .collect(),
        commands,
        body: cmd.body.as_ref().map(body_to_proto),
        responses: cmd.responses.as_ref().map(response_config_to_proto),
        auth: cmd.auth.clone(),
    }
}

pub fn command_from_proto(cmd: &proto::Command) -> Result<Command> {
    let mut params = std::collections::HashMap::new();
    for (k, v) in &cmd.params {
        params.insert(k.clone(), param_from_proto(v)?);
    }

    let mut commands = std::collections::HashMap::new();
    for (k, v) in &cmd.commands {
        commands.insert(k.clone(), command_from_proto(v)?);
    }

    Ok(Command {
        description: cmd.description.clone(),
        endpoint: cmd.endpoint.clone(),
        method: cmd.method.map(method_from_i32).transpose()?,
        auth: cmd.auth.clone(),
        headers: cmd.headers.clone(),
        params,
        body: match &cmd.body {
            Some(body) => Some(body_from_proto(body)?),
            None => None,
        },
        responses: cmd.responses.as_ref().map(response_config_from_proto),
        commands: if commands.is_empty() {
            None
        } else {
            Some(commands)
        },
    })
}

pub fn response_config_to_proto(config: &ResponseConfig) -> proto::ResponseConfig {
    proto::ResponseConfig {
        success: config.success.as_ref().map(response_entry_to_proto),
        failure: config.failure.as_ref().map(response_entry_to_proto),
        warn: config.warn.as_ref().map(response_entry_to_proto),
        codes: config
            .codes
            .iter()
            .map(|(k, v)| (k.clone(), response_entry_to_proto(v)))
            .collect(),
    }
}

pub fn response_config_from_proto(config: &proto::ResponseConfig) -> ResponseConfig {
    ResponseConfig {
        success: config.success.as_ref().map(response_entry_from_proto),
        failure: config.failure.as_ref().map(response_entry_from_proto),
        warn: config.warn.as_ref().map(response_entry_from_proto),
        codes: config
            .codes
            .iter()
            .map(|(k, v)| (k.clone(), response_entry_from_proto(v)))
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

pub fn api_error_config_to_proto(config: &ApiErrorConfig) -> proto::ApiErrorConfig {
    proto::ApiErrorConfig {
        default_entry: config.default.as_ref().map(response_entry_to_proto),
        codes: config
            .codes
            .iter()
            .map(|(k, v)| (k.clone(), response_entry_to_proto(v)))
            .collect(),
    }
}

pub fn api_error_config_from_proto(config: &proto::ApiErrorConfig) -> ApiErrorConfig {
    ApiErrorConfig {
        default: config.default_entry.as_ref().map(response_entry_from_proto),
        codes: config
            .codes
            .iter()
            .map(|(k, v)| (k.clone(), response_entry_from_proto(v)))
            .collect(),
    }
}

pub fn body_to_proto(body: &BodyConfig) -> proto::BodyConfig {
    proto::BodyConfig {
        json: body.json.as_ref().map(|v| v.to_string()),
        fields: body
            .form
            .as_ref()
            .map(|m| {
                let mut map = std::collections::HashMap::new();
                for (k, v) in m {
                    map.insert(k.clone(), v.clone());
                }
                map
            })
            .unwrap_or_default(),
        raw: body.raw.clone(),
        multipart: body
            .multipart
            .as_ref()
            .map(|fields| {
                fields
                    .iter()
                    .map(|f| proto::MultipartField {
                        name: f.name.clone(),
                        text: f.text.clone(),
                        file: f.file.clone(),
                    })
                    .collect()
            })
            .unwrap_or_default(),
    }
}

pub fn body_from_proto(body: &proto::BodyConfig) -> Result<BodyConfig> {
    let json = if let Some(s) = &body.json {
        Some(
            serde_json::from_str(s)
                .map_err(|e| YcallrError::Protobuf(format!("Invalid JSON in body.json: {}", e)))?,
        )
    } else {
        None
    };

    Ok(BodyConfig {
        json,
        form: if body.fields.is_empty() {
            None
        } else {
            Some(body.fields.clone())
        },
        raw: body.raw.clone(),
        multipart: if body.multipart.is_empty() {
            None
        } else {
            Some(
                body.multipart
                    .iter()
                    .map(|f| crate::models::MultipartField {
                        name: f.name.clone(),
                        text: f.text.clone(),
                        file: f.file.clone(),
                    })
                    .collect(),
            )
        },
    })
}

pub fn env_to_proto(env: &EnvVar) -> proto::EnvVar {
    proto::EnvVar {
        name: env.name.clone(),
        required: env.required,
    }
}

pub fn env_from_proto(env: &proto::EnvVar) -> EnvVar {
    EnvVar {
        name: env.name.clone(),
        required: env.required,
    }
}

pub fn method_to_proto(method: &HttpMethod) -> proto::HttpMethod {
    match method {
        HttpMethod::GET => proto::HttpMethod::Get,
        HttpMethod::POST => proto::HttpMethod::Post,
        HttpMethod::PUT => proto::HttpMethod::Put,
        HttpMethod::DELETE => proto::HttpMethod::Delete,
        HttpMethod::PATCH => proto::HttpMethod::Patch,
    }
}

pub fn method_from_i32(method: i32) -> Result<HttpMethod> {
    match method {
        0 => Ok(HttpMethod::GET),
        1 => Ok(HttpMethod::POST),
        2 => Ok(HttpMethod::PUT),
        3 => Ok(HttpMethod::DELETE),
        4 => Ok(HttpMethod::PATCH),
        _ => Err(YcallrError::Protobuf(format!(
            "Unknown HTTP method enum value: {}",
            method
        ))),
    }
}

pub fn param_to_proto(param: &Parameter) -> proto::Parameter {
    proto::Parameter {
        description: param.description.clone(),
        r#type: type_to_proto(&param.param_type) as i32,
        required: param.required,
    }
}

pub fn param_from_proto(param: &proto::Parameter) -> Result<Parameter> {
    Ok(Parameter {
        description: param.description.clone(),
        param_type: type_from_i32(param.r#type)?,
        required: param.required,
    })
}

pub fn type_to_proto(t: &ParamType) -> proto::ParamType {
    match t {
        ParamType::String => proto::ParamType::String,
        ParamType::Number => proto::ParamType::Number,
        ParamType::Boolean => proto::ParamType::Boolean,
        ParamType::Array => proto::ParamType::Array,
    }
}

pub fn type_from_i32(t: i32) -> Result<ParamType> {
    match t {
        0 => Ok(ParamType::String),
        1 => Ok(ParamType::Number),
        2 => Ok(ParamType::Boolean),
        3 => Ok(ParamType::Array),
        _ => Err(YcallrError::Protobuf(format!(
            "Unknown parameter type enum value: {}",
            t
        ))),
    }
}

pub fn auth_to_proto(auth: &AuthConfig) -> proto::AuthConfig {
    match auth {
        AuthConfig::Bearer { token } => proto::AuthConfig {
            kind: Some(proto::auth_config::Kind::Bearer(proto::BearerAuth {
                token: token.clone(),
            })),
        },
        AuthConfig::ApiKey { key, name, in_ } => proto::AuthConfig {
            kind: Some(proto::auth_config::Kind::ApiKey(proto::ApiKeyAuth {
                key: key.clone(),
                name: name.clone(),
                location: api_key_location_to_proto(in_) as i32,
            })),
        },
        AuthConfig::Http {
            scheme,
            token,
            username,
            password,
            prefix,
        } => proto::AuthConfig {
            kind: Some(proto::auth_config::Kind::Http(proto::HttpAuth {
                scheme: scheme.clone(),
                token: token.clone(),
                username: username.clone(),
                password: password.clone(),
                prefix: prefix.clone(),
            })),
        },
    }
}

pub fn auth_from_proto(auth: &proto::AuthConfig) -> Result<AuthConfig> {
    match &auth.kind {
        Some(proto::auth_config::Kind::Bearer(bearer)) => Ok(AuthConfig::Bearer {
            token: bearer.token.clone(),
        }),
        Some(proto::auth_config::Kind::ApiKey(api_key)) => Ok(AuthConfig::ApiKey {
            key: api_key.key.clone(),
            name: api_key.name.clone(),
            in_: api_key_location_from_i32(api_key.location)?,
        }),
        Some(proto::auth_config::Kind::Http(http)) => Ok(AuthConfig::Http {
            scheme: http.scheme.clone(),
            token: http.token.clone(),
            username: http.username.clone(),
            password: http.password.clone(),
            prefix: http.prefix.clone(),
        }),
        None => Err(YcallrError::Protobuf(
            "AuthConfig has no variant set".into(),
        )),
    }
}

pub fn api_key_location_to_proto(location: &ApiKeyLocation) -> proto::ApiKeyLocation {
    match location {
        ApiKeyLocation::Header => proto::ApiKeyLocation::Header,
        ApiKeyLocation::Query => proto::ApiKeyLocation::Query,
        ApiKeyLocation::Cookie => proto::ApiKeyLocation::Cookie,
    }
}

pub fn api_key_location_from_i32(location: i32) -> Result<ApiKeyLocation> {
    match location {
        0 => Ok(ApiKeyLocation::Header),
        1 => Ok(ApiKeyLocation::Query),
        2 => Ok(ApiKeyLocation::Cookie),
        _ => Err(YcallrError::Protobuf(format!(
            "Unknown API key location enum value: {}",
            location
        ))),
    }
}
