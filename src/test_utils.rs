use crate::client::ApiResponse;
use crate::models::{ApiDefinition, Command, HttpMethod, ParamType, Parameter};
use std::collections::HashMap;

pub struct MockApiClient {
    responses: HashMap<String, ApiResponse>,
    calls: Vec<MockCall>,
}

#[derive(Debug, Clone)]
pub struct MockCall {
    pub command: String,
    pub params: HashMap<String, String>,
    pub body: Option<serde_json::Value>,
}

impl MockApiClient {
    pub fn new() -> Self {
        Self {
            responses: HashMap::new(),
            calls: Vec::new(),
        }
    }

    pub fn expect(&mut self, command: &str, response: ApiResponse) -> &mut Self {
        self.responses.insert(command.to_string(), response);
        self
    }

    pub fn call(
        &mut self,
        command: &str,
        params: &HashMap<String, String>,
        body: Option<&serde_json::Value>,
    ) -> crate::Result<ApiResponse> {
        self.calls.push(MockCall {
            command: command.to_string(),
            params: params.clone(),
            body: body.cloned(),
        });

        self.responses
            .get(command)
            .cloned()
            .ok_or_else(|| crate::YcallrError::CommandNotFound(command.to_string()))
    }

    pub fn calls(&self) -> &[MockCall] {
        &self.calls
    }

    pub fn last_call(&self) -> Option<&MockCall> {
        self.calls.last()
    }

    pub fn was_called(&self, command: &str) -> bool {
        self.calls.iter().any(|c| c.command == command)
    }

    pub fn call_count(&self, command: &str) -> usize {
        self.calls.iter().filter(|c| c.command == command).count()
    }
}

pub fn github_api() -> ApiDefinition {
    let mut commands = HashMap::new();

    let mut get_repo_params = HashMap::new();
    get_repo_params.insert(
        "owner".to_string(),
        Parameter {
            description: "Repository owner".to_string(),
            param_type: ParamType::String,
            required: true,
        },
    );
    get_repo_params.insert(
        "repo".to_string(),
        Parameter {
            description: "Repository name".to_string(),
            param_type: ParamType::String,
            required: true,
        },
    );

    let mut get_repo_headers = HashMap::new();
    get_repo_headers.insert(
        "Accept".to_string(),
        "application/vnd.github.v3+json".to_string(),
    );

    commands.insert(
        "get-repo".to_string(),
        Command {
            endpoint: "/repos/{owner}/{repo}".to_string(),
            method: HttpMethod::GET,
            headers: get_repo_headers,
            params: get_repo_params,
        },
    );

    let mut create_issue_params = HashMap::new();
    create_issue_params.insert(
        "owner".to_string(),
        Parameter {
            description: "Repository owner".to_string(),
            param_type: ParamType::String,
            required: true,
        },
    );
    create_issue_params.insert(
        "repo".to_string(),
        Parameter {
            description: "Repository name".to_string(),
            param_type: ParamType::String,
            required: true,
        },
    );
    create_issue_params.insert(
        "title".to_string(),
        Parameter {
            description: "Issue title".to_string(),
            param_type: ParamType::String,
            required: true,
        },
    );

    let mut create_issue_headers = HashMap::new();
    create_issue_headers.insert(
        "Accept".to_string(),
        "application/vnd.github.v3+json".to_string(),
    );
    create_issue_headers.insert("Content-Type".to_string(), "application/json".to_string());

    commands.insert(
        "create-issue".to_string(),
        Command {
            endpoint: "/repos/{owner}/{repo}/issues".to_string(),
            method: HttpMethod::POST,
            headers: create_issue_headers,
            params: create_issue_params,
        },
    );

    let mut list_issues_params = HashMap::new();
    list_issues_params.insert(
        "owner".to_string(),
        Parameter {
            description: "Repository owner".to_string(),
            param_type: ParamType::String,
            required: true,
        },
    );
    list_issues_params.insert(
        "repo".to_string(),
        Parameter {
            description: "Repository name".to_string(),
            param_type: ParamType::String,
            required: true,
        },
    );
    list_issues_params.insert(
        "state".to_string(),
        Parameter {
            description: "Filter by state (open, closed, all)".to_string(),
            param_type: ParamType::String,
            required: false,
        },
    );

    let mut list_issues_headers = HashMap::new();
    list_issues_headers.insert(
        "Accept".to_string(),
        "application/vnd.github.v3+json".to_string(),
    );

    commands.insert(
        "list-issues".to_string(),
        Command {
            endpoint: "/repos/{owner}/{repo}/issues".to_string(),
            method: HttpMethod::GET,
            headers: list_issues_headers,
            params: list_issues_params,
        },
    );

    ApiDefinition {
        name: "github".to_string(),
        version: "1.0.0".to_string(),
        description: "GitHub REST API".to_string(),
        base_url: "https://api.github.com".to_string(),
        commands,
    }
}

pub fn simple_api() -> ApiDefinition {
    let mut commands = HashMap::new();
    let mut params = HashMap::new();

    params.insert(
        "id".to_string(),
        Parameter {
            description: "Resource ID".to_string(),
            param_type: ParamType::String,
            required: true,
        },
    );

    let mut headers = HashMap::new();
    headers.insert("Accept".to_string(), "application/json".to_string());

    commands.insert(
        "get-item".to_string(),
        Command {
            endpoint: "/items/{id}".to_string(),
            method: HttpMethod::GET,
            headers,
            params,
        },
    );

    commands.insert(
        "create-item".to_string(),
        Command {
            endpoint: "/items".to_string(),
            method: HttpMethod::POST,
            headers: HashMap::new(),
            params: HashMap::new(),
        },
    );

    ApiDefinition {
        name: "simple".to_string(),
        version: "1.0.0".to_string(),
        description: "Simple test API".to_string(),
        base_url: "https://api.example.com".to_string(),
        commands,
    }
}

pub fn response_ok(body: serde_json::Value) -> ApiResponse {
    ApiResponse {
        status: 200,
        headers: HashMap::new(),
        body,
    }
}

pub fn response_created(body: serde_json::Value) -> ApiResponse {
    ApiResponse {
        status: 201,
        headers: HashMap::new(),
        body,
    }
}

pub fn response_not_found() -> ApiResponse {
    ApiResponse {
        status: 404,
        headers: HashMap::new(),
        body: serde_json::json!({"message": "Not Found"}),
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
    }
}

pub fn make_params(pairs: &[(&str, &str)]) -> HashMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_client_expect_and_call() {
        let mut mock = MockApiClient::new();
        mock.expect("get-item", response_ok(serde_json::json!({"id": "1"})));

        let params = make_params(&[("id", "1")]);
        let response = mock.call("get-item", &params, None).unwrap();

        assert_eq!(response.status, 200);
        assert_eq!(response.body["id"], "1");
    }

    #[test]
    fn test_mock_client_tracks_calls() {
        let mut mock = MockApiClient::new();
        mock.expect("get-item", response_ok(serde_json::json!({})));

        let params = make_params(&[("id", "1")]);
        mock.call("get-item", &params, None).unwrap();
        mock.call("get-item", &params, None).unwrap();

        assert_eq!(mock.call_count("get-item"), 2);
        assert!(mock.was_called("get-item"));
        assert!(!mock.was_called("create-item"));
    }

    #[test]
    fn test_mock_client_last_call() {
        let mut mock = MockApiClient::new();
        mock.expect("get-item", response_ok(serde_json::json!({})));
        mock.expect("create-item", response_created(serde_json::json!({})));

        let params = make_params(&[("id", "1")]);
        mock.call("get-item", &params, None).unwrap();

        let body = serde_json::json!({"name": "test"});
        mock.call("create-item", &params, Some(&body)).unwrap();

        let last = mock.last_call().unwrap();
        assert_eq!(last.command, "create-item");
        assert_eq!(last.body.as_ref().unwrap()["name"], "test");
    }

    #[test]
    fn test_mock_client_not_found() {
        let mut mock = MockApiClient::new();
        let params = make_params(&[]);
        let result = mock.call("nonexistent", &params, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_github_api_structure() {
        let api = github_api();
        assert_eq!(api.name, "github");
        assert!(api.commands.contains_key("get-repo"));
        assert!(api.commands.contains_key("create-issue"));
        assert!(api.commands.contains_key("list-issues"));
    }

    #[test]
    fn test_simple_api_structure() {
        let api = simple_api();
        assert_eq!(api.name, "simple");
        assert_eq!(api.commands.len(), 2);
    }

    #[test]
    fn test_make_params() {
        let params = make_params(&[("owner", "rust-lang"), ("repo", "rust")]);
        assert_eq!(params.get("owner").unwrap(), "rust-lang");
        assert_eq!(params.get("repo").unwrap(), "rust");
    }

    #[test]
    fn test_response_helpers() {
        let ok = response_ok(serde_json::json!({"ok": true}));
        assert_eq!(ok.status, 200);

        let created = response_created(serde_json::json!({"id": "1"}));
        assert_eq!(created.status, 201);

        let not_found = response_not_found();
        assert_eq!(not_found.status, 404);

        let mut headers = HashMap::new();
        headers.insert("X-Custom".to_string(), "value".to_string());
        let with_headers = response_with_headers(200, headers, serde_json::json!({"data": 1}));
        assert_eq!(with_headers.status, 200);
        assert_eq!(with_headers.headers.get("X-Custom").unwrap(), "value");
    }

    #[test]
    fn test_mock_client_calls_method() {
        let mut mock = MockApiClient::new();
        mock.expect("get-item", response_ok(serde_json::json!({})));
        mock.expect("create-item", response_ok(serde_json::json!({})));

        let params = make_params(&[("id", "1")]);
        mock.call("get-item", &params, None).unwrap();
        mock.call("create-item", &params, None).unwrap();

        let calls = mock.calls();
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].command, "get-item");
        assert_eq!(calls[1].command, "create-item");
    }
}
