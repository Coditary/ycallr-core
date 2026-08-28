use std::collections::HashMap;

use crate::error::{Result, YcallrError};
use crate::models::ApiDefinition;

use super::types::{AuthConfig, EnvMode};
use super::YcallrClient;

pub struct YcallrClientBuilder {
    pub(crate) api: ApiDefinition,
    pub(crate) auth: Option<AuthConfig>,
    pub(crate) env_mode: EnvMode,
    pub(crate) env_vars: HashMap<String, String>,
}

impl YcallrClientBuilder {
    pub fn new(api: ApiDefinition) -> Self {
        Self {
            api,
            auth: None,
            env_mode: EnvMode::Auto,
            env_vars: HashMap::new(),
        }
    }

    pub fn auth(mut self, auth: AuthConfig) -> Self {
        self.auth = Some(auth);
        self
    }

    pub fn env_mode(mut self, mode: EnvMode) -> Self {
        self.env_mode = mode;
        self
    }

    pub fn env(mut self, key: &str, value: &str) -> Self {
        self.env_vars.insert(key.to_string(), value.to_string());
        self
    }

    pub fn envs(mut self, vars: HashMap<String, String>) -> Self {
        self.env_vars.extend(vars);
        self
    }

    pub fn build(self) -> Result<YcallrClient> {
        let mut resolved_env = HashMap::new();

        for env_var in &self.api.env {
            match self.env_mode {
                EnvMode::Auto => {
                    if let Ok(val) = std::env::var(&env_var.name) {
                        resolved_env.insert(env_var.name.clone(), val);
                    } else if let Some(val) = self.env_vars.get(&env_var.name) {
                        resolved_env.insert(env_var.name.clone(), val.clone());
                    }
                }
                EnvMode::Manual => {
                    if let Some(val) = self.env_vars.get(&env_var.name) {
                        resolved_env.insert(env_var.name.clone(), val.clone());
                    }
                }
            }
        }

        for (key, value) in &self.env_vars {
            resolved_env
                .entry(key.clone())
                .or_insert_with(|| value.clone());
        }

        for env_var in &self.api.env {
            if env_var.required && !resolved_env.contains_key(&env_var.name) {
                return Err(YcallrError::EnvVar(format!(
                    "Required environment variable '{}' is not set",
                    env_var.name
                )));
            }
        }

        let http_client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| YcallrError::HttpClient(e.to_string()))?;

        let auth_configs = self.api.auth.clone();

        Ok(YcallrClient {
            api: self.api,
            http_client,
            auth: self.auth,
            auth_configs,
            env_mode: self.env_mode,
            env_vars: resolved_env,
        })
    }
}
