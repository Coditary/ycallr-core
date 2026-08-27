mod types;
mod response;
mod command;

pub use types::*;

impl ApiDefinition {
    pub fn validate(&self) -> crate::Result<()> {
        if self.name.is_empty() {
            return Err(crate::YcallrError::InvalidDefinition(
                "API name cannot be empty".into(),
            ));
        }
        if !self.name.chars().all(|c| c.is_alphanumeric() || c == '-') {
            return Err(crate::YcallrError::InvalidDefinition(
                "API name must be alphanumeric or dash".into(),
            ));
        }
        if self.base_url.is_empty() {
            return Err(crate::YcallrError::InvalidDefinition(
                "Base URL cannot be empty".into(),
            ));
        }
        Ok(())
    }

    pub fn get_command(&self, path: &str) -> crate::Result<&Command> {
        let parts: Vec<&str> = path.split('.').collect();
        self.get_command_recursive(&parts)
    }

    fn get_command_recursive(&self, parts: &[&str]) -> crate::Result<&Command> {
        let name = parts[0];
        let cmd = self
            .commands
            .get(name)
            .ok_or_else(|| crate::YcallrError::CommandNotFound(name.into()))?;

        if parts.len() == 1 {
            Ok(cmd)
        } else {
            cmd.get_command_recursive(&parts[1..])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn create_test_api() -> ApiDefinition {
        let mut commands = HashMap::new();
        let mut params = HashMap::new();

        params.insert(
            "owner".to_string(),
            Parameter {
                description: "Repository owner".to_string(),
                param_type: ParamType::String,
                required: true,
            },
        );

        params.insert(
            "repo".to_string(),
            Parameter {
                description: "Repository name".to_string(),
                param_type: ParamType::String,
                required: true,
            },
        );

        let mut headers = HashMap::new();
        headers.insert(
            "Accept".to_string(),
            "application/vnd.github.v3+json".to_string(),
        );

        commands.insert(
            "create-issue".to_string(),
            Command {
                description: Some("Create a new issue".to_string()),
                endpoint: Some("/repos/{owner}/{repo}/issues".to_string()),
                method: Some(HttpMethod::POST),
                headers,
                params,
                body: None,
                responses: None,
                commands: None,
            },
        );

        ApiDefinition {
            name: "github".to_string(),
            version: "1.0.0".to_string(),
            description: "GitHub API".to_string(),
            base_url: "https://api.github.com".to_string(),
            env: vec![],
            commands,
        }
    }

    fn create_nested_api() -> ApiDefinition {
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
                body: None,
                responses: None,
                commands: None,
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
                body: None,
                responses: None,
                commands: None,
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
                body: None,
                responses: None,
                commands: Some(issues_commands),
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
                body: None,
                responses: None,
                commands: Some(repos_commands),
            },
        );

        ApiDefinition {
            name: "github".to_string(),
            version: "1.0.0".to_string(),
            description: "GitHub API".to_string(),
            base_url: "https://api.github.com".to_string(),
            env: vec![EnvVar {
                name: "GITHUB_TOKEN".to_string(),
                required: true,
            }],
            commands,
        }
    }

    #[test]
    fn test_api_definition_validate_valid() {
        let api = create_test_api();
        assert!(api.validate().is_ok());
    }

    #[test]
    fn test_api_definition_validate_empty_name() {
        let mut api = create_test_api();
        api.name = "".to_string();
        assert!(api.validate().is_err());
    }

    #[test]
    fn test_api_definition_validate_invalid_name() {
        let mut api = create_test_api();
        api.name = "invalid name!".to_string();
        assert!(api.validate().is_err());
    }

    #[test]
    fn test_api_definition_validate_empty_base_url() {
        let mut api = create_test_api();
        api.base_url = "".to_string();
        assert!(api.validate().is_err());
    }

    #[test]
    fn test_get_command_exists() {
        let api = create_test_api();
        let cmd = api.get_command("create-issue");
        assert!(cmd.is_ok());
        assert_eq!(cmd.unwrap().method.as_ref().unwrap(), &HttpMethod::POST);
    }

    #[test]
    fn test_get_command_not_exists() {
        let api = create_test_api();
        let cmd = api.get_command("nonexistent");
        assert!(cmd.is_err());
    }

    #[test]
    fn test_get_nested_command() {
        let api = create_nested_api();
        let cmd = api.get_command("repos.issues.create");
        assert!(cmd.is_ok());
        let cmd = cmd.unwrap();
        assert_eq!(cmd.method.as_ref().unwrap(), &HttpMethod::POST);
        assert_eq!(
            cmd.endpoint.as_deref(),
            Some("/repos/{owner}/{repo}/issues")
        );
    }

    #[test]
    fn test_get_nested_command_not_exists() {
        let api = create_nested_api();
        let cmd = api.get_command("repos.nonexistent");
        assert!(cmd.is_err());
    }

    #[test]
    fn test_get_nested_command_deep_not_exists() {
        let api = create_nested_api();
        let cmd = api.get_command("repos.issues.nonexistent");
        assert!(cmd.is_err());
    }

    #[test]
    fn test_get_command_branch_only() {
        let api = create_nested_api();
        let cmd = api.get_command("repos");
        assert!(cmd.is_ok());
        let cmd = cmd.unwrap();
        assert!(cmd.is_branch());
        assert!(cmd.is_leaf());
    }

    #[test]
    fn test_resolve_endpoint() {
        let cmd = Command {
            description: None,
            endpoint: Some("/repos/{owner}/{repo}/issues".to_string()),
            method: Some(HttpMethod::POST),
            headers: HashMap::new(),
            params: HashMap::new(),
            body: None,
            responses: None,
            commands: None,
        };

        let mut params = HashMap::new();
        params.insert("owner".to_string(), "rust-lang".to_string());
        params.insert("repo".to_string(), "rust".to_string());

        let resolved = cmd.resolve_endpoint(&params).unwrap();
        assert_eq!(resolved, "/repos/rust-lang/rust/issues");
    }

    #[test]
    fn test_resolve_endpoint_unresolved() {
        let cmd = Command {
            description: None,
            endpoint: Some("/repos/{owner}/{repo}/issues".to_string()),
            method: Some(HttpMethod::POST),
            headers: HashMap::new(),
            params: HashMap::new(),
            body: None,
            responses: None,
            commands: None,
        };

        let params = HashMap::new();
        let resolved = cmd.resolve_endpoint(&params);
        assert!(resolved.is_err());
    }

    #[test]
    fn test_resolve_endpoint_no_endpoint() {
        let cmd = Command {
            description: None,
            endpoint: None,
            method: None,
            headers: HashMap::new(),
            params: HashMap::new(),
            body: None,
            responses: None,
            commands: Some(HashMap::new()),
        };

        let params = HashMap::new();
        let resolved = cmd.resolve_endpoint(&params);
        assert!(resolved.is_err());
    }

    #[test]
    fn test_http_method_as_str() {
        assert_eq!(HttpMethod::GET.as_str(), "GET");
        assert_eq!(HttpMethod::POST.as_str(), "POST");
        assert_eq!(HttpMethod::PUT.as_str(), "PUT");
        assert_eq!(HttpMethod::DELETE.as_str(), "DELETE");
        assert_eq!(HttpMethod::PATCH.as_str(), "PATCH");
    }

    #[test]
    fn test_command_is_leaf() {
        let leaf = Command {
            description: None,
            endpoint: Some("/test".to_string()),
            method: Some(HttpMethod::GET),
            headers: HashMap::new(),
            params: HashMap::new(),
            body: None,
            responses: None,
            commands: None,
        };
        assert!(leaf.is_leaf());

        let branch = Command {
            description: None,
            endpoint: None,
            method: None,
            headers: HashMap::new(),
            params: HashMap::new(),
            body: None,
            responses: None,
            commands: Some(HashMap::new()),
        };
        assert!(!branch.is_leaf());
    }

    #[test]
    fn test_command_is_branch() {
        let mut sub_commands = HashMap::new();
        sub_commands.insert(
            "sub".to_string(),
            Command {
                description: None,
                endpoint: Some("/sub".to_string()),
                method: Some(HttpMethod::GET),
                headers: HashMap::new(),
                params: HashMap::new(),
                body: None,
                responses: None,
                commands: None,
            },
        );
        let branch = Command {
            description: None,
            endpoint: None,
            method: None,
            headers: HashMap::new(),
            params: HashMap::new(),
            body: None,
            responses: None,
            commands: Some(sub_commands),
        };
        assert!(branch.is_branch());

        let leaf = Command {
            description: None,
            endpoint: Some("/test".to_string()),
            method: Some(HttpMethod::GET),
            headers: HashMap::new(),
            params: HashMap::new(),
            body: None,
            responses: None,
            commands: None,
        };
        assert!(!leaf.is_branch());
    }

    #[test]
    fn test_command_description() {
        let cmd = create_test_api().commands.remove("create-issue").unwrap();
        assert_eq!(cmd.description, Some("Create a new issue".to_string()));
    }

    #[test]
    fn test_command_path_alias() {
        let yaml = r#"
name: test
version: "1.0.0"
base_url: https://api.test.com
commands:
  test:
    path: /test
    method: GET
"#;
        let api = crate::yaml_parser::parse_yaml(yaml).unwrap();
        let cmd = api.commands.get("test").unwrap();
        assert_eq!(cmd.endpoint.as_deref(), Some("/test"));
    }

    #[test]
    fn test_env_var_default_required() {
        let yaml = r#"
name: test
version: "1.0.0"
base_url: https://api.test.com
env:
  - name: TOKEN
commands:
  test:
    endpoint: /test
    method: GET
"#;
        let api = crate::yaml_parser::parse_yaml(yaml).unwrap();
        assert_eq!(api.env.len(), 1);
        assert_eq!(api.env[0].name, "TOKEN");
        assert!(api.env[0].required);
    }

    #[test]
    fn test_env_var_not_required() {
        let yaml = r#"
name: test
version: "1.0.0"
base_url: https://api.test.com
env:
  - name: TOKEN
    required: false
commands:
  test:
    endpoint: /test
    method: GET
"#;
        let api = crate::yaml_parser::parse_yaml(yaml).unwrap();
        assert_eq!(api.env.len(), 1);
        assert!(!api.env[0].required);
    }

    #[test]
    fn test_env_empty() {
        let api = create_test_api();
        assert!(api.env.is_empty());
    }

    #[test]
    fn test_response_config_parsing() {
        let yaml = r#"
name: test
version: "1.0.0"
base_url: https://api.test.com
commands:
  create:
    endpoint: /items
    method: POST
    responses:
      success:
        message: "Created {output.title}"
      failure:
        message: "Failed to create"
      404:
        message: "{input.owner} not found"
"#;
        let api = crate::yaml_parser::parse_yaml(yaml).unwrap();
        let cmd = api.commands.get("create").unwrap();
        let responses = cmd.responses.as_ref().unwrap();

        assert_eq!(
            responses.success.as_ref().unwrap().message,
            "Created {output.title}"
        );
        assert_eq!(
            responses.failure.as_ref().unwrap().message,
            "Failed to create"
        );
        assert_eq!(
            responses.codes.get("404").unwrap().message,
            "{input.owner} not found"
        );
    }

    #[test]
    fn test_response_config_get_entry_for_status() {
        let mut codes = HashMap::new();
        codes.insert(
            "404".to_string(),
            ResponseEntry {
                message: "Not found".to_string(),
            },
        );
        let config = ResponseConfig {
            success: Some(ResponseEntry {
                message: "OK".to_string(),
            }),
            failure: Some(ResponseEntry {
                message: "Error".to_string(),
            }),
            warn: None,
            codes,
        };

        assert_eq!(config.get_entry_for_status(200).unwrap().message, "OK");
        assert_eq!(config.get_entry_for_status(201).unwrap().message, "OK");
        assert_eq!(
            config.get_entry_for_status(404).unwrap().message,
            "Not found"
        );
        assert_eq!(config.get_entry_for_status(500).unwrap().message, "Error");
        assert!(config.get_entry_for_status(301).is_none());
    }

    #[test]
    fn test_response_config_exact_code_takes_priority() {
        let mut codes = HashMap::new();
        codes.insert(
            "200".to_string(),
            ResponseEntry {
                message: "Exact 200".to_string(),
            },
        );
        let config = ResponseConfig {
            success: Some(ResponseEntry {
                message: "Any success".to_string(),
            }),
            failure: None,
            warn: None,
            codes,
        };

        assert_eq!(
            config.get_entry_for_status(200).unwrap().message,
            "Exact 200"
        );
        assert_eq!(
            config.get_entry_for_status(201).unwrap().message,
            "Any success"
        );
    }

    #[test]
    fn test_response_config_empty() {
        let config = ResponseConfig {
            success: None,
            failure: None,
            warn: None,
            codes: HashMap::new(),
        };

        assert!(config.get_entry_for_status(200).is_none());
        assert!(config.get_entry_for_status(404).is_none());
    }
}
