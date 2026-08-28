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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MultipartField {
    pub name: String,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub file: Option<String>,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
