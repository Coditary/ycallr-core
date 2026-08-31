use std::collections::HashMap;

use crate::call_engine::{resolve_client_env, EnvMode};
use crate::error::Result;
use crate::models::ApiDefinition;

use super::types::AuthConfig;
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

    pub fn build_context(&self) -> Result<crate::call_engine::ClientContext> {
        self.api.validate_for_client()?;
        let resolved_env = resolve_client_env(&self.api, &self.env_mode, &self.env_vars)?;
        Ok(crate::call_engine::ClientContext {
            api: self.api.clone(),
            auth: self.auth.clone(),
            auth_configs: self.api.auth.clone(),
            env_vars: resolved_env,
        })
    }

    pub fn build(self) -> Result<YcallrClient> {
        self.api.validate_for_client()?;

        let resolved_env = resolve_client_env(&self.api, &self.env_mode, &self.env_vars)?;

        let http_client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|e| crate::YcallrError::HttpClient(e.to_string()))?;

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

pub(crate) fn validate_declared_env_key(api: &ApiDefinition, key: &str) -> Result<()> {
    if api.env.iter().any(|e| e.name == key) {
        Ok(())
    } else {
        Err(crate::YcallrError::EnvVar(format!(
            "Environment variable '{}' is not declared in the API profile",
            key
        )))
    }
}
