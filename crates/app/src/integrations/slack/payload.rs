//! Ruby JSON access/coercion at Slack's untyped payload boundary.
use serde_json::{Value, json};

/// Ruby's default String#downcase has no context-sensitive final sigma rule.
/// Use the pinned Ruby mappings so the Rust toolchain's Unicode version cannot
/// change the query values or group names produced by the Rails Slack mappers.
pub fn downcase(value: &str) -> String {
    if value.is_ascii() {
        return value.to_ascii_lowercase();
    }
    static MAPPINGS: std::sync::LazyLock<std::collections::HashMap<char, String>> =
        std::sync::LazyLock::new(|| {
            serde_json::from_str::<Vec<(char, String)>>(include_str!(
                "../../../data/slack-ruby-downcase.json"
            ))
            .expect("pinned Ruby downcase table")
            .into_iter()
            .collect()
        });
    let mut lowered = String::with_capacity(value.len());
    for ch in value.chars() {
        if let Some(mapping) = MAPPINGS.get(&ch) {
            lowered.push_str(mapping);
        } else {
            lowered.push(ch);
        }
    }
    lowered
}

#[derive(Debug, thiserror::Error)]
#[error("{class}: {message}")]
pub struct Error {
    pub class: &'static str,
    pub message: String,
}
impl From<Error> for campfire_db::Error {
    fn from(error: Error) -> Self {
        Self::Other(error.to_string())
    }
}
pub fn no_method(method: &str, value: &Value) -> Error {
    let receiver = match value {
        Value::Null => "nil".into(),
        Value::Bool(b) => b.to_string(),
        Value::Number(_) => "an instance of Float".into(),
        Value::String(_) => "an instance of String".into(),
        _ => "an instance of Array".into(),
    };
    Error {
        class: "NoMethodError",
        message: format!("undefined method '{method}' for {receiver}"),
    }
}
pub fn at(value: &Value, key: &str) -> Result<Value, Error> {
    match value {
        Value::Object(o) => Ok(o.get(key).cloned().unwrap_or(Value::Null)),
        Value::String(s) => Ok(if s.contains(key) {
            json!(key)
        } else {
            Value::Null
        }),
        Value::Array(_) => Err(type_error()),
        Value::Number(n) if n.is_i64() || n.is_u64() => Err(type_error()),
        _ => Err(no_method("[]", value)),
    }
}
pub fn type_error() -> Error {
    Error {
        class: "TypeError",
        message: "no implicit conversion of String into Integer".into(),
    }
}
pub fn array(value: &Value) -> Vec<Value> {
    match value {
        Value::Null => vec![],
        Value::Array(a) => a.clone(),
        Value::Object(o) => o.iter().map(|(k, v)| json!([k, v])).collect(),
        v => vec![v.clone()],
    }
}
pub fn fields(value: &Value, keys: &[&str]) -> Result<Value, Error> {
    let mut result = json!({});
    for key in keys {
        result[*key] = at(value, key)?;
    }
    Ok(result)
}
