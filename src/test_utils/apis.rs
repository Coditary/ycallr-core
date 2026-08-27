use crate::models::{
    ApiDefinition, Command, EnvVar, HttpMethod, ParamType, Parameter, ResponseConfig, ResponseEntry,
};
use std::collections::HashMap;

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
