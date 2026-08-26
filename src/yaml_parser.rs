use crate::error::{Result, YcallrError};
use crate::models::ApiDefinition;

pub fn parse_yaml(yaml_content: &str) -> Result<ApiDefinition> {
    let api: ApiDefinition =
        serde_yaml::from_str(yaml_content).map_err(|e| YcallrError::YamlParse(e.to_string()))?;

    api.validate()?;

    Ok(api)
}

pub fn parse_yaml_file(path: &std::path::Path) -> Result<ApiDefinition> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| YcallrError::YamlParse(format!("Failed to read file: {}", e)))?;

    parse_yaml(&content)
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_YAML: &str = r#"
name: github
version: "1.0.0"
description: GitHub API
base_url: https://api.github.com
commands:
  create-issue:
    endpoint: /repos/{owner}/{repo}/issues
    method: POST
    headers:
      Accept: application/vnd.github.v3+json
    params:
      owner:
        description: Repository owner
        type: string
        required: true
      repo:
        description: Repository name
        type: string
        required: true
"#;

    #[test]
    fn test_parse_valid_yaml() {
        let api = parse_yaml(VALID_YAML).unwrap();
        assert_eq!(api.name, "github");
        assert_eq!(api.version, "1.0.0");
        assert_eq!(api.base_url, "https://api.github.com");
        assert!(api.commands.contains_key("create-issue"));
    }

    #[test]
    fn test_parse_yaml_missing_name() {
        let yaml = r#"
version: "1.0.0"
base_url: https://api.github.com
commands: {}
"#;
        let result = parse_yaml(yaml);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_yaml_invalid_method() {
        let yaml = r#"
name: test
version: "1.0.0"
base_url: https://api.test.com
commands:
  test:
    endpoint: /test
    method: INVALID
    headers: {}
    params: {}
"#;
        let result = parse_yaml(yaml);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_yaml_empty_commands() {
        let yaml = r#"
name: test
version: "1.0.0"
base_url: https://api.test.com
commands: {}
"#;
        let api = parse_yaml(yaml).unwrap();
        assert!(api.commands.is_empty());
    }

    #[test]
    fn test_parse_yaml_with_defaults() {
        let yaml = r#"
name: test
version: "1.0.0"
base_url: https://api.test.com
commands:
  test:
    endpoint: /test
    method: GET
"#;
        let api = parse_yaml(yaml).unwrap();
        let cmd = api.commands.get("test").unwrap();
        assert!(cmd.headers.is_empty());
        assert!(cmd.params.is_empty());
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn test_parse_yaml_file_valid() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.yaml");
        std::fs::write(&file_path, VALID_YAML).unwrap();

        let api = parse_yaml_file(&file_path).unwrap();
        assert_eq!(api.name, "github");
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn test_parse_yaml_file_not_found() {
        let path = std::path::Path::new("/nonexistent/path/test.yaml");
        let result = parse_yaml_file(path);
        assert!(result.is_err());
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn test_parse_yaml_file_invalid_content() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("bad.yaml");
        std::fs::write(&file_path, "not: valid: yaml: {{{").unwrap();

        let result = parse_yaml_file(&file_path);
        assert!(result.is_err());
    }
}
