//! The two JSON encoders Rails messages use. Both emit keys in insertion order, which relies on
//! serde_json's `preserve_order` feature (enabled in the workspace manifest).
use serde_json::Value;

/// `::JSON.generate` / `JSON.dump`: plain JSON, non-ASCII left as UTF-8.
pub fn generate(value: &Value) -> String {
    serde_json::to_string(value).expect("serializing a serde_json::Value can't fail")
}

/// `ActiveSupport::JSON.encode` with `escape_html_entities_in_json` (the default): like
/// `JSON.generate`, plus `<`, `>` and `&` escaped as `\uXXXX`. Those characters can only occur
/// inside JSON strings, so escaping them in the output is safe. U+2028/U+2029 are *not* escaped:
/// `load_defaults` 8.1+ turns `escape_js_separators_in_json` off.
pub fn encode(value: &Value) -> String {
    escape(&generate(value))
}

pub fn escape(json: &str) -> String {
    let mut out = String::with_capacity(json.len());
    for c in json.chars() {
        match c {
            '<' => out.push_str("\\u003c"),
            '>' => out.push_str("\\u003e"),
            '&' => out.push_str("\\u0026"),
            c => out.push(c),
        }
    }
    out
}

pub fn parse(bytes: &[u8]) -> Option<Value> {
    serde_json::from_slice(bytes).ok()
}
