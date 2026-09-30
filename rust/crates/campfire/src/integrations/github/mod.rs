//! GitHub domain and fixed-host clients, matching our Rails `app/models/github/`.
//! Controllers and HTML belong outside this module.

pub mod accounts;
pub mod actions;
pub mod agent_actions;
pub mod client;
pub mod fetcher;
pub mod jobs;
pub mod oauth;
pub mod webhooks;

fn blank(value: &str) -> bool {
    value.chars().all(char::is_whitespace)
}

/// ActiveModel::Type::Boolean, shared by fetched and webhook-supplied privacy.
fn boolean(value: &serde_json::Value) -> Option<bool> {
    use serde_json::Value;
    match value {
        Value::Null => None,
        Value::String(value) if value.is_empty() => None,
        Value::Bool(false) => Some(false),
        Value::Number(number) if number.as_f64() == Some(0.0) => Some(false),
        Value::String(value)
            if ["0", "f", "F", "false", "FALSE", "off", "OFF"].contains(&value.as_str()) =>
        {
            Some(false)
        }
        _ => Some(true),
    }
}

#[cfg(test)]
mod tests;
