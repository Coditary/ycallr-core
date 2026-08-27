use crate::error::Result;
use crate::models::{
    BodyConfig, Command, EnvVar, HttpMethod, ParamType, Parameter, ResponseConfig, ResponseEntry,
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
        method: cmd.method.map(|m| method_from_i32(m)),
        headers: cmd.headers.clone(),
        params,
        body: cmd.body.as_ref().map(body_from_proto),
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

pub fn body_from_proto(body: &proto::BodyConfig) -> BodyConfig {
    BodyConfig {
        json: body
            .json
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok()),
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
    }
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

pub fn method_from_i32(method: i32) -> HttpMethod {
    match method {
        0 => HttpMethod::GET,
        1 => HttpMethod::POST,
        2 => HttpMethod::PUT,
        3 => HttpMethod::DELETE,
        4 => HttpMethod::PATCH,
        _ => HttpMethod::GET,
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
        param_type: type_from_i32(param.r#type),
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

pub fn type_from_i32(t: i32) -> ParamType {
    match t {
        0 => ParamType::String,
        1 => ParamType::Number,
        2 => ParamType::Boolean,
        3 => ParamType::Array,
        _ => ParamType::String,
    }
}
