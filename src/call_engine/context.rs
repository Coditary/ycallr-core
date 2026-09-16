use std::collections::HashMap;

use crate::error::{Result, YcallrError};
use crate::models::{ApiDefinition, AuthConfig};

use crate::call_engine::types::EnvMode;

#[derive(Debug, Clone)]
pub struct ClientContext {
    pub api: ApiDefinition,
    pub auth: Option<AuthConfig>,
    pub auth_configs: HashMap<String, AuthConfig>,
    pub env_vars: HashMap<String, String>,
}

pub fn resolve_client_env(
    api: &ApiDefinition,
    env_mode: &EnvMode,
    env_vars: &HashMap<String, String>,
) -> Result<HashMap<String, String>> {
    for key in env_vars.keys() {
        validate_declared_env_key(api, key)?;
    }

    let mut resolved_env = HashMap::new();

    for env_var in &api.env {
        match env_mode {
            EnvMode::Auto => {
                if let Some(val) = read_env_value(&env_var.name) {
                    resolved_env.insert(env_var.name.clone(), val);
                } else if let Some(val) = env_vars.get(&env_var.name) {
                    resolved_env.insert(env_var.name.clone(), val.clone());
                }
            }
            EnvMode::Manual => {
                if let Some(val) = env_vars.get(&env_var.name) {
                    resolved_env.insert(env_var.name.clone(), val.clone());
                }
            }
        }
    }

    for env_var in &api.env {
        if !env_var.required && !resolved_env.contains_key(&env_var.name) {
            resolved_env.insert(env_var.name.clone(), String::new());
        }
    }

    validate_resolved_env_vars(api, &resolved_env)?;

    Ok(resolved_env)
}

/// Read an env value from `VAR` or `VAR_FILE` (file path whose contents are the secret).
pub fn read_env_value(name: &str) -> Option<String> {
    if let Ok(val) = std::env::var(name) {
        if !val.is_empty() {
            return Some(val);
        }
    }

    let file_key = format!("{}_FILE", name);
    if let Ok(path) = std::env::var(&file_key) {
        return read_secret_file(&path).ok();
    }

    None
}

fn read_secret_file(path: &str) -> Result<String> {
    let contents = std::fs::read_to_string(path).map_err(|e| {
        YcallrError::EnvVar(format!("Failed to read secret file '{}': {}", path, e))
    })?;
    let value = contents.trim_end_matches(['\n', '\r']).to_string();
    if value.is_empty() {
        return Err(YcallrError::EnvVar(format!(
            "Secret file '{}' is empty",
            path
        )));
    }
    Ok(value)
}

fn validate_declared_env_key(api: &ApiDefinition, key: &str) -> Result<()> {
    if api.env.iter().any(|e| e.name == key) {
        Ok(())
    } else {
        Err(YcallrError::EnvVar(format!(
            "Environment variable '{}' is not declared in the API profile",
            key
        )))
    }
}

fn validate_resolved_env_vars(
    api: &ApiDefinition,
    resolved_env: &HashMap<String, String>,
) -> Result<()> {
    for env_var in &api.env {
        if !env_var.required {
            continue;
        }

        match resolved_env.get(&env_var.name) {
            None => {
                return Err(YcallrError::EnvVar(format!(
                    "Required environment variable '{}' is not set (set {}, {}_FILE, use --env-file, or enter interactively)",
                    env_var.name,
                    env_var.name,
                    env_var.name
                )));
            }
            Some(value) if value.trim().is_empty() => {
                return Err(YcallrError::EnvVar(format!(
                    "Required environment variable '{}' cannot be empty",
                    env_var.name
                )));
            }
            Some(_) => {}
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ApiDefinition;

    #[test]
    fn test_resolve_client_env_auto_reads_os_env() {
        let key = "YCALLR_COVERAGE_ENV_TEST";
        std::env::set_var(key, "from-os");

        let api = ApiDefinition {
            name: "env".to_string(),
            version: "1".to_string(),
            description: "".to_string(),
            base_url: "https://api.test.com".to_string(),
            env: vec![crate::models::EnvVar {
                name: key.to_string(),
                required: false,
            }],
            auth: HashMap::new(),
            commands: HashMap::new(),
            errors: None,
        };

        let resolved = resolve_client_env(&api, &EnvMode::Auto, &HashMap::new()).unwrap();
        assert_eq!(resolved.get(key), Some(&"from-os".to_string()));

        std::env::remove_var(key);
    }

    #[test]
    fn test_resolve_client_env_auto_falls_back_to_manual_vars() {
        let key = "YCALLR_COVERAGE_FALLBACK";
        std::env::remove_var(key);

        let api = ApiDefinition {
            name: "env".to_string(),
            version: "1".to_string(),
            description: "".to_string(),
            base_url: "https://api.test.com".to_string(),
            env: vec![crate::models::EnvVar {
                name: key.to_string(),
                required: false,
            }],
            auth: HashMap::new(),
            commands: HashMap::new(),
            errors: None,
        };

        let manual = HashMap::from([(key.to_string(), "manual".to_string())]);
        let resolved = resolve_client_env(&api, &EnvMode::Auto, &manual).unwrap();
        assert_eq!(resolved.get(key), Some(&"manual".to_string()));
    }

    #[test]
    fn test_read_env_value_from_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("token.txt");
        std::fs::write(&path, "secret-from-file\n").unwrap();

        let key = "YCALLR_FILE_TEST_TOKEN";
        std::env::remove_var(key);
        std::env::set_var(format!("{}_FILE", key), path);

        assert_eq!(read_env_value(key), Some("secret-from-file".to_string()));

        std::env::remove_var(format!("{}_FILE", key));
    }
}
