use crate::client::ApiResponse;
use crate::models::{
    ApiDefinition, Command, EnvVar, HttpMethod, ParamType, Parameter, ResponseConfig, ResponseEntry,
};
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
            description: Some("Get a repository".to_string()),
            endpoint: Some("/repos/{owner}/{repo}".to_string()),
            method: Some(HttpMethod::GET),
            headers: get_repo_headers,
            params: get_repo_params,
            commands: None,
            body: None,
            responses: None,
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
            description: Some("Create a new issue".to_string()),
            endpoint: Some("/repos/{owner}/{repo}/issues".to_string()),
            method: Some(HttpMethod::POST),
            headers: create_issue_headers,
            params: create_issue_params,
            commands: None,
            body: None,
            responses: None,
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
            description: Some("List issues".to_string()),
            endpoint: Some("/repos/{owner}/{repo}/issues".to_string()),
            method: Some(HttpMethod::GET),
            headers: list_issues_headers,
            params: list_issues_params,
            commands: None,
            body: None,
            responses: None,
        },
    );

    ApiDefinition {
        name: "github".to_string(),
        version: "1.0.0".to_string(),
        description: "GitHub REST API".to_string(),
        base_url: "https://api.github.com".to_string(),
        env: vec![],
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
            description: Some("Get an item".to_string()),
            endpoint: Some("/items/{id}".to_string()),
            method: Some(HttpMethod::GET),
            headers,
            params,
            commands: None,
            body: None,
            responses: None,
        },
    );

    commands.insert(
        "create-item".to_string(),
        Command {
            description: Some("Create an item".to_string()),
            endpoint: Some("/items".to_string()),
            method: Some(HttpMethod::POST),
            headers: HashMap::new(),
            params: HashMap::new(),
            commands: None,
            body: None,
            responses: None,
        },
    );

    ApiDefinition {
        name: "simple".to_string(),
        version: "1.0.0".to_string(),
        description: "Simple test API".to_string(),
        base_url: "https://api.example.com".to_string(),
        env: vec![],
        commands,
    }
}

pub fn nested_github_api() -> ApiDefinition {
    let mut commands = HashMap::new();

    let mut repos_commands = HashMap::new();

    let mut issues_commands = HashMap::new();
    issues_commands.insert(
        "create".to_string(),
        Command {
            description: Some("Create an issue".to_string()),
            endpoint: Some("/repos/{owner}/{repo}/issues".to_string()),
            method: Some(HttpMethod::POST),
            headers: HashMap::new(),
            params: HashMap::new(),
            commands: None,
            body: None,
            responses: None,
        },
    );
    issues_commands.insert(
        "list".to_string(),
        Command {
            description: Some("List issues".to_string()),
            endpoint: Some("/repos/{owner}/{repo}/issues".to_string()),
            method: Some(HttpMethod::GET),
            headers: HashMap::new(),
            params: HashMap::new(),
            commands: None,
            body: None,
            responses: None,
        },
    );

    repos_commands.insert(
        "issues".to_string(),
        Command {
            description: Some("Issues operations".to_string()),
            endpoint: Some("/repos/{owner}/{repo}/issues".to_string()),
            method: Some(HttpMethod::GET),
            headers: HashMap::new(),
            params: HashMap::new(),
            commands: Some(issues_commands),
            body: None,
            responses: None,
        },
    );

    repos_commands.insert(
        "get".to_string(),
        Command {
            description: Some("Get a repository".to_string()),
            endpoint: Some("/repos/{owner}/{repo}".to_string()),
            method: Some(HttpMethod::GET),
            headers: HashMap::new(),
            params: HashMap::new(),
            commands: None,
            body: None,
            responses: None,
        },
    );

    commands.insert(
        "repos".to_string(),
        Command {
            description: Some("Repository operations".to_string()),
            endpoint: Some("/repos".to_string()),
            method: Some(HttpMethod::GET),
            headers: HashMap::new(),
            params: HashMap::new(),
            commands: Some(repos_commands),
            body: None,
            responses: None,
        },
    );

    let mut users_commands = HashMap::new();
    users_commands.insert(
        "get".to_string(),
        Command {
            description: Some("Get a user".to_string()),
            endpoint: Some("/users/{username}".to_string()),
            method: Some(HttpMethod::GET),
            headers: HashMap::new(),
            params: HashMap::new(),
            commands: None,
            body: None,
            responses: None,
        },
    );

    commands.insert(
        "users".to_string(),
        Command {
            description: Some("User operations".to_string()),
            endpoint: Some("/users".to_string()),
            method: Some(HttpMethod::GET),
            headers: HashMap::new(),
            params: HashMap::new(),
            commands: Some(users_commands),
            body: None,
            responses: None,
        },
    );

    ApiDefinition {
        name: "github-nested".to_string(),
        version: "1.0.0".to_string(),
        description: "GitHub REST API with nested commands".to_string(),
        base_url: "https://api.github.com".to_string(),
        env: vec![EnvVar {
            name: "GITHUB_TOKEN".to_string(),
            required: true,
        }],
        commands,
    }
}

pub fn env_api() -> ApiDefinition {
    let mut commands = HashMap::new();

    let mut headers = HashMap::new();
    headers.insert(
        "Authorization".to_string(),
        "Bearer ${GITHUB_TOKEN}".to_string(),
    );
    headers.insert(
        "Accept".to_string(),
        "application/vnd.github+json".to_string(),
    );

    commands.insert(
        "get-repo".to_string(),
        Command {
            description: Some("Get a repository".to_string()),
            endpoint: Some("/repos/{owner}/{repo}".to_string()),
            method: Some(HttpMethod::GET),
            headers,
            params: HashMap::new(),
            commands: None,
            body: None,
            responses: None,
        },
    );

    ApiDefinition {
        name: "github-env".to_string(),
        version: "1.0.0".to_string(),
        description: "GitHub API with env vars".to_string(),
        base_url: "https://api.github.com".to_string(),
        env: vec![
            EnvVar {
                name: "GITHUB_TOKEN".to_string(),
                required: true,
            },
            EnvVar {
                name: "API_VERSION".to_string(),
                required: false,
            },
        ],
        commands,
    }
}

pub fn response_api() -> ApiDefinition {
    let mut commands = HashMap::new();

    commands.insert(
        "create-item".to_string(),
        Command {
            description: Some("Create an item".to_string()),
            endpoint: Some("/items".to_string()),
            method: Some(HttpMethod::POST),
            headers: HashMap::new(),
            params: HashMap::new(),
            body: None,
            commands: None,
            responses: Some(ResponseConfig {
                success: Some(ResponseEntry {
                    message: "Created {output.name}".to_string(),
                }),
                failure: Some(ResponseEntry {
                    message: "Failed to create item".to_string(),
                }),
                warn: None,
                codes: {
                    let mut m = HashMap::new();
                    m.insert(
                        "404".to_string(),
                        ResponseEntry {
                            message: "{input.owner} not found".to_string(),
                        },
                    );
                    m
                },
            }),
        },
    );

    ApiDefinition {
        name: "response-api".to_string(),
        version: "1.0.0".to_string(),
        description: "API with response configs".to_string(),
        base_url: "https://api.example.com".to_string(),
        env: vec![],
        commands,
    }
}

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
    fn test_nested_github_api_structure() {
        let api = nested_github_api();
        assert_eq!(api.name, "github-nested");
        assert!(api.commands.contains_key("repos"));
        assert!(api.commands.contains_key("users"));

        let repos = api.commands.get("repos").unwrap();
        assert!(repos.is_branch());
        assert!(repos.is_leaf());
        assert!(repos.commands.is_some());

        let issues = repos.commands.as_ref().unwrap().get("issues").unwrap();
        assert!(issues.is_branch());
        assert!(issues.is_leaf());
    }

    #[test]
    fn test_nested_github_api_lookup() {
        let api = nested_github_api();

        let repos = api.get_command("repos");
        assert!(repos.is_ok());

        let issues = api.get_command("repos.issues");
        assert!(issues.is_ok());

        let create = api.get_command("repos.issues.create");
        assert!(create.is_ok());
        assert_eq!(create.unwrap().method.as_ref().unwrap(), &HttpMethod::POST);

        let users_get = api.get_command("users.get");
        assert!(users_get.is_ok());
        assert_eq!(
            users_get.unwrap().method.as_ref().unwrap(),
            &HttpMethod::GET
        );
    }

    #[test]
    fn test_nested_github_api_not_found() {
        let api = nested_github_api();

        let result = api.get_command("repos.nonexistent");
        assert!(result.is_err());

        let result = api.get_command("repos.issues.nonexistent");
        assert!(result.is_err());

        let result = api.get_command("nonexistent");
        assert!(result.is_err());
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

    #[test]
    fn test_mock_nested_command_tracking() {
        let mut mock = MockApiClient::new();
        mock.expect(
            "repos.issues.create",
            response_ok(serde_json::json!({"created": true})),
        );
        mock.expect(
            "repos.issues.list",
            response_ok(serde_json::json!([{"id": 1}])),
        );

        let params = make_params(&[("owner", "rust-lang"), ("repo", "rust")]);
        let body = serde_json::json!({"title": "Bug"});
        mock.call("repos.issues.create", &params, Some(&body))
            .unwrap();
        mock.call("repos.issues.list", &params, None).unwrap();

        assert_eq!(mock.call_count("repos.issues.create"), 1);
        assert_eq!(mock.call_count("repos.issues.list"), 1);
        assert!(mock.was_called("repos.issues.create"));
        assert!(mock.was_called("repos.issues.list"));
    }

    #[test]
    fn test_env_api_structure() {
        let api = env_api();
        assert_eq!(api.name, "github-env");
        assert_eq!(api.env.len(), 2);
        assert_eq!(api.env[0].name, "GITHUB_TOKEN");
        assert!(api.env[0].required);
        assert_eq!(api.env[1].name, "API_VERSION");
        assert!(!api.env[1].required);
    }

    #[test]
    fn test_env_api_has_substitution() {
        let api = env_api();
        let cmd = api.commands.get("get-repo").unwrap();
        assert_eq!(
            cmd.headers.get("Authorization").unwrap(),
            "Bearer ${GITHUB_TOKEN}"
        );
    }

    #[test]
    fn test_response_api_structure() {
        let api = response_api();
        assert_eq!(api.name, "response-api");
        let cmd = api.commands.get("create-item").unwrap();
        assert!(cmd.responses.is_some());
        let responses = cmd.responses.as_ref().unwrap();
        assert!(responses.success.is_some());
        assert!(responses.failure.is_some());
        assert!(responses.codes.contains_key("404"));
    }

    #[test]
    fn test_response_api_messages() {
        let api = response_api();
        let cmd = api.commands.get("create-item").unwrap();
        let responses = cmd.responses.as_ref().unwrap();
        assert_eq!(
            responses.success.as_ref().unwrap().message,
            "Created {output.name}"
        );
        assert_eq!(
            responses.failure.as_ref().unwrap().message,
            "Failed to create item"
        );
        assert_eq!(
            responses.codes.get("404").unwrap().message,
            "{input.owner} not found"
        );
    }

    #[test]
    fn test_response_with_message_helper() {
        let resp = response_with_message(
            200,
            serde_json::json!({"ok": true}),
            "Created item".to_string(),
        );
        assert_eq!(resp.status, 200);
        assert_eq!(resp.message, Some("Created item".to_string()));
    }
}
