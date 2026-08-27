mod mock;
mod apis;
mod helpers;

pub use mock::*;
pub use apis::*;
pub use helpers::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::HttpMethod;
    use std::collections::HashMap;

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
