//! Ruby numeric display shared by request coercions and form rendering.
/// rb_float_to_s (Ruby 3.4 numeric.c): shortest digits, decimal notation when
/// digits extend past a positive point, or -4 < point <= DBL_DIG; else an exponent.
pub fn number_to_s(value: &serde_json::Number) -> String {
    if !value.is_f64() {
        return value.to_string();
    }
    let text = value.to_string();
    let (sign, text) = text
        .strip_prefix('-')
        .map_or(("", text.as_str()), |s| ("-", s));
    let (mantissa, exponent) = text
        .split_once(['e', 'E'])
        .map_or((text, 0), |(m, e)| (m, e.parse::<i32>().unwrap()));
    let point = mantissa.find('.').unwrap_or(mantissa.len()) as i32 + exponent;
    let digits: String = mantissa.chars().filter(|c| *c != '.').collect();
    let leading = digits.bytes().take_while(|b| *b == b'0').count();
    let digits = digits[leading..].trim_end_matches('0');
    if digits.is_empty() {
        return format!("{sign}0.0");
    }
    let point = point - leading as i32;
    if point <= -4 || (point > 15 && point as usize >= digits.len()) {
        let rest = if digits.len() == 1 { "0" } else { &digits[1..] };
        let exponent = point - 1;
        format!(
            "{sign}{}.{rest}e{}{:02}",
            &digits[..1],
            if exponent < 0 { '-' } else { '+' },
            exponent.abs()
        )
    } else if point <= 0 {
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
