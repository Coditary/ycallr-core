use crate::client::ApiResponse;
use std::collections::HashMap;

pub fn response_ok(body: serde_json::Value) -> ApiResponse {
    ApiResponse {
        status: 200,
        headers: HashMap::new(),
        body,
        message: None,
    }
}

pub fn response_created(body: serde_json::Value) -> ApiResponse {
    ApiResponse {
        status: 201,
        headers: HashMap::new(),
        body,
        message: None,
    }
}

pub fn response_not_found() -> ApiResponse {
    ApiResponse {
        status: 404,
        headers: HashMap::new(),
        body: serde_json::json!({"message": "Not Found"}),
        message: None,
    }
}

pub fn response_with_headers(
    status: u16,
    headers: HashMap<String, String>,
    body: serde_json::Value,
) -> ApiResponse {
    ApiResponse {
        status,
        headers,
        body,
        message: None,
    }
}

pub fn response_with_message(status: u16, body: serde_json::Value, message: String) -> ApiResponse {
    ApiResponse {
        status,
        headers: HashMap::new(),
        body,
        message: Some(message),
    }
}

pub fn make_params(pairs: &[(&str, &str)]) -> HashMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}
