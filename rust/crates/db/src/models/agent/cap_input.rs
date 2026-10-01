//! AR integer casting and numericality validation retain the submitted value.
use regex::Regex;
use serde_json::{Value, json};
use std::sync::LazyLock;
static INTEGER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\A[+-]?[0-9]+\z").unwrap());
static NUMBER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\A[+-]?(?:(?:[0-9](?:_?[0-9])*)(?:\.(?:[0-9](?:_?[0-9])*)?)?|\.[0-9](?:_?[0-9])*)(?:[eE][+-]?[0-9](?:_?[0-9])*)?\z").unwrap()
});
#[derive(Debug, Clone)]
pub struct BudgetCapInput {
    pub before_type_cast: Value,
    pub value: Value,
    pub errors: Vec<&'static str>,
}
fn strip(s: &str) -> &str {
    s.trim_matches([' ', '\t', '\n', '\r', '\u{b}', '\u{c}', '\0'])
}
fn integer_value(s: &str) -> Value {
    let s = strip(s);
    let (sign, digits) = if let Some(s) = s.strip_prefix('-') {
        ("-", s)
    } else {
        ("", s.strip_prefix('+').unwrap_or(s))
    };
    let mut result = String::from(sign);
    let mut previous_digit = false;
    let mut chars = digits.chars().peekable();
    while let Some(c) = chars.next() {
        if c.is_ascii_digit() {
            result.push(c);
            previous_digit = true;
        } else if c == '_' && previous_digit && chars.peek().is_some_and(char::is_ascii_digit) {
            previous_digit = false;
        } else {
            break;
        }
    }
    if result.is_empty() || result == "-" {
        return json!(0);
    }
    if let Ok(n) = result.parse::<i64>() {
        json!(n)
    } else if let Ok(n) = result.parse::<u64>() {
        json!(n)
    } else {
        Value::String(result)
    }
}
impl BudgetCapInput {
    pub fn new(input: Value) -> Self {
        let value = match &input {
            Value::Null => Value::Null,
            Value::String(s) if campfire_richtext::ruby::is_blank(s) => Value::Null,
            Value::String(s) => integer_value(s),
            Value::Bool(b) => json!(i64::from(*b)),
            Value::Number(n) if n.is_f64() => {
                integer_value(&format!("{:.0}", n.as_f64().unwrap().trunc()))
            }
            value => value.clone(),
        };
        let errors = if value.is_null() {
            vec![]
        } else {
            let (number, integer) = match &input {
                Value::Bool(false) => (true, true), // Ruby's `raw_value || value` uses cast zero.
                Value::Number(n) => (true, !n.is_f64()),
                Value::String(s) => {
                    let cleaned = strip(s);
                    let number = NUMBER.is_match(cleaned)
                        && cleaned
                            .replace('_', "")
                            .parse::<f64>()
                            .is_ok_and(f64::is_finite);
                    (number, INTEGER.is_match(s))
                }
                _ => (false, false),
            };
            if !number {
                vec!["is not a number"]
            } else if !integer {
                vec!["must be an integer"]
            } else if value.as_i64().is_some_and(|n| n <= 0) {
                vec!["must be greater than 0"]
            } else {
                vec![]
            }
        };
        Self {
            before_type_cast: input,
            value,
            errors,
        }
    }
}
