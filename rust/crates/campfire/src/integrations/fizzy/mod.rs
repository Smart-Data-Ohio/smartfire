//! Fizzy's domain layer. SQL and client policy never render HTML.
pub mod accounts;
pub mod agent_action;
pub mod agent_job;
#[allow(
    dead_code,
    reason = "WS11’s authenticated agent REST/MCP adapters have not merged"
)]
pub mod agent_reads;
#[allow(
    dead_code,
    reason = "WS11's authenticated approval-request adapters have not merged"
)]
pub mod agent_requests;
pub mod cards;
pub mod client;
pub(super) mod error_body;
pub mod fetch;
pub mod urls;
use serde_json::Value;
#[allow(dead_code, reason = "Used by staged Fizzy create-card API")]
pub(crate) fn blank(value: &Value) -> bool {
    match value {
        Value::Null | Value::Bool(false) => true,
        Value::String(s) => s.chars().all(char::is_whitespace),
        Value::Array(a) => a.is_empty(),
        Value::Object(o) => o.is_empty(),
        _ => false,
    }
}
#[cfg(test)]
fn ruby_string(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(s) => s.clone(),
        Value::Array(_) | Value::Object(_) => ruby_inspect(value),
        _ => value.to_string(),
    }
}
#[cfg(test)]
fn ruby_inspect(value: &Value) -> String {
    match value {
        Value::Null => "nil".into(),
        Value::String(s) => serde_json::to_string(s).unwrap(),
        Value::Array(a) => format!(
            "[{}]",
            a.iter().map(ruby_inspect).collect::<Vec<_>>().join(", ")
        ),
        Value::Object(o) => format!(
            "{{{}}}",
            o.iter()
                .map(|(k, v)| format!("{}=>{}", serde_json::to_string(k).unwrap(), ruby_inspect(v)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        _ => value.to_string(),
    }
}
#[cfg(test)]
mod tests;
#[cfg(test)]
mod rails_client_tests;
