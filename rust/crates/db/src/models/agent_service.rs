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

/// RecordInvalid is a service denial, so surrounding writes such as a new DM
/// and a budget notice can commit as Rails does.
pub fn invalid(errors: crate::Errors) -> ServiceResult {
    let error = crate::slash_commands::sentence(errors.full_messages());
    let mut fields = serde_json::Map::new();
    for (field, message) in errors.0 {
        fields
            .entry(field.to_owned())
            .or_insert_with(|| json!([]))
            .as_array_mut()
            .expect("array")
            .push(json!(message));
    }
    ServiceResult {
        payload: Some(json!({"errors":fields})),
        error: Some(error),
        status: 422,
    }
}
