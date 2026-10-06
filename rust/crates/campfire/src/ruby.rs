//! Ruby core conversions the app's parameters and settings rely on.

/// `String#to_i`: optional leading whitespace and sign, then digits (underscores between them).
pub fn ruby_to_i(value: &str) -> i64 {
    let value = value.trim_start_matches([' ', '\t', '\n', '\u{b}', '\u{c}', '\r']);
    let (negative, rest) = match value.as_bytes().first() {
        Some(b'-') => (true, &value[1..]),
        Some(b'+') => (false, &value[1..]),
        _ => (false, value),
    };
    let rest = rest
        .strip_prefix("0d")
        .or_else(|| rest.strip_prefix("0D"))
        .unwrap_or(rest);
    let mut number: i64 = 0;
    let mut previous_digit = false;
    for c in rest.chars() {
        match c {
            '0'..='9' => {
                number = number
                    .saturating_mul(10)
                    .saturating_add(i64::from(c as u8 - b'0'));
                previous_digit = true;
            }
            '_' if previous_digit => previous_digit = false,
            _ => break,
        }
    }
    if negative { -number } else { number }
}
