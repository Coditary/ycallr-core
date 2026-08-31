use std::collections::HashMap;

use crate::call_engine::{build_api_response, prepare_http_request, ClientContext, PreparedBody};
use crate::error::{Result, YcallrError};
use crate::models::HttpMethod;

use super::types::ApiResponse;
use super::YcallrClient;

pub fn call(
    client: &YcallrClient,
    command: &str,
    params: &HashMap<String, String>,
    body: Option<&serde_json::Value>,
) -> Result<ApiResponse> {
    let ctx = ClientContext {
        api: client.api.clone(),
        auth: client.auth.clone(),
        auth_configs: client.auth_configs.clone(),
        env_vars: client.env_vars.clone(),
    };

    let prepared = prepare_http_request(&ctx, command, params, body)?;

    let mut request = new_request(client, &prepared.method, &prepared.url)?;

    for (key, value) in &prepared.headers {
        request = request.header(key.as_str(), value.as_str());
    }

    match &prepared.body {
        PreparedBody::None => {}
        PreparedBody::Json(json) => {
            request = request.json(json);
        }
        PreparedBody::Form(form) => {
            request = request.form(form);
        }
        PreparedBody::Raw { content_type, body } => {
            request = request
                .header("Content-Type", content_type)
                .body(body.clone());
        }
        #[cfg(not(target_arch = "wasm32"))]
        PreparedBody::MultipartNative(parts) => {
            let mut form = reqwest::blocking::multipart::Form::new();
            for part in parts {
                match part {
                    crate::call_engine::NativeMultipartPart::Text { name, value } => {
                        form = form.text(name.clone(), value.clone());
                    }
                    crate::call_engine::NativeMultipartPart::File { name, path } => {
                        let file_part = reqwest::blocking::multipart::Part::file(path)
                            .map_err(|e| YcallrError::HttpClient(e.to_string()))?
                            .mime_str("application/octet-stream")
                            .map_err(|e| YcallrError::HttpClient(e.to_string()))?;
                        form = form.part(name.clone(), file_part);
                    }
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

    Ok(build_api_response(
        status,
        headers,
        body_text,
        prepared.responses.as_ref(),
        client.api.errors.as_ref(),
        &prepared.params,
    ))
}

fn new_request(
    client: &YcallrClient,
    method: &HttpMethod,
    url: &str,
) -> Result<reqwest::blocking::RequestBuilder> {
    Ok(match method {
        HttpMethod::GET => client.http_client.get(url),
        HttpMethod::POST => client.http_client.post(url),
        HttpMethod::PUT => client.http_client.put(url),
        HttpMethod::DELETE => client.http_client.delete(url),
        HttpMethod::PATCH => client.http_client.patch(url),
    })
}
