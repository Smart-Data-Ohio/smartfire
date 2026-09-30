//! WS15e's shared Fizzy reads; authenticated REST/MCP adapters use this domain layer.
pub mod accounts;
pub mod agent_reads;
#[allow(dead_code, reason = "Fizzy action execution remains with WS15e's approval job")]
pub mod agent_action;
pub mod agent_requests;
pub mod client;
pub(super) mod error_body;
use serde_json::Value;
pub(crate) fn blank(value: &Value) -> bool {
    match value {
        Value::Null | Value::Bool(false) => true,
        Value::String(s) => s.chars().all(char::is_whitespace),
        Value::Array(a) => a.is_empty(),
        Value::Object(o) => o.is_empty(),
        _ => false,
    }
}

/// Per-app transport avoids process-wide network/config overrides in concurrent HTTP tests.
pub struct State {
    pub network: super::net::Network,
    pub base: String,
}
impl State {
    pub fn system() -> Self {
        Self {
            network: super::net::Network::system(),
            base: client::api_base_url(),
        }
    }
}
