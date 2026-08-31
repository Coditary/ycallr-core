use std::collections::{HashMap, HashSet};

use percent_encoding::{utf8_percent_encode, AsciiSet, NON_ALPHANUMERIC};

/// RFC 3986 unreserved characters in path segments: ALPHA / DIGIT / "-" / "." / "_" / "~"
const PATH_SEGMENT_ENCODE_SET: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'.')
    .remove(b'_')
    .remove(b'~');

use crate::models::{Command, HttpMethod, ParamType};

/// YAML value `auth: none` — explicit opt-out from global client auth.
pub const COMMAND_AUTH_NONE: &str = "none";

pub fn command_auth_is_none(auth: &str) -> bool {
    auth.eq_ignore_ascii_case(COMMAND_AUTH_NONE)
}

impl Command {
    pub fn endpoint_param_names(&self) -> Vec<String> {
        let endpoint = self.endpoint.as_deref().unwrap_or("");
        let mut names = Vec::new();
        let mut rest = endpoint;
        while let Some(start) = rest.find('{') {
            if let Some(end) = rest[start + 1..].find('}') {
                names.push(rest[start + 1..start + 1 + end].to_string());
                rest = &rest[start + 1 + end + 1..];
            } else {
                break;
            }
        }
        names
    }

    /// Unique path placeholder names from the endpoint (e.g. `{owner}` in `/repos/{owner}/{repo}`).
    pub fn endpoint_path_param_names(&self) -> Vec<String> {
        let mut seen = HashSet::new();
        let mut unique = Vec::new();
        for name in self.endpoint_param_names() {
            if seen.insert(name.clone()) {
                unique.push(name);
            }
        }
        unique
    }

    pub fn validate_params(
        &self,
        params: &HashMap<String, String>,
        body: Option<&serde_json::Value>,
    ) -> crate::Result<()> {
        for (name, param) in &self.params {
            let value = param_value_from_call(name, params, body);

            if param.required && value.is_none() {
                return Err(crate::YcallrError::ParamValidation(format!(
                    "Missing required parameter '{}'",
                    name
                )));
            }

            if let Some(value) = value {
                if !value.trim().is_empty() {
                    validate_param_type(name, &param.param_type, &value)?;
                }
            }
        }

        let path_params = self.endpoint_path_param_names();

        for key in &path_params {
            if !self.params.contains_key(key) {
                let value = param_value_from_call(key, params, body);
                if value.is_none() {
                    return Err(crate::YcallrError::ParamValidation(format!(
                        "Missing required path parameter '{}'",
                        key
                    )));
                }
                let value = value.unwrap();
                if value.trim().is_empty() {
                    return Err(crate::YcallrError::ParamValidation(format!(
                        "Missing required path parameter '{}'",
                        key
                    )));
                }
                validate_param_type(key, &ParamType::String, &value)?;
            }
        }

        for key in params.keys() {
            if !self.params.contains_key(key) && !path_params.contains(key) {
                return Err(crate::YcallrError::ParamValidation(format!(
                    "Unknown parameter '{}'",
                    key
                )));
            }
        }

        Ok(())
    }

    pub fn resolve_endpoint(&self, params: &HashMap<String, String>) -> crate::Result<String> {
        let endpoint = self
            .endpoint
            .as_deref()
            .ok_or_else(|| crate::YcallrError::ParamValidation("Command has no endpoint".into()))?;

        let mut resolved = endpoint.to_string();
        let path_params = self.endpoint_path_param_names();
        let mut missing = Vec::new();

        for key in &path_params {
            match params.get(key) {
                Some(value) if !value.trim().is_empty() => {
                    let encoded = encode_path_segment(value);
                    resolved = resolved.replace(&format!("{{{}}}", key), &encoded);
                }
                _ => missing.push(key.clone()),
            }
        }

        if !missing.is_empty() {
            return Err(crate::YcallrError::ParamValidation(format!(
                "Missing path parameters: {}",
                missing.join(", ")
            )));
        }

        if resolved.contains('{') {
            return Err(crate::YcallrError::ParamValidation(format!(
                "Unresolved parameters in endpoint: {}",
                resolved
            )));
        }

        Ok(resolved)
    }

    pub fn get_command_recursive(&self, parts: &[&str]) -> crate::Result<&Command> {
        let name = parts[0];
        let commands = self.commands.as_ref().ok_or_else(|| {
            crate::YcallrError::CommandNotFound(format!("{} has no sub-commands", name))
        })?;

        let cmd = commands
            .get(name)
            .ok_or_else(|| crate::YcallrError::CommandNotFound(name.into()))?;

        if parts.len() == 1 {
            Ok(cmd)
        } else {
            cmd.get_command_recursive(&parts[1..])
        }
    }

    pub fn is_leaf(&self) -> bool {
        self.endpoint.is_some() && self.method.is_some()
    }

    pub fn is_branch(&self) -> bool {
        self.commands.is_some() && !self.commands.as_ref().unwrap().is_empty()
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

fn encode_path_segment(value: &str) -> String {
    utf8_percent_encode(value, PATH_SEGMENT_ENCODE_SET).to_string()
}

fn param_value_from_call(
    name: &str,
    params: &HashMap<String, String>,
    body: Option<&serde_json::Value>,
) -> Option<String> {
    if let Some(value) = params.get(name) {
        if !value.trim().is_empty() {
            return Some(value.clone());
        }
    }

    body.and_then(|body| body.get(name))
        .and_then(json_value_to_param_string)
}

fn json_value_to_param_string(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::String(s) if !s.trim().is_empty() => Some(s.clone()),
        serde_json::Value::Number(n) => Some(n.to_string()),
        serde_json::Value::Bool(b) => Some(b.to_string()),
        serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
            serde_json::to_string(value).ok()
        }
        serde_json::Value::Null | serde_json::Value::String(_) => None,
    }
}

fn validate_param_type(name: &str, param_type: &ParamType, value: &str) -> crate::Result<()> {
    match param_type {
        ParamType::String => Ok(()),
        ParamType::Number => {
            if value.parse::<f64>().is_err() {
                Err(crate::YcallrError::ParamValidation(format!(
                    "Parameter '{}' must be a number, got '{}'",
                    name, value
                )))
            } else {
                Ok(())
            }
        }
        ParamType::Boolean => {
            let normalized = value.trim().to_ascii_lowercase();
            if matches!(
                normalized.as_str(),
                "true" | "false" | "1" | "0" | "yes" | "no"
            ) {
                Ok(())
            } else {
                Err(crate::YcallrError::ParamValidation(format!(
                    "Parameter '{}' must be a boolean (true/false), got '{}'",
                    name, value
                )))
            }
        }
        ParamType::Array => match serde_json::from_str::<serde_json::Value>(value) {
            Ok(v) if v.is_array() => Ok(()),
            _ => Err(crate::YcallrError::ParamValidation(format!(
                "Parameter '{}' must be a JSON array, got '{}'",
                name, value
            ))),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Parameter;

    fn param(name: &str, param_type: ParamType, required: bool) -> Parameter {
        Parameter {
            description: name.to_string(),
            param_type,
            required,
        }
    }

    fn command_with_params(params: HashMap<String, Parameter>) -> Command {
        command_with_endpoint("/items", params)
    }

    fn command_with_path_params(params: HashMap<String, Parameter>) -> Command {
        command_with_endpoint("/items/{id}", params)
    }

    fn command_with_endpoint(endpoint: &str, params: HashMap<String, Parameter>) -> Command {
        Command {
            description: None,
            endpoint: Some(endpoint.to_string()),
            method: Some(HttpMethod::GET),
            auth: None,
            headers: HashMap::new(),
            params,
            body: None,
            responses: None,
            commands: None,
        }
    }

    #[test]
    fn test_validate_params_required_missing() {
        let mut params = HashMap::new();
        params.insert("id".to_string(), param("id", ParamType::String, true));
        params.insert("id".to_string(), param("id", ParamType::String, true));
        let cmd = command_with_path_params(params);

        let result = cmd.validate_params(&HashMap::new(), None);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Missing required parameter 'id'"));
    }

    #[test]
    fn test_validate_params_required_empty() {
        let mut params = HashMap::new();
        params.insert("id".to_string(), param("id", ParamType::String, true));
        params.insert("id".to_string(), param("id", ParamType::String, true));
        let cmd = command_with_path_params(params);

        let call_params = HashMap::from([("id".to_string(), "   ".to_string())]);
        let result = cmd.validate_params(&call_params, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_params_implicit_path_param_without_yaml_declaration() {
        let cmd = Command {
            description: None,
            endpoint: Some("/users/{username}".to_string()),
            method: Some(HttpMethod::GET),
            auth: None,
            headers: HashMap::new(),
            params: HashMap::new(),
            body: None,
            responses: None,
            commands: None,
        };

        let call_params = HashMap::from([("username".to_string(), "octocat".to_string())]);
        assert!(cmd.validate_params(&call_params, None).is_ok());

        let resolved = cmd.resolve_endpoint(&call_params).unwrap();
        assert_eq!(resolved, "/users/octocat");
    }

    #[test]
    fn test_validate_params_implicit_path_param_missing() {
        let cmd = Command {
            description: None,
            endpoint: Some("/users/{username}".to_string()),
            method: Some(HttpMethod::GET),
            auth: None,
            headers: HashMap::new(),
            params: HashMap::new(),
            body: None,
            responses: None,
            commands: None,
        };

        let result = cmd.validate_params(&HashMap::new(), None);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Missing required path parameter 'username'"));
    }

    #[test]
    fn test_endpoint_path_param_names_unique() {
        let cmd = Command {
            description: None,
            endpoint: Some("/orgs/{owner}/repos/{owner}/{repo}".to_string()),
            method: Some(HttpMethod::GET),
            auth: None,
            headers: HashMap::new(),
            params: HashMap::new(),
            body: None,
            responses: None,
            commands: None,
        };

        assert_eq!(
            cmd.endpoint_path_param_names(),
            vec!["owner".to_string(), "repo".to_string()]
        );
    }

    #[test]
    fn test_resolve_endpoint_missing_path_params_lists_names() {
        let cmd = Command {
            description: None,
            endpoint: Some("/repos/{owner}/{repo}/issues".to_string()),
            method: Some(HttpMethod::GET),
            auth: None,
            headers: HashMap::new(),
            params: HashMap::new(),
            body: None,
            responses: None,
            commands: None,
        };

        let result = cmd.resolve_endpoint(&HashMap::new());
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Missing path parameters"));
        assert!(err.contains("owner"));
        assert!(err.contains("repo"));
    }

    #[test]
    fn test_validate_params_unknown() {
        let cmd = command_with_params(HashMap::new());
        let call_params = HashMap::from([("extra".to_string(), "x".to_string())]);
        let result = cmd.validate_params(&call_params, None);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Unknown parameter 'extra'"));
    }

    #[test]
    fn test_validate_params_types() {
        let mut params = HashMap::new();
        params.insert("num".to_string(), param("num", ParamType::Number, false));
        params.insert("flag".to_string(), param("flag", ParamType::Boolean, false));
        params.insert("tags".to_string(), param("tags", ParamType::Array, false));
        let cmd = command_with_params(params);

        assert!(cmd
            .validate_params(
                &HashMap::from([
                    ("num".to_string(), "42".to_string()),
                    ("flag".to_string(), "true".to_string()),
                    ("tags".to_string(), "[\"a\",\"b\"]".to_string()),
                ]),
                None,
            )
            .is_ok());

        assert!(cmd
            .validate_params(
                &HashMap::from([("num".to_string(), "not-a-number".to_string())]),
                None,
            )
            .is_err());
        assert!(cmd
            .validate_params(
                &HashMap::from([("flag".to_string(), "maybe".to_string())]),
                None,
            )
            .is_err());
        assert!(cmd
            .validate_params(
                &HashMap::from([("tags".to_string(), "not-json".to_string())]),
                None,
            )
            .is_err());
    }

    #[test]
    fn test_resolve_endpoint_encodes_special_chars() {
        let mut params = HashMap::new();
        params.insert("owner".to_string(), param("owner", ParamType::String, true));
        params.insert("repo".to_string(), param("repo", ParamType::String, true));
        let cmd = Command {
            description: None,
            endpoint: Some("/repos/{owner}/{repo}/issues".to_string()),
            method: Some(HttpMethod::GET),
            auth: None,
            headers: HashMap::new(),
            params,
            body: None,
            responses: None,
            commands: None,
        };

        let call_params = HashMap::from([
            ("owner".to_string(), "foo&bar".to_string()),
            ("repo".to_string(), "a/b".to_string()),
        ]);
        let resolved = cmd.resolve_endpoint(&call_params).unwrap();
        assert_eq!(resolved, "/repos/foo%26bar/a%2Fb/issues");
    }

    #[test]
    fn test_validate_params_required_from_body() {
        let mut params = HashMap::new();
        params.insert("title".to_string(), param("title", ParamType::String, true));
        let cmd = command_with_params(params);

        let body = serde_json::json!({"title": "Issue"});
        assert!(cmd.validate_params(&HashMap::new(), Some(&body)).is_ok());
    }

    #[test]
    fn test_endpoint_param_names_and_duplicate_replacement() {
        let cmd = Command {
            description: None,
            endpoint: Some("/orgs/{owner}/repos/{owner}/{repo}".to_string()),
            method: Some(HttpMethod::GET),
            auth: None,
            headers: HashMap::new(),
            params: HashMap::new(),
            body: None,
            responses: None,
            commands: None,
        };

        assert_eq!(
            cmd.endpoint_param_names(),
            vec!["owner".to_string(), "owner".to_string(), "repo".to_string()]
        );

        let resolved = cmd
            .resolve_endpoint(&HashMap::from([
                ("owner".to_string(), "rust-lang".to_string()),
                ("repo".to_string(), "rust".to_string()),
            ]))
            .unwrap();
        assert_eq!(resolved, "/orgs/rust-lang/repos/rust-lang/rust");
    }

    #[test]
    fn test_get_command_recursive_past_leaf() {
        let leaf = Command {
            description: None,
            endpoint: Some("/leaf".to_string()),
            method: Some(HttpMethod::GET),
            auth: None,
            headers: HashMap::new(),
            params: HashMap::new(),
            body: None,
            responses: None,
            commands: None,
        };

        let cmd = Command {
            description: None,
            endpoint: None,
            method: None,
            auth: None,
            headers: HashMap::new(),
            params: HashMap::new(),
            body: None,
            responses: None,
            commands: Some(HashMap::from([("leaf".to_string(), leaf)])),
        };

        let err = cmd.get_command_recursive(&["leaf", "extra"]).unwrap_err();
        assert!(err.to_string().contains("extra has no sub-commands"));
    }

    #[test]
    fn test_resolve_endpoint_preserves_unreserved_chars() {
        let cmd = Command {
            description: None,
            endpoint: Some("/repos/{owner}/{repo}".to_string()),
            method: Some(HttpMethod::GET),
            auth: None,
            headers: HashMap::new(),
            params: HashMap::new(),
            body: None,
            responses: None,
            commands: None,
        };

        let resolved = cmd
            .resolve_endpoint(&HashMap::from([
                ("owner".to_string(), "foo-bar_baz.rust~".to_string()),
                ("repo".to_string(), "my-repo".to_string()),
            ]))
            .unwrap();
        assert_eq!(resolved, "/repos/foo-bar_baz.rust~/my-repo");
    }

    #[test]
    fn test_validate_params_boolean_aliases() {
        let mut params = HashMap::new();
        params.insert("flag".to_string(), param("flag", ParamType::Boolean, false));
        let cmd = command_with_params(params);

        for value in ["yes", "no", "1", "0", "TRUE", "False"] {
            assert!(
                cmd.validate_params(
                    &HashMap::from([("flag".to_string(), value.to_string())]),
                    None
                )
                .is_ok(),
                "expected '{}' to be accepted",
                value
            );
        }
    }

    #[test]
    fn test_validate_params_body_number_satisfies_string_param() {
        let mut params = HashMap::new();
        params.insert("id".to_string(), param("id", ParamType::String, true));
        let cmd = command_with_path_params(params);

        let body = serde_json::json!({"id": 42});
        assert!(cmd.validate_params(&HashMap::new(), Some(&body)).is_ok());
    }

    #[test]
    fn test_is_leaf_requires_endpoint_and_method() {
        let endpoint_only = Command {
            description: None,
            endpoint: Some("/path".to_string()),
            method: None,
            auth: None,
            headers: HashMap::new(),
            params: HashMap::new(),
            body: None,
            responses: None,
            commands: None,
        };
        assert!(!endpoint_only.is_leaf());
        assert!(!endpoint_only.is_callable());

        let method_only = Command {
            description: None,
            endpoint: None,
            method: Some(HttpMethod::GET),
            auth: None,
            headers: HashMap::new(),
            params: HashMap::new(),
            body: None,
            responses: None,
            commands: None,
        };
        assert!(!method_only.is_leaf());
    }

    #[test]
    fn test_validate_params_array_rejects_object_json() {
        let mut params = HashMap::new();
        params.insert("tags".to_string(), param("tags", ParamType::Array, false));
        let cmd = command_with_params(params);

        let result = cmd.validate_params(
            &HashMap::from([("tags".to_string(), r#"{"not":"array"}"#.to_string())]),
            None,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("JSON array"));
    }
}
