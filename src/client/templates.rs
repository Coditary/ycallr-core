use crate::models::BodyConfig;
use regex::Regex;
use std::collections::HashMap;

pub fn resolve_env_vars(text: &str, env_vars: &HashMap<String, String>) -> crate::Result<String> {
    let re = Regex::new(r"\$\{([^}]+)\}").unwrap();
    let mut result = text.to_string();

    for cap in re.captures_iter(text) {
        let var_name = &cap[1];
        let replacement = env_vars.get(var_name).map(|s| s.as_str()).unwrap_or("");
        result = result.replace(&cap[0], replacement);
    }

    Ok(result)
}

pub fn resolve_response_template(
    template: &str,
    params: &HashMap<String, String>,
    body: &serde_json::Value,
) -> String {
    let re = Regex::new(r"\{(input|output)\.([^}]+)\}").unwrap();
    let mut result = template.to_string();

    for cap in re.captures_iter(template) {
        let prefix = &cap[1];
        let field = &cap[2];

        let replacement = match prefix {
            "input" => params.get(field).map(|s| s.as_str()).unwrap_or(""),
            "output" => {
                if let Some(val) = body.get(field) {
                    match val {
                        serde_json::Value::String(s) => s.as_str(),
                        other => {
                            let s = other.to_string();
                            return result.replace(&cap[0], &s);
                        }
                    }
                } else {
                    ""
                }
            }
            _ => "",
        };

        result = result.replace(&cap[0], replacement);
    }

    result
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
                    text: f
                        .text
                        .as_ref()
                        .map(|t| resolve_string_templates(t, params)),
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
    fn test_resolve_env_vars() {
        let mut env_vars = HashMap::new();
        env_vars.insert("TOKEN".to_string(), "abc123".to_string());
        let result = resolve_env_vars("Bearer ${TOKEN}", &env_vars).unwrap();
        assert_eq!(result, "Bearer abc123");
    }

    #[test]
    fn test_resolve_env_vars_unknown() {
        let env_vars = HashMap::new();
        let result = resolve_env_vars("${UNKNOWN}", &env_vars).unwrap();
        assert_eq!(result, "");
    }

    #[test]
    fn test_resolve_response_template_input() {
        let mut params = HashMap::new();
        params.insert("owner".to_string(), "rust-lang".to_string());
        let body = serde_json::json!({});

        let result = resolve_response_template("{input.owner} not found", &params, &body);
        assert_eq!(result, "rust-lang not found");
    }

    #[test]
    fn test_resolve_response_template_output() {
        let params = HashMap::new();
        let body = serde_json::json!({"name": "rust", "stars": 90000});

        let result = resolve_response_template(
            "Got repo {output.name} with {output.stars} stars",
            &params,
            &body,
        );
        assert_eq!(result, "Got repo rust with 90000 stars");
    }

    #[test]
    fn test_resolve_response_template_mixed() {
        let mut params = HashMap::new();
        params.insert("owner".to_string(), "rust-lang".to_string());
        let body = serde_json::json!({"name": "rust"});

        let result = resolve_response_template("{input.owner}/{output.name}", &params, &body);
        assert_eq!(result, "rust-lang/rust");
    }

    #[test]
    fn test_resolve_response_template_missing_field() {
        let params = HashMap::new();
        let body = serde_json::json!({"name": "rust"});

        let result = resolve_response_template("{output.missing}", &params, &body);
        assert_eq!(result, "");
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
}
