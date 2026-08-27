use crate::error::{Result, YcallrError};
use crate::models::HttpMethod;
use std::collections::HashMap;

use super::templates;
use super::types::{ApiResponse, AuthConfig};
use super::YcallrClient;

pub fn call(
    client: &YcallrClient,
    command: &str,
    params: &HashMap<String, String>,
    body: Option<&serde_json::Value>,
) -> Result<ApiResponse> {
    let cmd = client.api.get_command(command)?;

    let endpoint = cmd.resolve_endpoint(params)?;
    let resolved_endpoint = templates::resolve_env_vars(&endpoint, &client.env_vars)?;
    let url = format!(
        "{}{}",
        client.api.base_url.trim_end_matches('/'),
        resolved_endpoint
    );

    let method = cmd
        .method
        .as_ref()
        .ok_or_else(|| YcallrError::ParamValidation("Command has no method".into()))?;

    let mut request = match method {
        HttpMethod::GET => client.http_client.get(&url),
        HttpMethod::POST => client.http_client.post(&url),
        HttpMethod::PUT => client.http_client.put(&url),
        HttpMethod::DELETE => client.http_client.delete(&url),
        HttpMethod::PATCH => client.http_client.patch(&url),
    };

    for (key, value) in &cmd.headers {
        let resolved_value = templates::resolve_env_vars(value, &client.env_vars)?;
        request = request.header(key.as_str(), resolved_value.as_str());
    }

    if let Some(auth) = &client.auth {
        match auth {
            AuthConfig::Bearer(token) => {
                request = request.bearer_auth(token);
            }
            AuthConfig::ApiKey { key, header } => {
                request = request.header(header.as_str(), key.as_str());
            }
        }
    }

    let resolved_body_config = if let Some(body_config) = &cmd.body {
        let resolved = templates::resolve_body(body_config, params)?;
        if resolved.json.is_none()
            && resolved.form.is_none()
            && resolved.multipart.is_none()
            && resolved.raw.is_none()
        {
            None
        } else {
            Some(resolved)
        }
    } else {
        None
    };

    if let Some(body) = body {
        request = request.json(body);
    } else if let Some(body_config) = &resolved_body_config {
        if let Some(json) = &body_config.json {
            request = request.json(json);
        } else if let Some(form) = &body_config.form {
            request = request.form(form);
        } else if let Some(raw) = &body_config.raw {
            request = request
                .header("Content-Type", "text/plain")
                .body(raw.clone());
        } else if let Some(multipart_fields) = &body_config.multipart {
            let mut form = reqwest::blocking::multipart::Form::new();
            for field in multipart_fields {
                if let Some(text) = &field.text {
                    form = form.text(field.name.clone(), text.clone());
                } else if let Some(file_path) = &field.file {
                    let path = std::path::Path::new(file_path);
                    let file_name = path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| field.name.clone());
                    let part = reqwest::blocking::multipart::Part::file(path)
                        .map_err(|e| YcallrError::HttpClient(e.to_string()))?
                        .mime_str("application/octet-stream")
                        .map_err(|e| YcallrError::HttpClient(e.to_string()))?;
                    form = form.part(file_name, part);
                }
            }
            request = request.multipart(form);
        }
    }

    let response = request
        .send()
        .map_err(|e| YcallrError::HttpClient(e.to_string()))?;

    let status = response.status().as_u16();

    let headers: HashMap<String, String> = response
        .headers()
        .iter()
        .filter_map(|(k, v)| v.to_str().ok().map(|val| (k.to_string(), val.to_string())))
        .collect();

    let body_text = response
        .text()
        .map_err(|e| YcallrError::HttpClient(e.to_string()))?;

    let body_json: serde_json::Value = serde_json::from_str(&body_text)
        .unwrap_or_else(|_| serde_json::Value::String(body_text));

    let message = if let Some(responses) = &cmd.responses {
        if let Some(entry) = responses.get_entry_for_status(status) {
            Some(templates::resolve_response_template(
                &entry.message,
                params,
                &body_json,
            ))
        } else {
            None
        }
    } else {
        None
    };

    Ok(ApiResponse {
        status,
        headers,
        body: body_json,
        message,
    })
}
