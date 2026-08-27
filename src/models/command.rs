use std::collections::HashMap;

use crate::models::{Command, HttpMethod};

impl Command {
    pub fn resolve_endpoint(&self, params: &HashMap<String, String>) -> crate::Result<String> {
        let endpoint = self
            .endpoint
            .as_deref()
            .ok_or_else(|| crate::YcallrError::ParamValidation("Command has no endpoint".into()))?;

        let mut resolved = endpoint.to_string();
        for (key, value) in params {
            resolved = resolved.replace(&format!("{{{}}}", key), value);
        }

        let unresolved: Vec<_> = resolved.matches('{').collect();
        if !unresolved.is_empty() {
            return Err(crate::YcallrError::ParamValidation(format!(
                "Unresolved parameters in endpoint: {}",
                resolved
            )));
        }

        Ok(resolved)
    }

    pub fn get_command_recursive(&self, parts: &[&str]) -> crate::Result<&Command> {
        let name = parts[0];
        let commands = self.commands.as_ref().ok_or_else(|| {
            crate::YcallrError::CommandNotFound(format!("{} has no sub-commands", name))
        })?;

        let cmd = commands
            .get(name)
            .ok_or_else(|| crate::YcallrError::CommandNotFound(name.into()))?;

        if parts.len() == 1 {
            Ok(cmd)
        } else {
            cmd.get_command_recursive(&parts[1..])
        }
    }

    pub fn is_leaf(&self) -> bool {
        self.endpoint.is_some() && self.method.is_some()
    }

    pub fn is_branch(&self) -> bool {
        self.commands.is_some() && !self.commands.as_ref().unwrap().is_empty()
    }
}

impl HttpMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            HttpMethod::GET => "GET",
            HttpMethod::POST => "POST",
            HttpMethod::PUT => "PUT",
            HttpMethod::DELETE => "DELETE",
            HttpMethod::PATCH => "PATCH",
        }
    }
}
