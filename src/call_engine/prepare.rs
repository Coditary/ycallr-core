use std::collections::HashMap;

use percent_encoding::{utf8_percent_encode, AsciiSet, NON_ALPHANUMERIC};

use crate::error::{Result, YcallrError};
use crate::models::{ApiKeyLocation, AuthConfig, Command, HttpMethod, ResponseConfig};

use super::context::ClientContext;
use super::templates;
use super::types::ApiResponse;

const QUERY_ENCODE_SET: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'.')
    .remove(b'_')
    .remove(b'~');

#[derive(Debug, Clone)]
pub struct PreparedHttpRequest {
    pub method: HttpMethod,
    pub url: String,
    pub headers: HashMap<String, String>,
    pub body: PreparedBody,
    pub responses: Option<ResponseConfig>,
    pub params: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub enum PreparedBody {
    None,
    Json(serde_json::Value),
    Form(Vec<(String, String)>),
    Raw {
        content_type: String,
        body: String,
    },
    #[cfg(not(target_arch = "wasm32"))]
    MultipartNative(Vec<NativeMultipartPart>),
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone)]
pub enum NativeMultipartPart {
    Text {
        name: String,
        value: String,
    },
    File {
        name: String,
        path: std::path::PathBuf,
    },
}

pub fn prepare_http_request(
    ctx: &ClientContext,
    command: &str,
    params: &HashMap<String, String>,
    body: Option<&serde_json::Value>,
) -> Result<PreparedHttpRequest> {
    let cmd = ctx.api.get_command(command)?;

    cmd.validate_params(params, body)?;

    let endpoint = cmd.resolve_endpoint(params)?;
    let resolved_endpoint = templates::resolve_env_vars(&endpoint, &ctx.env_vars)?;
    let base_url = format!(
        "{}{}",
        ctx.api.base_url.trim_end_matches('/'),
        resolved_endpoint
    );

    let method = cmd
        .method
        .as_ref()
        .ok_or_else(|| YcallrError::ParamValidation("Command has no method".into()))?
        .clone();

    let mut query_pairs = build_query_params(cmd, params);
    let auth_config = resolve_auth_config(cmd, ctx)?;
    if let Some(auth) = auth_config {
        add_auth_query_params(auth, &ctx.env_vars, &mut query_pairs)?;
    }

    let mut headers = headers_map(cmd, &ctx.env_vars)?;
    if let Some(auth) = auth_config {
        apply_auth_headers(auth, &ctx.env_vars, &mut headers)?;
    }

    let url = append_query_pairs(&base_url, &query_pairs);

    let prepared_body = if let Some(runtime_body) = body {
        PreparedBody::Json(runtime_body.clone())
    } else if let Some(body_config) = &cmd.body {
        let resolved = templates::resolve_body(body_config, params)?;
        prepare_yaml_body(&resolved)?
    } else {
        PreparedBody::None
    };

    Ok(PreparedHttpRequest {
        method,
        url,
        headers,
        body: prepared_body,
        responses: cmd.responses.clone(),
        params: params.clone(),
    })
}

fn headers_map(
    cmd: &Command,
    env_vars: &HashMap<String, String>,
) -> Result<HashMap<String, String>> {
    let mut headers = HashMap::new();
    for (key, value) in &cmd.headers {
        headers.insert(key.clone(), templates::resolve_env_vars(value, env_vars)?);
    }
    Ok(headers)
}

fn prepare_yaml_body(body_config: &crate::models::BodyConfig) -> Result<PreparedBody> {
    if let Some(json) = &body_config.json {
        return Ok(PreparedBody::Json(json.clone()));
    }
    if let Some(form) = &body_config.form {
        return Ok(PreparedBody::Form(
            form.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
        ));
    }
    if let Some(raw) = &body_config.raw {
        return Ok(PreparedBody::Raw {
            content_type: "text/plain".to_string(),
            body: raw.clone(),
        });
    }
    if let Some(multipart_fields) = &body_config.multipart {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = multipart_fields;
            return Err(YcallrError::HttpClient(
                "Multipart bodies are not supported in WASM".into(),
            ));
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut parts = Vec::new();
            for field in multipart_fields {
                if let Some(text) = &field.text {
                    parts.push(NativeMultipartPart::Text {
                        name: field.name.clone(),
                        value: text.clone(),
                    });
                } else if let Some(file_path) = &field.file {
                    let path = resolve_multipart_file_path(file_path)?;
                    parts.push(NativeMultipartPart::File {
                        name: field.name.clone(),
                        path,
                    });
                }
            }
            if parts.is_empty() {
                Ok(PreparedBody::None)
            } else {
                Ok(PreparedBody::MultipartNative(parts))
            }
        }
    } else {
        Ok(PreparedBody::None)
    }
}

pub fn build_api_response(
    status: u16,
    headers: HashMap<String, String>,
    body_text: String,
    responses: Option<&ResponseConfig>,
    params: &HashMap<String, String>,
) -> ApiResponse {
    let body_json: serde_json::Value =
        serde_json::from_str(&body_text).unwrap_or_else(|_| serde_json::Value::String(body_text));

    let message = responses.and_then(|responses| {
        responses
            .get_entry_for_status(status)
            .map(|entry| templates::resolve_response_template(&entry.message, params, &body_json))
    });

    ApiResponse {
        status,
        headers,
        body: body_json,
        message,
    }
}

fn append_query_pairs(url: &str, pairs: &[(String, String)]) -> String {
    if pairs.is_empty() {
        return url.to_string();
    }

    let query = pairs
        .iter()
        .map(|(key, value)| {
            format!(
                "{}={}",
                encode_query_component(key),
                encode_query_component(value)
            )
        })
        .collect::<Vec<_>>()
        .join("&");

    if url.contains('?') {
        format!("{}&{}", url, query)
    } else {
        format!("{}?{}", url, query)
    }
}

fn encode_query_component(value: &str) -> String {
    utf8_percent_encode(value, QUERY_ENCODE_SET).to_string()
}

fn build_query_params(cmd: &Command, params: &HashMap<String, String>) -> Vec<(String, String)> {
    let path_params = cmd.endpoint_path_param_names();
    params
        .iter()
        .filter(|(key, _)| !path_params.contains(key))
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect()
}

fn resolve_auth_config<'a>(
    cmd: &'a Command,
    ctx: &'a ClientContext,
) -> Result<Option<&'a AuthConfig>> {
    match cmd.auth.as_ref() {
        None => Ok(ctx.auth.as_ref()),
        Some(name) if crate::models::command_auth_is_none(name) => Ok(None),
        Some(name) => ctx.auth_configs.get(name).map(Some).ok_or_else(|| {
            YcallrError::InvalidDefinition(format!("Auth config '{}' not found", name))
        }),
    }
}

fn add_auth_query_params(
    auth: &AuthConfig,
    env_vars: &HashMap<String, String>,
    query_pairs: &mut Vec<(String, String)>,
) -> Result<()> {
    if let AuthConfig::ApiKey {
        key,
        name,
        in_: ApiKeyLocation::Query,
    } = auth
    {
        let resolved_key = templates::resolve_env_vars(key, env_vars)?;
        let resolved_name = templates::resolve_env_vars(name, env_vars)?;
        require_nonempty_auth_value(&resolved_name, "api_key name")?;
        require_nonempty_auth_value(&resolved_key, "api_key value")?;
        if query_pairs.iter().any(|(key, _)| key == &resolved_name) {
            return Err(YcallrError::ParamValidation(format!(
                "Duplicate query parameter '{}'",
                resolved_name
            )));
        }
        query_pairs.push((resolved_name, resolved_key));
    }
    Ok(())
}

fn apply_auth_headers(
    auth: &AuthConfig,
    env_vars: &HashMap<String, String>,
    headers: &mut HashMap<String, String>,
) -> Result<()> {
    match auth {
        AuthConfig::Bearer { token } => {
            let resolved_token = templates::resolve_env_vars(token, env_vars)?;
            require_nonempty_auth_value(&resolved_token, "bearer token")?;
            headers.insert(
                "Authorization".to_string(),
                format!("Bearer {}", resolved_token),
            );
        }
        AuthConfig::ApiKey { key, name, in_ } => match in_ {
            ApiKeyLocation::Header => {
                let resolved_key = templates::resolve_env_vars(key, env_vars)?;
                let resolved_name = templates::resolve_env_vars(name, env_vars)?;
                require_nonempty_auth_value(&resolved_name, "api_key name")?;
                require_nonempty_auth_value(&resolved_key, "api_key value")?;
                headers.insert(resolved_name, resolved_key);
            }
            ApiKeyLocation::Query => {}
            ApiKeyLocation::Cookie => {
                let resolved_key = templates::resolve_env_vars(key, env_vars)?;
                let resolved_name = templates::resolve_env_vars(name, env_vars)?;
                require_nonempty_auth_value(&resolved_name, "api_key name")?;
                require_nonempty_auth_value(&resolved_key, "api_key value")?;
                validate_cookie_component(&resolved_name, "name")?;
                validate_cookie_component(&resolved_key, "value")?;
                headers.insert(
                    "Cookie".to_string(),
                    format!("{}={}", resolved_name, resolved_key),
                );
            }
        },
        AuthConfig::Http {
            scheme,
            token,
            username,
            password,
            prefix,
        } => {
            let resolved_token = token
                .as_ref()
                .map(|t| templates::resolve_env_vars(t, env_vars))
                .transpose()?;
            let resolved_prefix = prefix
                .as_ref()
                .map(|p| templates::resolve_env_vars(p, env_vars))
                .transpose()?;

            match scheme.to_lowercase().as_str() {
                "bearer" => {
                    let resolved = resolved_token.as_ref().ok_or_else(|| {
                        YcallrError::ParamValidation("HTTP bearer auth requires token".into())
                    })?;
                    require_nonempty_auth_value(resolved, "bearer token")?;
                    headers.insert("Authorization".to_string(), format!("Bearer {}", resolved));
                }
                "basic" => {
                    let resolved_user = username
                        .as_ref()
                        .map(|u| templates::resolve_env_vars(u, env_vars))
                        .transpose()?
                        .ok_or_else(|| {
                            YcallrError::ParamValidation("HTTP basic auth requires username".into())
                        })?;
                    let resolved_pass = password
                        .as_ref()
                        .map(|p| templates::resolve_env_vars(p, env_vars))
                        .transpose()?
                        .ok_or_else(|| {
                            YcallrError::ParamValidation("HTTP basic auth requires password".into())
                        })?;
                    require_nonempty_auth_value(&resolved_user, "basic username")?;
                    require_nonempty_auth_value(&resolved_pass, "basic password")?;
                    use base64::Engine;
                    let credentials = format!("{}:{}", resolved_user, resolved_pass);
                    let encoded = base64::engine::general_purpose::STANDARD.encode(credentials);
                    headers.insert("Authorization".to_string(), format!("Basic {}", encoded));
                }
                "custom" => {
                    let resolved_prefix = resolved_prefix.as_ref().ok_or_else(|| {
                        YcallrError::ParamValidation("HTTP custom auth requires prefix".into())
                    })?;
                    let resolved_token = resolved_token.as_ref().ok_or_else(|| {
                        YcallrError::ParamValidation("HTTP custom auth requires token".into())
                    })?;
                    require_nonempty_auth_value(resolved_prefix, "custom auth prefix")?;
                    require_nonempty_auth_value(resolved_token, "custom auth token")?;
                    headers.insert(
                        "Authorization".to_string(),
                        format!("{}{}", resolved_prefix, resolved_token),
                    );
                }
                _ => {
                    let resolved = resolved_token.as_ref().ok_or_else(|| {
                        YcallrError::ParamValidation(format!(
                            "HTTP auth scheme '{}' requires token",
                            scheme
                        ))
                    })?;
                    require_nonempty_auth_value(resolved, "auth token")?;
                    let scheme_upper = scheme.to_uppercase();
                    headers.insert(
                        "Authorization".to_string(),
                        format!("{} {}", scheme_upper, resolved),
                    );
                }
            }
        }
    }

    Ok(())
}

fn require_nonempty_auth_value(value: &str, field: &str) -> Result<()> {
    if value.trim().is_empty() {
        Err(YcallrError::ParamValidation(format!(
            "Auth {} is empty or unresolved",
            field
        )))
    } else {
        Ok(())
    }
}

fn validate_cookie_component(value: &str, label: &str) -> Result<()> {
    if value.chars().any(|c| matches!(c, ';' | '\r' | '\n' | '\0')) {
        return Err(YcallrError::ParamValidation(format!(
            "Cookie {} contains invalid characters",
            label
        )));
    }
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
fn resolve_multipart_file_path(file_path: &str) -> Result<std::path::PathBuf> {
    if file_path.trim().is_empty() {
        return Err(YcallrError::ParamValidation(
            "Multipart file path cannot be empty".into(),
        ));
    }

    if file_path.contains("..") {
        return Err(YcallrError::ParamValidation(
            "Multipart file path cannot contain '..'".into(),
        ));
    }

    let path = std::path::Path::new(file_path);

    if path.is_absolute() {
        let canonical = path.canonicalize().map_err(|e| {
            YcallrError::ParamValidation(format!(
                "Multipart file '{}' not found or not accessible: {}",
                file_path, e
            ))
        })?;
        let temp_dir = std::env::temp_dir();
        let temp_canonical = temp_dir.canonicalize().unwrap_or(temp_dir);
        if !canonical.starts_with(&temp_canonical) {
            return Err(YcallrError::ParamValidation(
                "Absolute multipart file paths must be under the system temp directory".into(),
            ));
        }
        return Ok(canonical);
    }

    let cwd = std::env::current_dir().map_err(|e| YcallrError::HttpClient(e.to_string()))?;
    let full = cwd.join(path);
    let canonical = full.canonicalize().map_err(|e| {
        YcallrError::ParamValidation(format!(
            "Multipart file '{}' not found or not accessible: {}",
            file_path, e
        ))
    })?;
    let cwd_canonical = cwd.canonicalize().unwrap_or(cwd);
    if !canonical.starts_with(&cwd_canonical) {
        return Err(YcallrError::ParamValidation(
            "Multipart file path escapes the current working directory".into(),
        ));
    }

    Ok(canonical)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ApiDefinition, HttpMethod, ParamType, Parameter};
    use std::collections::HashMap;

    #[test]
    fn test_append_query_pairs() {
        let url = append_query_pairs(
            "https://api.test.com/search",
            &[
                ("q".to_string(), "hello world".to_string()),
                ("page".to_string(), "2".to_string()),
            ],
        );
        assert_eq!(url, "https://api.test.com/search?q=hello%20world&page=2");
    }

    #[test]
    fn test_prepare_http_request_get() {
        let mut params_map = HashMap::new();
        params_map.insert(
            "owner".to_string(),
            Parameter {
                description: "owner".to_string(),
                param_type: ParamType::String,
                required: true,
            },
        );
        params_map.insert(
            "repo".to_string(),
            Parameter {
                description: "repo".to_string(),
                param_type: ParamType::String,
                required: true,
            },
        );

        let mut commands = HashMap::new();
        commands.insert(
            "get-repo".to_string(),
            crate::models::Command {
                description: None,
                endpoint: Some("/repos/{owner}/{repo}".to_string()),
                method: Some(HttpMethod::GET),
                auth: None,
                headers: HashMap::new(),
                params: params_map,
                body: None,
                responses: None,
                commands: None,
            },
        );

        let ctx = ClientContext {
            api: ApiDefinition {
                name: "test".to_string(),
                version: "1".to_string(),
                description: "".to_string(),
                base_url: "https://api.test.com".to_string(),
                env: vec![],
                auth: HashMap::new(),
                commands,
            },
            auth: None,
            auth_configs: HashMap::new(),
            env_vars: HashMap::new(),
        };

        let call_params = HashMap::from([
            ("owner".to_string(), "rust-lang".to_string()),
            ("repo".to_string(), "rust".to_string()),
        ]);

        let prepared = prepare_http_request(&ctx, "get-repo", &call_params, None).unwrap();
        assert_eq!(prepared.method, HttpMethod::GET);
        assert_eq!(prepared.url, "https://api.test.com/repos/rust-lang/rust");
        assert!(matches!(prepared.body, PreparedBody::None));
    }

    #[test]
    fn test_prepare_multipart_text_and_file() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("part.bin");
        std::fs::write(&file_path, b"data").unwrap();
        let canonical = file_path.canonicalize().unwrap().to_string_lossy().to_string();

        let mut commands = HashMap::new();
        commands.insert(
            "upload".to_string(),
            crate::models::Command {
                description: None,
                endpoint: Some("/upload".to_string()),
                method: Some(HttpMethod::POST),
                auth: None,
                headers: HashMap::new(),
                params: HashMap::new(),
                body: Some(crate::models::BodyConfig {
                    json: None,
                    form: None,
                    raw: None,
                    multipart: Some(vec![
                        crate::models::MultipartField {
                            name: "note".to_string(),
                            text: Some("hi".to_string()),
                            file: None,
                        },
                        crate::models::MultipartField {
                            name: "file".to_string(),
                            text: None,
                            file: Some(canonical),
                        },
                    ]),
                }),
                responses: None,
                commands: None,
            },
        );

        let ctx = ClientContext {
            api: ApiDefinition {
                name: "test".to_string(),
                version: "1".to_string(),
                description: "".to_string(),
                base_url: "https://api.test.com".to_string(),
                env: vec![],
                auth: HashMap::new(),
                commands,
            },
            auth: None,
            auth_configs: HashMap::new(),
            env_vars: HashMap::new(),
        };

        let prepared = prepare_http_request(&ctx, "upload", &HashMap::new(), None).unwrap();
        match &prepared.body {
            PreparedBody::MultipartNative(parts) => assert_eq!(parts.len(), 2),
            other => panic!("expected multipart body, got {:?}", other),
        }
    }

    #[test]
    fn test_prepare_http_auth_generic_scheme() {
        let mut commands = HashMap::new();
        commands.insert(
            "get".to_string(),
            crate::models::Command {
                description: None,
                endpoint: Some("/get".to_string()),
                method: Some(HttpMethod::GET),
                auth: None,
                headers: HashMap::new(),
                params: HashMap::new(),
                body: None,
                responses: None,
                commands: None,
            },
        );

        let ctx = ClientContext {
            api: ApiDefinition {
                name: "test".to_string(),
                version: "1".to_string(),
                description: "".to_string(),
                base_url: "https://api.test.com".to_string(),
                env: vec![],
                auth: HashMap::new(),
                commands,
            },
            auth: Some(crate::models::AuthConfig::Http {
                scheme: "Digest".to_string(),
                token: Some("secret".to_string()),
                username: None,
                password: None,
                prefix: None,
            }),
            auth_configs: HashMap::new(),
            env_vars: HashMap::new(),
        };

        let prepared = prepare_http_request(&ctx, "get", &HashMap::new(), None).unwrap();
        assert_eq!(
            prepared.headers.get("Authorization").map(String::as_str),
            Some("DIGEST secret")
        );
    }
}
