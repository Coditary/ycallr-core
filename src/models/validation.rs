use std::collections::HashMap;

use crate::YcallrError;

use super::command::command_auth_is_none;
use super::{ApiDefinition, AuthConfig, BodyConfig, Command};

/// Full validation for YAML/proto ingestion (includes blocked-host checks).
pub fn validate_api(api: &ApiDefinition) -> crate::Result<()> {
    validate_api_inner(api, true)
}

/// Validation when constructing a runtime HTTP client (allows loopback base URLs).
pub fn validate_api_for_client(api: &ApiDefinition) -> crate::Result<()> {
    validate_api_inner(api, false)
}

fn validate_api_inner(api: &ApiDefinition, block_sensitive_hosts: bool) -> crate::Result<()> {
    if api.name.is_empty() {
        return Err(YcallrError::InvalidDefinition(
            "API name cannot be empty".into(),
        ));
    }
    if !api.name.chars().all(|c| c.is_alphanumeric() || c == '-') {
        return Err(YcallrError::InvalidDefinition(
            "API name must be alphanumeric or dash".into(),
        ));
    }
    if api.base_url.is_empty() {
        return Err(YcallrError::InvalidDefinition(
            "Base URL cannot be empty".into(),
        ));
    }

    validate_base_url(&api.base_url, block_sensitive_hosts)?;

    for (name, auth) in &api.auth {
        validate_auth_config(name, auth)?;
    }

    validate_env_vars(&api.env)?;

    for (name, cmd) in &api.commands {
        validate_command(name, cmd, &api.auth)?;
    }

    Ok(())
}

fn validate_base_url(base_url: &str, block_sensitive_hosts: bool) -> crate::Result<()> {
    let lower = base_url.to_lowercase();
    if !lower.starts_with("http://") && !lower.starts_with("https://") {
        return Err(YcallrError::InvalidDefinition(
            "Base URL must use http:// or https://".into(),
        ));
    }

    let after_scheme = base_url
        .split("//")
        .nth(1)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| YcallrError::InvalidDefinition("Base URL is missing a host".into()))?;

    let authority = after_scheme.split('/').next().unwrap_or("");
    if authority.is_empty() {
        return Err(YcallrError::InvalidDefinition(
            "Base URL is missing a host".into(),
        ));
    }

    let host = authority
        .rsplit('@')
        .next()
        .unwrap_or(authority)
        .split(':')
        .next()
        .unwrap_or(authority)
        .trim()
        .to_lowercase();

    if host.is_empty() {
        return Err(YcallrError::InvalidDefinition(
            "Base URL is missing a host".into(),
        ));
    }

    if block_sensitive_hosts && is_blocked_host(&host) {
        return Err(YcallrError::InvalidDefinition(format!(
            "Base URL host '{}' is not allowed",
            host
        )));
    }

    Ok(())
}

fn is_blocked_host(host: &str) -> bool {
    let host = host.trim();
    let host = host
        .strip_prefix('[')
        .and_then(|h| h.strip_suffix(']'))
        .unwrap_or(host);

    let lower = host.to_ascii_lowercase();

    if matches!(
        lower.as_str(),
        "localhost" | "127.0.0.1" | "0.0.0.0" | "::1" | "metadata.google.internal" | "metadata"
    ) || lower.starts_with("127.")
        || lower == "169.254.169.254"
        || lower.starts_with("169.254.")
    {
        return true;
    }

    if is_private_ipv4(&lower) {
        return true;
    }

    is_private_ipv6(&lower)
}

fn is_private_ipv4(host: &str) -> bool {
    let parts: Vec<u8> = host.split('.').filter_map(|p| p.parse().ok()).collect();
    if parts.len() != 4 {
        return false;
    }

    let [a, b, _, _] = [parts[0], parts[1], parts[2], parts[3]];
    a == 10
        || a == 127
        || a == 0
        || (a == 172 && (16..=31).contains(&b))
        || (a == 192 && b == 168)
        || (a == 169 && b == 254)
}

fn is_private_ipv6(host: &str) -> bool {
    let lower = host.to_ascii_lowercase();
    lower.starts_with("fe80:")
        || lower.starts_with("fc")
        || lower.starts_with("fd")
        || lower == "::1"
}

fn validate_command(
    path: &str,
    cmd: &Command,
    auth_configs: &HashMap<String, AuthConfig>,
) -> crate::Result<()> {
    let has_endpoint = cmd.endpoint.is_some();
    let has_method = cmd.method.is_some();

    if has_endpoint != has_method {
        return Err(YcallrError::InvalidDefinition(format!(
            "Command '{}': endpoint and method must both be set for callable commands",
            path
        )));
    }

    if let Some(auth_name) = &cmd.auth {
        if !command_auth_is_none(auth_name) && !auth_configs.contains_key(auth_name) {
            return Err(YcallrError::InvalidDefinition(format!(
                "Command '{}': auth config '{}' not found",
                path, auth_name
            )));
        }
    }

    if let Some(body) = &cmd.body {
        validate_body_config(path, body)?;
    }

    if let Some(children) = &cmd.commands {
        for (child_name, child) in children {
            let child_path = format!("{}.{}", path, child_name);
            validate_command(&child_path, child, auth_configs)?;
        }
    }

    Ok(())
}

fn body_config_kinds(body: &BodyConfig) -> Vec<&'static str> {
    body.body_kinds()
}

pub(crate) fn validate_body_config(path: &str, body: &BodyConfig) -> crate::Result<()> {
    let kinds = body_config_kinds(body);
    if kinds.len() > 1 {
        return Err(YcallrError::InvalidDefinition(format!(
            "Command '{}': body must specify only one of json, form, raw, or multipart (found: {})",
            path,
            kinds.join(", ")
        )));
    }

    if let Some(fields) = &body.multipart {
        for (index, field) in fields.iter().enumerate() {
            if field.name.trim().is_empty() {
                return Err(YcallrError::InvalidDefinition(format!(
                    "Command '{}': multipart field at index {} must have a non-empty name",
                    path, index
                )));
            }

            let has_text = field.text.as_ref().is_some_and(|t| !t.is_empty());
            let has_file = field.file.as_ref().is_some_and(|f| !f.is_empty());

            if !has_text && !has_file {
                return Err(YcallrError::InvalidDefinition(format!(
                    "Command '{}': multipart field '{}' must specify text or file",
                    path, field.name
                )));
            }

            if has_text && has_file {
                return Err(YcallrError::InvalidDefinition(format!(
                    "Command '{}': multipart field '{}' cannot specify both text and file",
                    path, field.name
                )));
            }
        }
    }

    Ok(())
}

fn validate_env_vars(env: &[super::EnvVar]) -> crate::Result<()> {
    let mut seen = std::collections::HashSet::new();
    for env_var in env {
        if env_var.name.trim().is_empty() {
            return Err(YcallrError::InvalidDefinition(
                "Environment variable name cannot be empty".into(),
            ));
        }

        if !env_var
            .name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            return Err(YcallrError::InvalidDefinition(format!(
                "Environment variable '{}' must be alphanumeric or underscore",
                env_var.name
            )));
        }

        if seen.contains(&env_var.name) {
            return Err(YcallrError::InvalidDefinition(format!(
                "Duplicate environment variable '{}'",
                env_var.name
            )));
        }
        seen.insert(env_var.name.clone());
    }
    Ok(())
}

pub(crate) fn validate_auth_config(name: &str, auth: &AuthConfig) -> crate::Result<()> {
    match auth {
        AuthConfig::Bearer { token } => {
            if token.trim().is_empty() {
                return Err(YcallrError::InvalidDefinition(format!(
                    "Auth config '{}': bearer token cannot be empty",
                    name
                )));
            }
        }
        AuthConfig::ApiKey {
            key,
            name: key_name,
            ..
        } => {
            if key.trim().is_empty() {
                return Err(YcallrError::InvalidDefinition(format!(
                    "Auth config '{}': api_key key cannot be empty",
                    name
                )));
            }
            if key_name.trim().is_empty() {
                return Err(YcallrError::InvalidDefinition(format!(
                    "Auth config '{}': api_key name cannot be empty",
                    name
                )));
            }
        }
        AuthConfig::Http {
            scheme,
            token,
            username,
            password,
            prefix,
        } => match scheme.to_lowercase().as_str() {
            "basic" => {
                if username.as_ref().is_none_or(|u| u.trim().is_empty()) {
                    return Err(YcallrError::InvalidDefinition(format!(
                        "Auth config '{}': http basic auth requires username",
                        name
                    )));
                }
                if password.as_ref().is_none_or(|p| p.trim().is_empty()) {
                    return Err(YcallrError::InvalidDefinition(format!(
                        "Auth config '{}': http basic auth requires password",
                        name
                    )));
                }
            }
            "bearer" => {
                if token.as_ref().is_none_or(|t| t.trim().is_empty()) {
                    return Err(YcallrError::InvalidDefinition(format!(
                        "Auth config '{}': http bearer auth requires token",
                        name
                    )));
                }
            }
            "custom" => {
                if prefix.as_ref().is_none_or(|p| p.trim().is_empty()) {
                    return Err(YcallrError::InvalidDefinition(format!(
                        "Auth config '{}': http custom auth requires prefix",
                        name
                    )));
                }
                if token.as_ref().is_none_or(|t| t.trim().is_empty()) {
                    return Err(YcallrError::InvalidDefinition(format!(
                        "Auth config '{}': http custom auth requires token",
                        name
                    )));
                }
            }
            _ => {
                if token.as_ref().is_none_or(|t| t.trim().is_empty()) {
                    return Err(YcallrError::InvalidDefinition(format!(
                        "Auth config '{}': http scheme '{}' requires token",
                        name, scheme
                    )));
                }
            }
        },
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ApiDefinition, BodyConfig, HttpMethod, COMMAND_AUTH_NONE};
    use std::collections::HashMap;

    #[test]
    fn test_validate_base_url_blocks_loopback() {
        assert!(validate_base_url("http://127.0.0.1", true).is_err());
        assert!(validate_base_url("https://localhost/api", true).is_err());
        assert!(validate_base_url("http://169.254.169.254", true).is_err());
        assert!(validate_base_url("http://127.0.0.1", false).is_ok());
    }

    #[test]
    fn test_validate_base_url_blocks_private_ipv4() {
        assert!(validate_base_url("http://10.0.0.1", true).is_err());
        assert!(validate_base_url("http://192.168.1.1/api", true).is_err());
        assert!(validate_base_url("http://172.16.0.1", true).is_err());
        assert!(validate_base_url("http://10.0.0.1", false).is_ok());
    }

    #[test]
    fn test_validate_base_url_allows_public_host() {
        assert!(validate_base_url("https://api.github.com", true).is_ok());
    }

    #[test]
    fn test_validate_api_for_client_allows_loopback() {
        let api = ApiDefinition {
            name: "test".to_string(),
            version: "1.0.0".to_string(),
            description: "".to_string(),
            base_url: "http://127.0.0.1:8080".to_string(),
            env: vec![],
            auth: HashMap::new(),
            commands: HashMap::new(),
        };
        assert!(validate_api_for_client(&api).is_ok());
        assert!(validate_api(&api).is_err());
    }

    #[test]
    fn test_validate_command_auth_none_is_allowed() {
        let mut commands = HashMap::new();
        commands.insert(
            "public".to_string(),
            Command {
                description: None,
                endpoint: Some("/health".to_string()),
                method: Some(HttpMethod::GET),
                auth: Some(COMMAND_AUTH_NONE.to_string()),
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
            description: "".to_string(),
            base_url: "https://api.test.com".to_string(),
            env: vec![],
            auth: HashMap::new(),
            commands,
        };

        assert!(validate_api(&api).is_ok());
    }

    #[test]
    fn test_validate_command_auth_reference() {
        let mut commands = HashMap::new();
        commands.insert(
            "get".to_string(),
            Command {
                description: None,
                endpoint: Some("/x".to_string()),
                method: Some(HttpMethod::GET),
                auth: Some("missing".to_string()),
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
            description: "".to_string(),
            base_url: "https://api.test.com".to_string(),
            env: vec![],
            auth: HashMap::new(),
            commands,
        };

        assert!(validate_api(&api).is_err());
    }

    #[test]
    fn test_validate_command_endpoint_method_mismatch() {
        let mut commands = HashMap::new();
        commands.insert(
            "broken".to_string(),
            Command {
                description: None,
                endpoint: Some("/x".to_string()),
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
            description: "".to_string(),
            base_url: "https://api.test.com".to_string(),
            env: vec![],
            auth: HashMap::new(),
            commands,
        };

        let err = validate_api(&api).unwrap_err().to_string();
        assert!(err.contains("endpoint and method"));
    }

    #[test]
    fn test_validate_auth_config_basic_requires_credentials() {
        let auth = AuthConfig::Http {
            scheme: "basic".to_string(),
            token: None,
            username: None,
            password: Some("pass".to_string()),
            prefix: None,
        };
        assert!(validate_auth_config("basic", &auth).is_err());
    }

    #[test]
    fn test_validate_auth_config_bearer_ok_with_template() {
        let auth = AuthConfig::Bearer {
            token: "${TOKEN}".to_string(),
        };
        assert!(validate_auth_config("primary", &auth).is_ok());
    }

    #[test]
    fn test_validate_body_config_rejects_multiple_types() {
        let body = BodyConfig {
            json: Some(serde_json::json!({"key": "value"})),
            form: None,
            raw: Some("fallback".to_string()),
            multipart: None,
        };
        let err = validate_body_config("create", &body)
            .unwrap_err()
            .to_string();
        assert!(err.contains("only one of json, form, raw, or multipart"));
        assert!(err.contains("json"));
        assert!(err.contains("raw"));
    }

    #[test]
    fn test_validate_body_config_allows_single_type() {
        let body = BodyConfig {
            json: Some(serde_json::json!({"key": "value"})),
            form: None,
            raw: None,
            multipart: None,
        };
        assert!(validate_body_config("create", &body).is_ok());
    }

    #[test]
    fn test_validate_body_config_ignores_empty_secondary_fields() {
        let body = BodyConfig {
            json: Some(serde_json::json!({"key": "value"})),
            form: Some(HashMap::new()),
            raw: Some("".to_string()),
            multipart: Some(vec![]),
        };
        assert!(validate_body_config("create", &body).is_ok());
    }

    #[test]
    fn test_validate_multipart_field_requires_text_or_file() {
        let body = BodyConfig {
            json: None,
            form: None,
            raw: None,
            multipart: Some(vec![crate::models::MultipartField {
                name: "file".to_string(),
                text: None,
                file: None,
            }]),
        };
        let err = validate_body_config("upload", &body)
            .unwrap_err()
            .to_string();
        assert!(err.contains("must specify text or file"));
    }

    #[test]
    fn test_validate_env_duplicate_name_rejected() {
        let api = ApiDefinition {
            name: "test".to_string(),
            version: "1.0.0".to_string(),
            description: "".to_string(),
            base_url: "https://api.test.com".to_string(),
            env: vec![
                crate::models::EnvVar {
                    name: "TOKEN".to_string(),
                    required: true,
                },
                crate::models::EnvVar {
                    name: "TOKEN".to_string(),
                    required: false,
                },
            ],
            auth: HashMap::new(),
            commands: HashMap::new(),
        };
        assert!(validate_api(&api).is_err());
    }

    #[test]
    fn test_validate_auth_http_digest_requires_token() {
        let mut auth = HashMap::new();
        auth.insert(
            "digest".to_string(),
            AuthConfig::Http {
                scheme: "Digest".to_string(),
                token: None,
                username: None,
                password: None,
                prefix: None,
            },
        );
        let err = validate_auth_config("digest", auth.get("digest").unwrap())
            .unwrap_err()
            .to_string();
        assert!(err.contains("requires token"));
    }
}
