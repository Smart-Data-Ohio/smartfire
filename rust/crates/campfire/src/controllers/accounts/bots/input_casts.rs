//! Request-only Ruby coercions; writes and validation remain with the model owners.
use campfire_db::Timestamp;
use campfire_kit::Param;
use campfire_views::time::Zone;

pub(super) fn datetime(
    param: Option<&Param>,
    zone: &Zone,
    now: Timestamp,
) -> Result<Option<Timestamp>, rails_compat::datetime::DateRangeError> {
    let value = match param {
        Some(Param::Str(value)) => Some(value.as_str()),
        _ => None,
    };
    rails_compat::datetime::cast(value, zone.tz(), now)
}

pub(crate) fn extended_datetime(at: Timestamp, zone: &Zone, suffix: bool) -> String {
    rails_compat::datetime::render(at, zone.tz(), suffix)

}

/// GithubConnectionsController calls `params[:access_token].to_s.strip` before
/// blank?; unlike permitted model attributes, arrays and hashes reach this cast.
pub(super) fn token_string(input: &Param) -> String {
    match input {
        Param::Null => String::new(),
        Param::Str(value) => value.clone(),
        Param::Array(_) => inspect(input, true),
        _ => inspect(input, false),
    }
}

/// RawValue checks JSON syntax without rounding an Integer to f64 or rejecting
/// an overflowing exponent. Recursion and body bytes are bounded by the kit.
pub(super) fn json_token_string(raw: &serde_json::value::RawValue) -> serde_json::Result<String> {
    match raw.get().as_bytes().first() {
        Some(b'n') => Ok(String::new()),
        Some(b'"') => serde_json::from_str(raw.get()),
        Some(b'[') => json_inspect(raw, true, 0),
        _ => json_inspect(raw, false, 0),
    }
}
fn json_inspect(
    raw: &serde_json::value::RawValue,
    parameters: bool,
    depth: usize,
) -> serde_json::Result<String> {
    use serde_json::value::RawValue;
    let text = raw.get();
    if depth >= 100 {
        return Err(serde::de::Error::custom(
            "JSON nesting exceeds parameter limit",
        ));
    }
    Ok(match text.as_bytes().first() {
        Some(b'n') => "nil".into(),
        Some(b't' | b'f') => text.into(),
        Some(b'"') => ruby_string(&serde_json::from_str::<String>(text)?),
        Some(b'[') => {
            let values: Vec<Box<RawValue>> = serde_json::from_str(text)?;
            let body = values
                .iter()
                .filter(|v| v.get() != "null")
                .map(|v| json_inspect(v, parameters, depth + 1))
                .collect::<serde_json::Result<Vec<_>>>()?
                .join(", ");
            format!("[{body}]")
        }
        Some(b'{') => {
            let values: indexmap::IndexMap<String, Box<RawValue>> = serde_json::from_str(text)?;
            let body = values
                .iter()
                .map(|(k, v)| {
                    Ok(format!(
                        "{} => {}",
                        ruby_string(k),
                        json_inspect(v, false, depth + 1)?
                    ))
                })
                .collect::<serde_json::Result<Vec<_>>>()?
                .join(", ");
            let body = format!("{{{body}}}");
            if parameters {
                format!("#<ActionController::Parameters {body} permitted: false>")
            } else {
                body
            }
        }
        _ if !text.contains(['.', 'e', 'E']) => {
            // JSON's sole signed-zero Integer spelling is -0; Ruby normalizes it.
            if text == "-0" {
                "0".into()
            } else {
                text.into()
            }
        }
        _ => {
            let value = json_float(text)?;
            if value.is_infinite() {
                if value.is_sign_negative() {
                    "-Infinity".into()
                } else {
                    "Infinity".into()
                }
            } else {
                rails_compat::numbers::number_to_s(
                    &serde_json::Number::from_f64(value).expect("JSON finite or infinite float"),
                )
            }
        }
    })
}

/// json-2.21.2's json_parse_number/json_decode_float checks the adjusted
/// exponent before the mantissa, including zero and signed zero. Exponent
/// digit counts and i64 saturation also precede decimal-point adjustment.
fn json_float(text: &str) -> serde_json::Result<f64> {
    let (mantissa, exponent) = text.split_once(['e', 'E']).unwrap_or((text, "0"));
    let negative = exponent.starts_with('-');
    let digits = exponent.trim_start_matches(['-', '+']);
    let magnitude = digits.parse::<u64>().ok();
    let saturated = digits.len() >= 20 || magnitude.is_none_or(|n| n > i64::MAX as u64);
    let exponent = if saturated {
        if negative { i64::MIN } else { i64::MAX }
    } else {
        let n = magnitude.expect("bounded exponent") as i64;
        if negative { -n } else { n }
    };
    let fractional_digits = mantissa
        .split_once('.')
        .map_or(0, |(_, fraction)| fraction.len() as i64);
    // In the pinned compiled json-2.21.2 parser, the saturated branch keeps its
    // sign at the i32 cutoffs. The in-range branch's signed decimal adjustment
    // wraps instead. The Rails boundary vectors cover both execution paths.
    let exponent = if saturated {
        exponent
    } else {
        exponent.wrapping_sub(fractional_digits)
    };
    let sign = if text.starts_with('-') { -1.0 } else { 1.0 };
    if exponent > i64::from(i32::MAX) {
        Ok(sign * f64::INFINITY)
    } else if exponent < i64::from(i32::MIN) {
        Ok(sign * 0.0)
    } else {
        text.parse::<f64>().map_err(serde::de::Error::custom)
    }
}

fn inspect(input: &Param, parameters: bool) -> String {
    match input {
        Param::Null => "nil".into(),
        Param::Bool(value) => value.to_string(),
        Param::Number(value) => rails_compat::numbers::number_to_s(value),
        Param::Str(value) => ruby_string(value),
        Param::BigInteger(value) => value.clone(),
        Param::File(file) => format!(
            "#<ActionDispatch::Http::UploadedFile:{:p}>",
            std::sync::Arc::as_ptr(file)
        ),
        Param::Array(values) => format!(
            "[{}]",
            values
                .iter()
                .map(|value| inspect(value, parameters))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Param::Hash(values) => {
            // Hash#to_s invokes to_unsafe_h first; hashes inside a parameter
            // array remain ActionController::Parameters during Array#inspect.
            let body = format!(
                "{{{}}}",
                values
                    .iter()
                    .map(|(key, value)| format!(
                        "{} => {}",
                        ruby_string(key),
                        inspect(value, false)
                    ))
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            if parameters {
                format!("#<ActionController::Parameters {body} permitted: false>")
            } else {
                body
            }
        }
    }
}


fn ruby_string(value: &str) -> String {
    let mut result = String::from("\"");
    let mut chars = value.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            '\u{7}' => result.push_str("\\a"),
            '\u{8}' => result.push_str("\\b"),
            '\u{b}' => result.push_str("\\v"),
            '\u{c}' => result.push_str("\\f"),
            '\u{1b}' => result.push_str("\\e"),
            '#' if chars.peek().is_some_and(|c| matches!(c, '{' | '$' | '@')) => {
                result.push_str("\\#")
            }
            ch if ch.is_control() || matches!(ch, '\u{2028}' | '\u{2029}') => {
                result.push_str(&format!("\\u{:04X}", u32::from(ch)))
            }
            ch => result.push(ch),
        }
    }
    result.push('"');
    result
}
