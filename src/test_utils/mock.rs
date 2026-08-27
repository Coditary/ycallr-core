use crate::client::ApiResponse;
use std::collections::HashMap;

pub struct MockApiClient {
    responses: HashMap<String, ApiResponse>,
    calls: Vec<MockCall>,
}

#[derive(Debug, Clone)]
pub struct MockCall {
    pub command: String,
    pub params: HashMap<String, String>,
    pub body: Option<serde_json::Value>,
}

impl MockApiClient {
    pub fn new() -> Self {
        Self {
            responses: HashMap::new(),
            calls: Vec::new(),
        }
    }

    pub fn expect(&mut self, command: &str, response: ApiResponse) -> &mut Self {
        self.responses.insert(command.to_string(), response);
        self
    }

    pub fn call(
        &mut self,
        command: &str,
        params: &HashMap<String, String>,
        body: Option<&serde_json::Value>,
    ) -> crate::Result<ApiResponse> {
        self.calls.push(MockCall {
            command: command.to_string(),
            params: params.clone(),
            body: body.cloned(),
        });

        self.responses
            .get(command)
            .cloned()
            .ok_or_else(|| crate::YcallrError::CommandNotFound(command.to_string()))
    }

    pub fn calls(&self) -> &[MockCall] {
        &self.calls
    }

    pub fn last_call(&self) -> Option<&MockCall> {
        self.calls.last()
    }

    pub fn was_called(&self, command: &str) -> bool {
        self.calls.iter().any(|c| c.command == command)
    }

    pub fn call_count(&self, command: &str) -> usize {
        self.calls.iter().filter(|c| c.command == command).count()
    }
}
