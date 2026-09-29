//! `ActiveSupport::JSON.encode`, as Action Cable uses it for every frame and broadcast.
//!
//! Rails 8.2 defaults (`config.load_defaults 8.2`) escape `<`, `>` and `&` as `\u003c`, `\u003e`
//! and `\u0026`, but no longer escape U+2028/U+2029 (`escape_js_separators_in_json = false` since
//! 8.1). The JSON gem itself escapes the same characters serde_json does (quote, backslash and
//! control characters, lowercase hex), and neither escapes `/`.
use serde::Serialize;

/// Serializes `value` the way `ActiveSupport::JSON.encode` would.
pub fn encode<T: Serialize + ?Sized>(value: &T) -> String {
    escape_html_entities(serde_json::to_string(value).expect("JSON serialization cannot fail"))
}

/// Re-escapes already-encoded JSON. `<`, `>` and `&` can only appear inside JSON strings, so
/// replacing them anywhere in the document is safe, and doing it twice is harmless.
pub fn escape_html_entities(json: String) -> String {
    if !json.contains(['<', '>', '&']) {
        return json;
    }
    let mut escaped = String::with_capacity(json.len() + 16);
    for c in json.chars() {
        match c {
            '<' => escaped.push_str("\\u003c"),
            '>' => escaped.push_str("\\u003e"),
            '&' => escaped.push_str("\\u0026"),
            c => escaped.push(c),
        }
    }
    escaped
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn escapes_html_entities_but_not_separators_or_slashes() {
        assert_eq!(encode(&json!({ "key": "<a href=\"/x\">&</a>\u{2028}" })),
            "{\"key\":\"\\u003ca href=\\\"/x\\\"\\u003e\\u0026\\u003c/a\\u003e\u{2028}\"}");
    }

    #[test]
    fn escapes_control_characters_like_the_json_gem() {
        assert_eq!(encode("\u{1f}\n\t\u{8}\u{c}"), "\"\\u001f\\n\\t\\b\\f\"");
    }
}
