use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub use crate::models::AuthConfig;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvMode {
    Auto,
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    pub status: u16,
    pub message: String,
    pub body: Option<serde_json::Value>,
}
