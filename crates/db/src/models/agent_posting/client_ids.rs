//! The raw bot route passes unpermitted client ids into AR's duplicate lookup and String
//! assignment. Their coercions differ: false is blank for lookup but stores "f"; arrays
//! bind their elements as an IN query while String stores their Ruby inspect representation.
use serde_json::Value;

#[derive(Debug, Clone, Default)]
pub enum Lookup {
    #[default]
    Attribute,
    Values(Vec<String>),
    InvalidParameters,
}
#[derive(Debug, Clone, Default)]
pub struct ClientId {
    pub stored: Option<String>,
    pub lookup: Lookup,
}
impl ClientId {
    pub fn from_json(input: Option<&Value>) -> Self {
        let Some(value) = input.filter(|v| !v.is_null()) else {
            return Self::default();
        };
        let stored = Some(ar_string(value));
        let lookup = match value {
            Value::Bool(false) => Lookup::Values(vec![]),
            Value::Array(items) => {
                if contains_object(value) {
                    Lookup::InvalidParameters
                } else {
                    Lookup::Values(flattened(items))
                }
            }
            Value::Object(map) if !map.is_empty() => Lookup::InvalidParameters,
            Value::Object(_) => Lookup::Values(vec![]),
            Value::String(s) if campfire_richtext::ruby::is_blank(s) => Lookup::Values(vec![]),
            _ => Lookup::Attribute,
        };
        Self { stored, lookup }
    }
}
fn contains_object(value: &Value) -> bool {
    match value {
        Value::Object(_) => true,
        Value::Array(items) => items.iter().any(contains_object),
        _ => false,
    }
}
fn flattened(items: &[Value]) -> Vec<String> {
    let mut values = vec![];
    for value in items {
        match value {
            Value::Array(items) => values.extend(flattened(items)),
            Value::Null => {}
            _ => values.push(ar_string(value)),
        }
    }
    values
}
fn ar_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Bool(true) => "t".into(),
        Value::Bool(false) => "f".into(),
        Value::Number(n) => ruby_number(n),
        Value::Array(items) => format!(
            "[{}]",
            items.iter().map(inspect).collect::<Vec<_>>().join(", ")
        ),
        Value::Object(_) => "{}".into(),
        _ => v.to_string(),
    }
}
fn inspect(v: &Value) -> String {
    match v {
        Value::Null => "nil".into(),
        Value::String(s) => inspect_string(s),
        Value::Number(n) => ruby_number(n),
        Value::Array(items) => format!(
            "[{}]",
            items.iter().map(inspect).collect::<Vec<_>>().join(", ")
        ),
        _ => v.to_string(),
    }
}
fn inspect_string(s: &str) -> String {
    let mut out = String::from("\"");
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{7}' => out.push_str("\\a"),
            '\u{8}' => out.push_str("\\b"),
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\u{b}' => out.push_str("\\v"),
            '\u{c}' => out.push_str("\\f"),
            '\r' => out.push_str("\\r"),
            '\u{1b}' => out.push_str("\\e"),
            '#' if chars.peek().is_some_and(|c| ['{', '@', '$'].contains(c)) => out.push_str("\\#"),
            c if c.is_control() || ['\u{2028}', '\u{2029}'].contains(&c) => {
                out.push_str(&format!("\\u{:04X}", c as u32))
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
/// Float#to_s uses the same shortest digits with Ruby's fixed/scientific thresholds and
/// two-place exponent. Integer JSON parameters stay integer strings.
fn ruby_number(n: &serde_json::Number) -> String {
    if n.is_i64() || n.is_u64() {
        return n.to_string();
    }
    let raw = n.to_string();
    let value = n.as_f64().expect("finite JSON number");
    if value == 0.0 {
        return if value.is_sign_negative() {
            "-0.0"
        } else {
            "0.0"
        }
        .into();
    }
    let (sign, text) = if let Some(text) = raw.strip_prefix('-') {
        ("-", text)
    } else {
        ("", raw.as_str())
    };
    let (mantissa, power) = text.split_once(['e', 'E']).map_or((text, 0), |(m, e)| {
        (m, e.parse::<i32>().expect("JSON exponent"))
    });
    let before = mantissa.find('.').unwrap_or(mantissa.len()) as i32;
    let digits = mantissa.replace('.', "");
    let leading = digits.len() - digits.trim_start_matches('0').len();
    let power = power + before - 1 - leading as i32;
    let digits = digits.trim_start_matches('0').trim_end_matches('0');
    if !(-4..15).contains(&power) {
        let tail = if digits.len() > 1 { &digits[1..] } else { "0" };
        return format!("{sign}{}.{}e{power:+03}", &digits[..1], tail);
    }
    let point = power + 1;
    if point <= 0 {
        format!("{sign}0.{}{digits}", "0".repeat((-point) as usize))
    } else if point as usize >= digits.len() {
        format!(
            "{sign}{digits}{}.0",
            "0".repeat(point as usize - digits.len())
        )
    } else {
        format!(
            "{sign}{}.{}",
            &digits[..point as usize],
            &digits[point as usize..]
        )
    }
}
