use crate::models::{ApiErrorConfig, ResponseConfig};

use super::types::ResponseEntry;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// API `errors:` templates must not reference `{input.*}` — callers may omit params.
pub fn api_error_template_references_input(message: &str) -> bool {
    message.contains("{input.")
}

impl<'de> Deserialize<'de> for ResponseEntry {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Helper {
            Text(String),
            Map(ResponseEntryMap),
        }

        #[derive(Deserialize)]
        struct ResponseEntryMap {
            message: String,
        }

        match Helper::deserialize(deserializer)? {
            Helper::Text(message) => Ok(ResponseEntry { message }),
            Helper::Map(map) => Ok(ResponseEntry {
                message: map.message,
            }),
        }
    }
}

impl Serialize for ResponseEntry {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.message)
    }
}

impl ResponseConfig {
    pub fn get_entry_for_status(&self, status: u16) -> Option<&ResponseEntry> {
        let code_str = status.to_string();
        if let Some(entry) = self.codes.get(&code_str) {
            return Some(entry);
        }

        match status {
            200..=299 => self.success.as_ref(),
            300..=399 => self.warn.as_ref(),
            400..=599 => self.failure.as_ref(),
            _ => None,
        }
    }
}

impl ApiErrorConfig {
    /// Exact status code in `errors`, then `default` for HTTP error responses (status >= 400).
    pub fn get_entry_for_status(&self, status: u16) -> Option<&ResponseEntry> {
        let code_str = status.to_string();
        if let Some(entry) = self.codes.get(&code_str) {
            return Some(entry);
        }
        if status >= 400 {
            return self.default.as_ref();
        }
        None
    }
}

/// Built-in fallback when neither command `responses` nor API `errors` define a template.
pub fn builtin_response_template(status: u16) -> &'static str {
    match status {
        200..=299 => "Request succeeded (HTTP {status})",
        300..=399 => "Request completed (HTTP {status})",
        400..=499 => "Request failed (HTTP {status})",
        500..=599 => "Request failed (HTTP {status})",
        _ => "HTTP {status}",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_response_entry_yaml_string_form() {
        let entry: ResponseEntry = serde_yaml::from_str("\"Not found\"").unwrap();
        assert_eq!(entry.message, "Not found");
    }

    #[test]
    fn test_response_config_warn_bucket() {
        let config = ResponseConfig {
            codes: HashMap::new(),
            success: None,
            warn: Some(ResponseEntry {
                message: "Redirect {status}".to_string(),
            }),
            failure: None,
        };
        assert_eq!(
            config.get_entry_for_status(302).unwrap().message,
            "Redirect {status}"
        );
    }

    #[test]
    fn test_builtin_response_template_warn_and_server_error() {
        assert_eq!(
            builtin_response_template(301),
            "Request completed (HTTP {status})"
        );
        assert_eq!(
            builtin_response_template(500),
            "Request failed (HTTP {status})"
        );
    }

    #[test]
    fn test_api_error_template_references_input() {
        assert!(api_error_template_references_input("{input.foo}"));
        assert!(!api_error_template_references_input("{output.foo}"));
    }

    #[test]
    fn test_response_entry_serialize() {
        let entry = ResponseEntry {
            message: "ok".to_string(),
        };
        let yaml = serde_yaml::to_string(&entry).unwrap();
        assert_eq!(yaml.trim(), "ok");
    }

    #[test]
    fn test_api_error_config_exact_code_and_default() {
        let mut codes = HashMap::new();
        codes.insert(
            "404".to_string(),
            ResponseEntry {
                message: "Not found".to_string(),
            },
        );
        let config = ApiErrorConfig {
            default: Some(ResponseEntry {
                message: "API error".to_string(),
            }),
            codes,
        };

        assert_eq!(
            config.get_entry_for_status(404).unwrap().message,
            "Not found"
        );
        assert_eq!(
            config.get_entry_for_status(500).unwrap().message,
            "API error"
        );
        assert!(config.get_entry_for_status(200).is_none());
    }

    #[test]
    fn test_builtin_response_templates() {
        assert_eq!(
            builtin_response_template(200),
            "Request succeeded (HTTP {status})"
        );
        assert_eq!(
            builtin_response_template(404),
            "Request failed (HTTP {status})"
        );
    }
}
