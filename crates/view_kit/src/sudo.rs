//! `sudo_replay_fields`: the hidden fields that replay a confirmed request.

use serde_json::Value;

use crate::helpers::{self as h};

/// Reject malformed names and non-scalars again at render time.
pub fn replay_fields(params: &Value, prefix: Option<&str>) -> h::Html {
    let Some(params) = params.as_object() else {
        return h::empty();
    };
    let mut output = String::new();
    for (key, value) in params {
        let name = prefix.map_or_else(|| key.clone(), |prefix| format!("{prefix}[{key}]"));
        if name.is_empty()
            || !name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-' | '[' | ']'))
        {
            continue;
        }
        match value {
            Value::Object(_) => output.push_str(&replay_fields(value, Some(&name)).0),
            Value::Array(values) => {
                for value in values {
                    if let Some(field) = scalar_field(&format!("{name}[]"), value) {
                        output.push_str(&field.0);
                    }
                }
            }
            _ => {
                if let Some(field) = scalar_field(&name, value) {
                    output.push_str(&field.0);
                }
            }
        }
    }
    h::raw(output)
}

fn scalar_field(name: &str, value: &Value) -> Option<h::Html> {
    let value = match value {
        Value::Object(_) | Value::Array(_) => return None,
        Value::Null => None,
        Value::String(value) => Some(value.clone()),
        Value::Number(number) => Some(rails_compat::numbers::number_to_s(number)),
        other => Some(other.to_string()),
    };
    Some(h::hidden_field_tag(name, value.as_deref(), h::attrs()))
}
