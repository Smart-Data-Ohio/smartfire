/// Rails' default number_to_human_size: base 1024, three significant digits, stripped decimals.
pub fn human_size(size: i64) -> String {
    if size == 1 {
        return "1 Byte".into();
    }
    if size.unsigned_abs() < 1024 {
        return format!("{size} Bytes");
    }
    let exponent = ((size.unsigned_abs() as f64).log(1024.0).floor() as usize).min(6);
    let value = size as f64 / 1024_f64.powi(exponent as i32);
    let precision = 2 - value.abs().log10().floor() as i32;
    let factor = 10_f64.powi(precision);
    let rounded = (value * factor).round() / factor;
    let text = format!("{:.*}", precision.max(0) as usize, rounded);
    let text = if text.contains('.') {
        text.trim_end_matches('0').trim_end_matches('.')
    } else {
        &text
    };
    format!(
        "{text} {}",
        ["Bytes", "KB", "MB", "GB", "TB", "PB", "EB"][exponent]
    )
}
