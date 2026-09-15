//! Resolve `include:` fragments when parsing API profile YAML from disk.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::error::{Result, YcallrError};
use crate::models::{
    ApiDefinition, ApiErrorConfig, AuthConfig, BodyConfig, Command, EnvVar, HttpMethod, Parameter,
    ResponseConfig,
};

#[derive(Debug, Clone, Deserialize)]
struct ApiDefinitionRaw {
    name: String,
    version: String,
    #[serde(default)]
    description: String,
    base_url: String,
    #[serde(default)]
    env: Vec<EnvVar>,
    #[serde(default)]
    auth: HashMap<String, AuthConfig>,
    commands: HashMap<String, CommandWithInclude>,
    #[serde(default)]
    errors: Option<ApiErrorConfig>,
}

#[derive(Debug, Clone, Deserialize)]
struct CommandWithInclude {
    #[serde(default)]
    description: Option<String>,
    #[serde(default, alias = "path")]
    endpoint: Option<String>,
    #[serde(default)]
    method: Option<HttpMethod>,
    #[serde(default)]
    auth: Option<String>,
    #[serde(default)]
    headers: HashMap<String, String>,
    #[serde(default)]
    params: HashMap<String, Parameter>,
    #[serde(default)]
    body: Option<BodyConfig>,
    #[serde(default)]
    responses: Option<ResponseConfig>,
    #[serde(default)]
    include: Option<String>,
    #[serde(default)]
    commands: Option<HashMap<String, CommandWithInclude>>,
}

/// Parse YAML and resolve `include:` references relative to `base_dir`.
///
/// When `base_dir` is `None` (in-memory YAML), any `include` key is rejected.
pub fn resolve_api_from_yaml(yaml: &str, base_dir: Option<&Path>) -> Result<ApiDefinition> {
    let raw: ApiDefinitionRaw = serde_yaml::from_str(yaml)
        .map_err(|e| YcallrError::YamlParse(e.to_string()))?;

    let mut stack = HashSet::new();
    let commands = resolve_commands_map(&raw.commands, "commands", base_dir, &mut stack)?;

    Ok(ApiDefinition {
        name: raw.name,
        version: raw.version,
        description: raw.description,
        base_url: raw.base_url,
        env: raw.env,
        auth: raw.auth,
        commands,
        errors: raw.errors,
    })
}

fn resolve_commands_map(
    commands: &HashMap<String, CommandWithInclude>,
    path_prefix: &str,
    base_dir: Option<&Path>,
    stack: &mut HashSet<PathBuf>,
) -> Result<HashMap<String, Command>> {
    let mut resolved = HashMap::new();
    for (name, cmd) in commands {
        let path = format!("{}.{}", path_prefix, name);
        resolved.insert(name.clone(), resolve_command(cmd.clone(), &path, base_dir, stack)?);
    }
    Ok(resolved)
}

fn resolve_command(
    cmd: CommandWithInclude,
    command_path: &str,
    base_dir: Option<&Path>,
    stack: &mut HashSet<PathBuf>,
) -> Result<Command> {
    if let Some(include) = &cmd.include {
        if cmd.commands.is_some() {
            return Err(YcallrError::InvalidDefinition(format!(
                "Command '{}': cannot use both 'include' and inline 'commands'",
                command_path
            )));
        }
        if include.trim().is_empty() {
            return Err(YcallrError::InvalidDefinition(format!(
                "Command '{}': 'include' path cannot be empty",
                command_path
            )));
        }

        let Some(base_dir) = base_dir else {
            return Err(YcallrError::InvalidDefinition(format!(
                "Command '{}': 'include' requires parsing from a YAML file on disk",
                command_path
            )));
        };

        let include_path = resolve_include_path(base_dir, include);
        let canonical = canonicalize_include_path(&include_path, command_path)?;

        if stack.contains(&canonical) {
            return Err(YcallrError::InvalidDefinition(format!(
                "Command '{}': circular include detected for '{}'",
                command_path,
                include_path.display()
            )));
        }

        stack.insert(canonical.clone());
        let fragment_dir = include_path.parent().unwrap_or(base_dir);
        let children = load_command_fragment(&include_path)?;
        let commands = resolve_commands_map(&children, command_path, Some(fragment_dir), stack)?;
        stack.remove(&canonical);

        return Ok(command_shell(&cmd, Some(commands)));
    }

    let commands = match &cmd.commands {
        Some(children) => {
            let resolved = resolve_commands_map(children, command_path, base_dir, stack)?;
            if resolved.is_empty() {
                None
            } else {
                Some(resolved)
            }
        }
        None => None,
    };

    Ok(command_shell(&cmd, commands))
}

fn command_shell(cmd: &CommandWithInclude, commands: Option<HashMap<String, Command>>) -> Command {
    Command {
        description: cmd.description.clone(),
        endpoint: cmd.endpoint.clone(),
        method: cmd.method.clone(),
        auth: cmd.auth.clone(),
        headers: cmd.headers.clone(),
        params: cmd.params.clone(),
        body: cmd.body.clone(),
        responses: cmd.responses.clone(),
        commands,
    }
}

fn resolve_include_path(base_dir: &Path, include: &str) -> PathBuf {
    let path = Path::new(include);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base_dir.join(path)
    }
}

fn canonicalize_include_path(path: &Path, command_path: &str) -> Result<PathBuf> {
    path.canonicalize().map_err(|e| {
        YcallrError::InvalidDefinition(format!(
            "Command '{}': failed to read include '{}': {}",
            command_path,
            path.display(),
            e
        ))
    })
}

fn load_command_fragment(path: &Path) -> Result<HashMap<String, CommandWithInclude>> {
    let content = std::fs::read_to_string(path).map_err(|e| {
        YcallrError::InvalidDefinition(format!(
            "Failed to read include file '{}': {}",
            path.display(),
            e
        ))
    })?;

    let root: serde_yaml::Value = serde_yaml::from_str(&content).map_err(|e| {
        YcallrError::YamlParse(format!(
            "Failed to parse include file '{}': {}",
            path.display(),
            e
        ))
    })?;

    let commands_value = if let Some(commands) = root.get("commands") {
        if root.as_mapping().is_some_and(|map| map.len() == 1) {
            commands.clone()
        } else {
            return Err(YcallrError::InvalidDefinition(format!(
                "Include file '{}' cannot combine top-level 'commands' with other keys",
                path.display()
            )));
        }
    } else {
        root
    };

    serde_yaml::from_value(commands_value).map_err(|e| {
        YcallrError::YamlParse(format!(
            "Failed to parse include file '{}': {}",
            path.display(),
            e
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn write(dir: &Path, name: &str, content: &str) -> PathBuf {
        let path = dir.join(name);
        fs::write(&path, content).unwrap();
        path
    }

    const MAIN: &str = r#"
name: demo
version: "1.0.0"
base_url: https://api.example.com
env:
  - name: API_TOKEN
    required: true
auth:
  bearer:
    type: bearer
    token: ${API_TOKEN}
commands:
  repos:
    include: ./repos.yaml
"#;

    const REPOS: &str = r#"
list:
  endpoint: /user/repos
  method: GET
  auth: bearer
get:
  endpoint: /repos/{owner}/{repo}
  method: GET
  auth: bearer
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
    fn test_resolve_flat_include() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "repos.yaml", REPOS);
        write(dir.path(), "main.yaml", MAIN);

        let content = fs::read_to_string(dir.path().join("main.yaml")).unwrap();
        let api = resolve_api_from_yaml(&content, Some(dir.path())).unwrap();
        let repos = api.commands.get("repos").unwrap();
        let children = repos.commands.as_ref().unwrap();
        assert!(children.contains_key("list"));
        assert!(children.contains_key("get"));
        assert_eq!(
            children.get("list").unwrap().endpoint.as_deref(),
            Some("/user/repos")
        );
    }

    #[test]
    fn test_resolve_wrapped_include() {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "repos.yaml",
            "commands:\n  list:\n    endpoint: /user/repos\n    method: GET\n    auth: bearer\n",
        );
        write(dir.path(), "main.yaml", MAIN);

        let content = fs::read_to_string(dir.path().join("main.yaml")).unwrap();
        let api = resolve_api_from_yaml(&content, Some(dir.path())).unwrap();
        assert!(api
            .commands
            .get("repos")
            .unwrap()
            .commands
            .as_ref()
            .unwrap()
            .contains_key("list"));
    }

    #[test]
    fn test_hybrid_command_with_include() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "repos.yaml", REPOS);
        let main = r#"
name: demo
version: "1.0.0"
base_url: https://api.example.com
auth:
  bearer:
    type: bearer
    token: tok
commands:
  repos:
    description: Repositories
    endpoint: /user/repos
    method: GET
    auth: bearer
    include: ./repos.yaml
"#;
        write(dir.path(), "main.yaml", main);

        let content = fs::read_to_string(dir.path().join("main.yaml")).unwrap();
        let api = resolve_api_from_yaml(&content, Some(dir.path())).unwrap();
        let repos = api.commands.get("repos").unwrap();
        assert_eq!(repos.description.as_deref(), Some("Repositories"));
        assert_eq!(repos.endpoint.as_deref(), Some("/user/repos"));
        assert_eq!(repos.method.as_ref(), Some(&HttpMethod::GET));
        assert_eq!(repos.commands.as_ref().unwrap().len(), 2);
    }

    #[test]
    fn test_nested_include_relative_to_fragment() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("commands")).unwrap();
        write(
            &dir.path().join("commands"),
            "branches.yaml",
            "list:\n  endpoint: /branches\n  method: GET\n  auth: bearer\n",
        );
        write(
            dir.path(),
            "repos.yaml",
            "branches:\n  include: ./commands/branches.yaml\n",
        );
        write(dir.path(), "main.yaml", MAIN);

        let content = fs::read_to_string(dir.path().join("main.yaml")).unwrap();
        let api = resolve_api_from_yaml(&content, Some(dir.path())).unwrap();
        let branches = api
            .commands
            .get("repos")
            .unwrap()
            .commands
            .as_ref()
            .unwrap()
            .get("branches")
            .unwrap();
        assert!(branches.commands.as_ref().unwrap().contains_key("list"));
    }

    #[test]
    fn test_include_without_base_dir_fails() {
        let err = resolve_api_from_yaml(MAIN, None).unwrap_err();
        assert!(err.to_string().contains("requires parsing from a YAML file"));
    }

    #[test]
    fn test_include_and_commands_fails() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "repos.yaml", REPOS);
        let main = r#"
name: demo
version: "1.0.0"
base_url: https://api.example.com
commands:
  repos:
    include: ./repos.yaml
    commands:
      inline:
        endpoint: /x
        method: GET
"#;
        write(dir.path(), "main.yaml", main);

        let content = fs::read_to_string(dir.path().join("main.yaml")).unwrap();
        let err = resolve_api_from_yaml(&content, Some(dir.path())).unwrap_err();
        assert!(err.to_string().contains("cannot use both 'include' and inline 'commands'"));
    }

    #[test]
    fn test_circular_include_fails() {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "a.yaml",
            "child:\n  include: ./b.yaml\n  endpoint: /a\n  method: GET\n",
        );
        write(
            dir.path(),
            "b.yaml",
            "child:\n  include: ./a.yaml\n  endpoint: /b\n  method: GET\n",
        );
        let main = r#"
name: demo
version: "1.0.0"
base_url: https://api.example.com
commands:
  root:
    include: ./a.yaml
"#;
        write(dir.path(), "main.yaml", main);

        let content = fs::read_to_string(dir.path().join("main.yaml")).unwrap();
        let err = resolve_api_from_yaml(&content, Some(dir.path())).unwrap_err();
        assert!(err.to_string().contains("circular include"));
    }
}
