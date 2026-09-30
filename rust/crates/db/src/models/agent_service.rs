//! Agents::ServiceResult, shared by HTTP and MCP adapters.
use serde_json::{Value, json};

#[derive(Debug, Clone)]
pub struct ServiceResult {
    pub payload: Option<Value>,
    pub error: Option<String>,
    pub status: u16,
}
impl ServiceResult {
    pub fn ok(payload: Value, status: u16) -> Self {
        Self {
            payload: Some(payload),
            error: None,
            status,
        }
    }
    pub fn fail(error: impl Into<String>, status: u16) -> Self {
        Self {
            payload: None,
            error: Some(error.into()),
            status,
        }
    }
    pub fn budget(payload: Value) -> Self {
        Self {
            error: payload["error"].as_str().map(str::to_owned),
            payload: Some(payload),
            status: 429,
        }
    }
    pub fn is_ok(&self) -> bool {
        self.error.is_none()
    }
    pub fn failure_body(&self) -> Value {
        self.payload
            .clone()
            .unwrap_or_else(|| json!({"error":self.error}))
    }
}
