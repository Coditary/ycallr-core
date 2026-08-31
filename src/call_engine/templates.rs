use crate::models::BodyConfig;
use regex::Regex;
use std::collections::HashMap;

pub fn resolve_env_vars(text: &str, env_vars: &HashMap<String, String>) -> crate::Result<String> {
    let re = Regex::new(r"\$\{([^}]+)\}").unwrap();
    let mut result = text.to_string();

    for cap in re.captures_iter(text) {
        let var_name = &cap[1];
        let replacement = env_vars.get(var_name).ok_or_else(|| {
            crate::YcallrError::EnvVar(format!(
                "Unknown environment variable '{}' in template",
                var_name
            ))
        })?;
        result = result.replace(&cap[0], replacement);
    }

    Ok(result)
}

pub fn resolve_response_template(
    template: &str,
    status: u16,
    params: &HashMap<String, String>,
    body: &serde_json::Value,
) -> String {
    let mut result = template.replace("{status}", &status.to_string());

    let re = Regex::new(r"\{(input|output)\.([^}]+)\}").unwrap();

    for cap in re.captures_iter(template) {
        let full_match = cap.get(0).map(|m| m.as_str()).unwrap_or("");
        let prefix = &cap[1];
        let field = &cap[2];

        let replacement = match prefix {
            "input" => params.get(field).cloned(),
            "output" => json_path(body, field).map(value_to_template_string),
            _ => None,
        };

        if let Some(repl) = replacement {
            result = result.replace(full_match, &repl);
        }
    }

    result
}

fn json_path<'a>(value: &'a serde_json::Value, path: &str) -> Option<&'a serde_json::Value> {
    let mut current = value;
    for segment in path.split('.') {
        if segment.is_empty() {
            return None;
        }
        current = if let Ok(index) = segment.parse::<usize>() {
            current.get(index)?
        } else {
            current.get(segment)?
        };
    }
    Some(current)
}

fn value_to_template_string(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Null => String::new(),
        other => other.to_string(),
    }
}

pub fn resolve_string_templates(text: &str, params: &HashMap<String, String>) -> String {
    let mut resolved = text.to_string();
    for (key, val) in params {
        resolved = resolved.replace(&format!("{{{}}}", key), val);
    }
    resolved
}

pub fn resolve_body(
    body_config: &BodyConfig,
    params: &HashMap<String, String>,
) -> crate::Result<BodyConfig> {
    Ok(BodyConfig {
        json: body_config
            .json
            .as_ref()
            .map(|v| resolve_json_templates(v, params))
            .transpose()?,
        form: body_config.form.as_ref().map(|m| {
            m.iter()
                .map(|(k, v)| {
                    (
                        resolve_string_templates(k, params),
                        resolve_string_templates(v, params),
                    )
                })
                .collect()
        }),
        multipart: body_config.multipart.as_ref().map(|fields| {
            fields
                .iter()
                .map(|f| crate::models::MultipartField {
                    name: resolve_string_templates(&f.name, params),
                    text: f.text.as_ref().map(|t| resolve_string_templates(t, params)),
                    file: f.file.clone(),
                })
                .collect()
        }),
        raw: body_config
            .raw
            .as_ref()
            .map(|r| resolve_string_templates(r, params)),
    })
}

pub fn resolve_json_templates(
    value: &serde_json::Value,
    params: &HashMap<String, String>,
) -> crate::Result<serde_json::Value> {
    match value {
        serde_json::Value::String(s) => {
            let mut resolved = s.clone();
            for (key, val) in params {
                resolved = resolved.replace(&format!("{{{}}}", key), val);
            }
            Ok(serde_json::Value::String(resolved))
        }
        serde_json::Value::Array(arr) => {
            let mut resolved = Vec::new();
            for item in arr {
                resolved.push(resolve_json_templates(item, params)?);
            }
            Ok(serde_json::Value::Array(resolved))
        }
        serde_json::Value::Object(map) => {
            let mut resolved = serde_json::Map::new();
            for (k, v) in map {
                resolved.insert(k.clone(), resolve_json_templates(v, params)?);
            }
            Ok(serde_json::Value::Object(resolved))
        }
        other => Ok(other.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_json_templates_array_and_object() {
        let params = HashMap::from([
            ("name".to_string(), "rust".to_string()),
            ("tag".to_string(), "1.0".to_string()),
        ]);
        let value = serde_json::json!({
            "items": ["{name}", {"v": "{tag}"}],
            "meta": {"name": "{name}"}
        });
        let resolved = resolve_json_templates(&value, &params).unwrap();
        assert_eq!(resolved["items"][0], "rust");
        assert_eq!(resolved["items"][1]["v"], "1.0");
        assert_eq!(resolved["meta"]["name"], "rust");
    }

    #[test]
    fn test_resolve_json_templates_non_string_passthrough() {
        let params = HashMap::new();
        let value = serde_json::json!([1, true, null]);
        let resolved = resolve_json_templates(&value, &params).unwrap();
        assert_eq!(resolved[0], 1);
        assert_eq!(resolved[1], true);
        assert!(resolved[2].is_null());
    }

    #[test]
    fn test_resolve_env_vars() {
        let mut env_vars = HashMap::new();
        env_vars.insert("TOKEN".to_string(), "abc123".to_string());
        let result = resolve_env_vars("Bearer ${TOKEN}", &env_vars).unwrap();
        assert_eq!(result, "Bearer abc123");
    }

    #[test]
    fn test_resolve_env_vars_unknown() {
        let env_vars = HashMap::new();
        let result = resolve_env_vars("${UNKNOWN}", &env_vars);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Unknown environment variable 'UNKNOWN'"));
    }

    #[test]
    fn test_resolve_env_vars_optional_empty() {
        let env_vars = HashMap::from([("OPTIONAL".to_string(), String::new())]);
        let result = resolve_env_vars("version=${OPTIONAL}", &env_vars).unwrap();
        assert_eq!(result, "version=");
    }

    #[test]
    fn test_resolve_response_template_input() {
        let mut params = HashMap::new();
        params.insert("owner".to_string(), "rust-lang".to_string());
        let body = serde_json::json!({});

        let result = resolve_response_template("{input.owner} not found", 404, &params, &body);
        assert_eq!(result, "rust-lang not found");
    }

    #[test]
    fn test_resolve_response_template_output() {
        let params = HashMap::new();
        let body = serde_json::json!({"name": "rust", "stars": 90000});

        let result = resolve_response_template(
            "Got repo {output.name} with {output.stars} stars",
            200,
            &params,
            &body,
        );
        assert_eq!(result, "Got repo rust with 90000 stars");
    }

    #[test]
    fn test_resolve_response_template_output_number_first() {
        let params = HashMap::new();
        let body = serde_json::json!({"name": "rust", "stars": 90000});

        let result = resolve_response_template(
            "{output.stars} stars for {output.name}",
            200,
            &params,
            &body,
        );
        assert_eq!(result, "90000 stars for rust");
    }

    #[test]
    fn test_resolve_response_template_mixed() {
        let mut params = HashMap::new();
        params.insert("owner".to_string(), "rust-lang".to_string());
        let body = serde_json::json!({"name": "rust"});

        let result = resolve_response_template("{input.owner}/{output.name}", 200, &params, &body);
        assert_eq!(result, "rust-lang/rust");
    }

    #[test]
    fn test_resolve_response_template_missing_field() {
        let params = HashMap::new();
        let body = serde_json::json!({"name": "rust"});

        let result = resolve_response_template("{output.missing}", 200, &params, &body);
        assert_eq!(result, "{output.missing}");
    }

    #[test]
    fn test_resolve_response_template_nested_output() {
        let params = HashMap::new();
        let body = serde_json::json!({
            "user": { "name": "rust", "id": 42 },
            "meta": { "count": 3 }
        });

        let result = resolve_response_template(
            "User {output.user.name} (#{output.user.id})",
            200,
            &params,
            &body,
        );
        assert_eq!(result, "User rust (#42)");
    }

    #[test]
    fn test_resolve_response_template_nested_output_array_index() {
        let params = HashMap::new();
        let body = serde_json::json!({
            "items": [{ "title": "first" }, { "title": "second" }]
        });

        let result = resolve_response_template("{output.items.0.title}", 200, &params, &body);
        assert_eq!(result, "first");
    }

    #[test]
    fn test_resolve_response_template_nested_missing_path() {
        let params = HashMap::new();
        let body = serde_json::json!({ "user": { "name": "rust" } });

        let result = resolve_response_template("{output.user.email}", 200, &params, &body);
        assert_eq!(result, "{output.user.email}");
    }

    #[test]
    fn test_resolve_string_templates() {
        let mut params = HashMap::new();
        params.insert("name".to_string(), "test".to_string());
        let result = resolve_string_templates("Hello {name}!", &params);
        assert_eq!(result, "Hello test!");
    }

    #[test]
    fn test_resolve_json_templates_string() {
        let mut params = HashMap::new();
        params.insert("key".to_string(), "value".to_string());
        let value = serde_json::json!({"field": "{key}"});
        let resolved = resolve_json_templates(&value, &params).unwrap();
        assert_eq!(resolved["field"], "value");
    }

    #[test]
    fn test_resolve_json_templates_array() {
        let mut params = HashMap::new();
        params.insert("item".to_string(), "test".to_string());
        let value = serde_json::json!(["{item}", "static"]);
        let resolved = resolve_json_templates(&value, &params).unwrap();
        assert_eq!(resolved[0], "test");
        assert_eq!(resolved[1], "static");
    }

    #[test]
    fn test_resolve_json_templates_nested_object() {
        let mut params = HashMap::new();
        params.insert("owner".to_string(), "rust-lang".to_string());
        let value = serde_json::json!({"data": {"owner": "{owner}"}});
        let resolved = resolve_json_templates(&value, &params).unwrap();
        assert_eq!(resolved["data"]["owner"], "rust-lang");
    }

    #[test]
    fn test_resolve_env_vars_multiple_in_one_string() {
        let env = HashMap::from([
            ("A".to_string(), "alpha".to_string()),
            ("B".to_string(), "beta".to_string()),
        ]);
        let result = resolve_env_vars("prefix-${A}-mid-${B}-suffix", &env).unwrap();
        assert_eq!(result, "prefix-alpha-mid-beta-suffix");
    }

    #[test]
    fn test_resolve_response_template_input_missing() {
        let params = HashMap::new();
        let body = serde_json::json!({"name": "rust"});
        let result = resolve_response_template("Owner: {input.owner}", 200, &params, &body);
        assert_eq!(result, "Owner: {input.owner}");
    }
}
