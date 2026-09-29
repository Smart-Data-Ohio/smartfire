//! The bits of Ruby and Active Support semantics the token checks lean on.
use serde_json::Value;

/// `String#strip`: leading and trailing ASCII whitespace and NUL.
pub(crate) fn strip(s: &str) -> &str {
    s.trim_matches(|c: char| matches!(c, ' ' | '\t' | '\n' | '\u{b}' | '\u{c}' | '\r' | '\0'))
}

/// `String#blank?`: empty or only whitespace.
pub(crate) fn str_blank(s: &str) -> bool {
    s.chars().all(char::is_whitespace)
}

/// `Object#blank?` for a JSON value: nil, false, a blank string, an empty array or hash.
pub(crate) fn blank(value: Option<&Value>) -> bool {
    match value {
        None | Some(Value::Null | Value::Bool(false)) => true,
        Some(Value::String(s)) => str_blank(s),
        Some(Value::Array(a)) => a.is_empty(),
        Some(Value::Object(o)) => o.is_empty(),
        Some(Value::Bool(true) | Value::Number(_)) => false,
    }
}

/// Ruby truthiness: everything but nil and false.
pub(crate) fn truthy(value: Option<&Value>) -> bool {
    !matches!(value, None | Some(Value::Null | Value::Bool(false)))
}

/// `#to_i` on a JSON value, or `None` where Ruby raises `NoMethodError` (booleans, arrays,
/// hashes). Integers past `i128` saturate, which keeps every comparison with a timestamp intact.
pub(crate) fn to_i(value: &Value) -> Option<i128> {
    match value {
        Value::Null => Some(0),
        Value::Number(n) => Some(if let Some(i) = n.as_i64() {
            i as i128
        } else if let Some(u) = n.as_u64() {
            u as i128
        } else {
            n.as_f64().map_or(0, |f| f.trunc() as i128)
        }),
        Value::String(s) => Some(string_to_i(s)),
        Value::Bool(_) | Value::Array(_) | Value::Object(_) => None,
    }
}

/// `String#to_i`: optional leading whitespace and sign, then digits (single underscores between
/// digits allowed) up to the first other character; `0` when there are none.
fn string_to_i(s: &str) -> i128 {
    let s = s.trim_start_matches([' ', '\t', '\n', '\u{b}', '\u{c}', '\r']);
    let (negative, digits) = match s.as_bytes().first() {
        Some(b'-') => (true, &s[1..]),
        Some(b'+') => (false, &s[1..]),
        _ => (false, s),
    };
    let bytes = digits.as_bytes();
    let mut value: i128 = 0;
    for (i, &byte) in bytes.iter().enumerate() {
        match byte {
            b'0'..=b'9' => value = value.saturating_mul(10).saturating_add((byte - b'0') as i128),
            b'_' if i > 0 && bytes[i - 1].is_ascii_digit() && bytes.get(i + 1).is_some_and(u8::is_ascii_digit) => {}
            _ => break,
        }
    }
    if negative { -value } else { value }
}

/// `value.is_a?(Numeric) && value > threshold`, or `None` when the value isn't a number.
pub(crate) fn numeric_gt(value: Option<&Value>, threshold: i64) -> Option<bool> {
    let Some(Value::Number(n)) = value else { return None };
    Some(if let Some(i) = n.as_i64() {
        i > threshold
    } else if let Some(u) = n.as_u64() {
        u as i128 > threshold as i128
    } else {
        n.as_f64()? > threshold as f64
    })
}

/// `String#split(separator)` with a string separator: trailing empty fields are dropped.
pub(crate) fn split(s: &str, separator: char) -> Vec<&str> {
    let mut parts: Vec<&str> = s.split(separator).collect();
    while parts.last().is_some_and(|part| part.is_empty()) {
        parts.pop();
    }
    parts
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn to_i_follows_ruby() {
        for (value, expected) in [
            (json!(12), Some(12)),
            (json!(12.9), Some(12)),
            (json!(-12.9), Some(-12)),
            (json!(null), Some(0)),
            (json!(" 1_000abc"), Some(1000)),
            (json!("1__0"), Some(1)),
            (json!("_1"), Some(0)),
            (json!("-5"), Some(-5)),
            (json!("abc"), Some(0)),
            (json!(true), None),
            (json!([1]), None),
        ] {
            assert_eq!(to_i(&value), expected, "{value}");
        }
    }

    /// Probed in the reference image (Ruby 3.4.10, Active Support's `blank?`).
    #[test]
    fn strip_and_blank_follow_ruby() {
        assert_eq!(strip("\0 \t a\0b \n\u{b}\u{c}\r\0"), "a\0b");
        assert_eq!(strip("\0\0a"), "a");
        assert_eq!(strip("\u{a0}a\u{a0}"), "\u{a0}a\u{a0}");
        assert_eq!(strip("\u{3000}a"), "\u{3000}a");
        for (s, expected) in [("\u{a0}", true), ("\u{3000}", true), ("\0", false), (" \u{2028}", true), ("\u{85}", true), ("\u{180e}", false), ("\u{200b}", false), ("\u{feff}", false), ("\u{1c}", false)] {
            assert_eq!(str_blank(s), expected, "{s:?}");
        }
    }

    #[test]
    fn split_drops_trailing_empties() {
        assert_eq!(split("a@b@", '@'), ["a", "b"]);
        assert_eq!(split("@", '@'), Vec::<&str>::new());
        assert_eq!(split("a..", '.'), ["a"]);
        assert_eq!(split(".a", '.'), ["", "a"]);
    }
}
