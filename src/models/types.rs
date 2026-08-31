use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum AuthConfig {
    #[serde(rename = "bearer")]
    Bearer { token: String },
    #[serde(rename = "api_key")]
    ApiKey {
        key: String,
        #[serde(default = "default_header_name")]
        name: String,
        #[serde(default = "default_api_key_location", rename = "in")]
        in_: ApiKeyLocation,
    },
    #[serde(rename = "http")]
    Http {
        scheme: String,
        #[serde(default)]
        token: Option<String>,
        #[serde(default)]
        username: Option<String>,
        #[serde(default)]
        password: Option<String>,
        #[serde(default)]
        prefix: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ApiKeyLocation {
    Header,
    Query,
    Cookie,
}

fn default_api_key_location() -> ApiKeyLocation {
    ApiKeyLocation::Header
}

fn default_header_name() -> String {
    "X-API-Key".to_string()
}

impl AuthConfig {
    pub fn bearer(token: String) -> Self {
        Self::Bearer { token }
    }

    pub fn api_key(key: String, header: String) -> Self {
        Self::ApiKey {
            key,
            name: header,
            in_: ApiKeyLocation::Header,
        }
    }

    pub fn api_key_in(key: String, name: String, in_: ApiKeyLocation) -> Self {
        Self::ApiKey { key, name, in_ }
    }

    pub fn http_basic(username: String, password: String) -> Self {
        Self::Http {
            scheme: "basic".to_string(),
            token: None,
            username: Some(username),
            password: Some(password),
            prefix: None,
        }
    }

    pub fn http_custom(prefix: String, token: String) -> Self {
        Self::Http {
            scheme: "custom".to_string(),
            token: Some(token),
            username: None,
            password: None,
            prefix: Some(prefix),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ApiDefinition {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: String,
    pub base_url: String,
    #[serde(default)]
    pub env: Vec<EnvVar>,
    #[serde(default)]
    pub auth: HashMap<String, AuthConfig>,
    pub commands: HashMap<String, Command>,
    #[serde(default)]
    pub errors: Option<ApiErrorConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EnvVar {
    pub name: String,
    #[serde(default = "default_true")]
    pub required: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BodyConfig {
    #[serde(default)]
    pub json: Option<serde_json::Value>,
    #[serde(default)]
    pub form: Option<HashMap<String, String>>,
    #[serde(default)]
    pub multipart: Option<Vec<MultipartField>>,
    #[serde(default)]
    pub raw: Option<String>,
}

impl BodyConfig {
    /// Non-empty body kinds configured on this command (json, form, raw, multipart).
    pub fn body_kinds(&self) -> Vec<&'static str> {
        let mut kinds = Vec::new();
        if self.json.is_some() {
            kinds.push("json");
        }
        if self.form.as_ref().is_some_and(|f| !f.is_empty()) {
            kinds.push("form");
        }
        if self.raw.as_ref().is_some_and(|r| !r.is_empty()) {
            kinds.push("raw");
        }
        if self.multipart.as_ref().is_some_and(|m| !m.is_empty()) {
            kinds.push("multipart");
        }
        kinds
    }

    pub fn active_body_kind(&self) -> Option<&'static str> {
        self.body_kinds().first().copied()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MultipartField {
    pub name: String,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub file: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_config_constructors() {
        let bearer = AuthConfig::bearer("t".to_string());
        assert!(matches!(bearer, AuthConfig::Bearer { .. }));
        let key = AuthConfig::api_key("k".to_string(), "H".to_string());
        assert!(matches!(key, AuthConfig::ApiKey { .. }));
        let key_q = AuthConfig::api_key_in("k".to_string(), "Q".to_string(), ApiKeyLocation::Query);
        assert!(matches!(key_q, AuthConfig::ApiKey { .. }));
        let basic = AuthConfig::http_basic("u".to_string(), "p".to_string());
        assert!(matches!(basic, AuthConfig::Http { .. }));
        let custom = AuthConfig::http_custom("Token ".to_string(), "abc".to_string());
        assert!(matches!(custom, AuthConfig::Http { .. }));
    }

    #[test]
    fn test_body_config_kinds() {
        let body = BodyConfig {
            json: Some(serde_json::json!({"a": 1})),
            form: None,
            multipart: None,
            raw: None,
        };
        assert_eq!(body.active_body_kind(), Some("json"));
        assert_eq!(body.body_kinds(), vec!["json"]);

        let empty_form = BodyConfig {
            json: None,
            form: Some(HashMap::new()),
            multipart: None,
            raw: None,
        };
        assert!(empty_form.active_body_kind().is_none());
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Command {
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default, alias = "path")]
    pub endpoint: Option<String>,
    #[serde(default)]
    pub method: Option<HttpMethod>,
    #[serde(default)]
    pub auth: Option<String>,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    #[serde(default)]
    pub params: HashMap<String, Parameter>,
    #[serde(default)]
    pub body: Option<BodyConfig>,
    #[serde(default)]
    pub responses: Option<ResponseConfig>,
    #[serde(default)]
    pub commands: Option<HashMap<String, Command>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResponseConfig {
    pub success: Option<ResponseEntry>,
    pub failure: Option<ResponseEntry>,
    pub warn: Option<ResponseEntry>,
    #[serde(flatten)]
    pub codes: HashMap<String, ResponseEntry>,
}

/// API-root default error message templates (`errors:` in YAML).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ApiErrorConfig {
    #[serde(default)]
    pub default: Option<ResponseEntry>,
    #[serde(flatten)]
    pub codes: HashMap<String, ResponseEntry>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResponseEntry {
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HttpMethod {
    GET,
    POST,
    PUT,
    DELETE,
    PATCH,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Parameter {
    pub description: String,
    #[serde(rename = "type")]
    pub param_type: ParamType,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ParamType {
    #[serde(rename = "string")]
    String,
    #[serde(rename = "number")]
    Number,
    #[serde(rename = "boolean")]
    Boolean,
    #[serde(rename = "array")]
    Array,
}
