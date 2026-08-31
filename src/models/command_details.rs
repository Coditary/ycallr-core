use serde::Serialize;
use std::collections::HashMap;

use super::{Command, HttpMethod, Parameter};

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct CommandDetails {
    pub path: String,
    pub description: Option<String>,
    pub endpoint: Option<String>,
    pub method: Option<HttpMethod>,
    pub is_leaf: bool,
    pub is_branch: bool,
    pub is_callable: bool,
    pub auth: Option<String>,
    pub headers: HashMap<String, String>,
    pub params: HashMap<String, Parameter>,
    pub has_body: bool,
    pub has_responses: bool,
    pub subcommands: Vec<String>,
}

impl Command {
    pub fn subcommand_names(&self) -> Vec<String> {
        match &self.commands {
            Some(commands) => {
                let mut names: Vec<String> = commands.keys().cloned().collect();
                names.sort();
                names
            }
            None => Vec::new(),
        }
    }

    pub fn is_callable(&self) -> bool {
        self.is_leaf()
    }

    pub fn details_at_path(&self, path: &str) -> CommandDetails {
        CommandDetails {
            path: path.to_string(),
            description: self.description.clone(),
            endpoint: self.endpoint.clone(),
            method: self.method.clone(),
            is_leaf: self.is_leaf(),
            is_branch: self.is_branch(),
            is_callable: self.is_callable(),
            auth: self.auth.clone(),
            headers: self.headers.clone(),
            params: self.params.clone(),
            has_body: self.body.is_some(),
            has_responses: self.responses.is_some(),
            subcommands: self.subcommand_names(),
        }
    }
}

impl crate::models::ApiDefinition {
    pub fn command_details(&self, path: &str) -> crate::Result<CommandDetails> {
        let cmd = self.get_command(path)?;
        Ok(cmd.details_at_path(path))
    }

    pub fn list_subcommands(&self, path: &str) -> crate::Result<Vec<String>> {
        let cmd = self.get_command(path)?;
        Ok(cmd.subcommand_names())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{BodyConfig, HttpMethod, ResponseConfig, ResponseEntry};
    use std::collections::HashMap;

    fn branch_only_tree() -> Command {
        let mut issues_commands = HashMap::new();
        issues_commands.insert(
            "list".to_string(),
            Command {
                description: Some("List issues".to_string()),
                endpoint: Some("/issues".to_string()),
                method: Some(HttpMethod::GET),
                auth: None,
                headers: HashMap::new(),
                params: HashMap::new(),
                body: None,
                responses: None,
                commands: None,
            },
        );

        let mut repos_commands = HashMap::new();
        repos_commands.insert(
            "issues".to_string(),
            Command {
                description: Some("Issues branch".to_string()),
                endpoint: None,
                method: None,
                auth: None,
                headers: HashMap::new(),
                params: HashMap::new(),
                body: None,
                responses: None,
                commands: Some(issues_commands),
            },
        );

        Command {
            description: Some("Repos branch".to_string()),
            endpoint: None,
            method: None,
            auth: None,
            headers: HashMap::new(),
            params: HashMap::new(),
            body: None,
            responses: None,
            commands: Some(repos_commands),
        }
    }

    #[test]
    fn test_subcommand_names_sorted() {
        let mut commands = HashMap::new();
        commands.insert("zebra".to_string(), empty_leaf());
        commands.insert("alpha".to_string(), empty_leaf());
        commands.insert("beta".to_string(), empty_leaf());

        let cmd = Command {
            description: None,
            endpoint: None,
            method: None,
            auth: None,
            headers: HashMap::new(),
            params: HashMap::new(),
            body: None,
            responses: None,
            commands: Some(commands),
        };

        assert_eq!(cmd.subcommand_names(), vec!["alpha", "beta", "zebra"]);
    }

    #[test]
    fn test_details_branch_only_not_callable() {
        let cmd = branch_only_tree();
        let details = cmd.details_at_path("repos");

        assert!(!details.is_leaf);
        assert!(details.is_branch);
        assert!(!details.is_callable);
        assert!(details.endpoint.is_none());
        assert!(details.method.is_none());
        assert_eq!(details.subcommands, vec!["issues".to_string()]);
    }

    #[test]
    fn test_details_includes_body_responses_auth_headers() {
        let mut headers = HashMap::new();
        headers.insert("X-Custom".to_string(), "value".to_string());

        let cmd = Command {
            description: Some("Desc".to_string()),
            endpoint: Some("/path".to_string()),
            method: Some(HttpMethod::POST),
            auth: Some("primary".to_string()),
            headers,
            params: HashMap::new(),
            body: Some(BodyConfig {
                json: Some(serde_json::json!({"k": "v"})),
                form: None,
                raw: None,
                multipart: None,
            }),
            responses: Some(ResponseConfig {
                success: Some(ResponseEntry {
                    message: "ok".to_string(),
                }),
                failure: None,
                warn: None,
                codes: HashMap::new(),
            }),
            commands: None,
        };

        let details = cmd.details_at_path("cmd");
        assert_eq!(details.description.as_deref(), Some("Desc"));
        assert_eq!(details.auth.as_deref(), Some("primary"));
        assert_eq!(
            details.headers.get("X-Custom").map(String::as_str),
            Some("value")
        );
        assert!(details.has_body);
        assert!(details.has_responses);
        assert!(details.is_callable);
    }

    fn empty_leaf() -> Command {
        Command {
            description: None,
            endpoint: Some("/x".to_string()),
            method: Some(HttpMethod::GET),
            auth: None,
            headers: HashMap::new(),
            params: HashMap::new(),
            body: None,
            responses: None,
            commands: None,
        }
    }
}
