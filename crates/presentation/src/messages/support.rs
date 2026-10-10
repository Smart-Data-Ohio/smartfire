// Small Ruby/Rails behaviors the message and room views depend on: time formats and
// `Float#to_s`.

use jiff::Timestamp;

/// `time.iso8601` for a UTC `ActiveSupport::TimeWithZone`: seconds precision, `Z` suffix.
pub fn iso8601(time: Timestamp) -> String {
    time.strftime("%Y-%m-%dT%H:%M:%SZ").to_string()
}

/// `time.to_fs(:epoch)`, defined in `reference/config/initializers/time_formats.rb` as
/// `(time.to_f * 1000).to_i`. The float round trip is deliberate: it truncates some
/// millisecond values down by one, and the client compares these numbers.
pub fn epoch_ms(time: Timestamp) -> i64 {
    // `Time#to_f` is the nearest double to the exact rational, which parsing the decimal
    // representation gives us.
    let seconds = time.as_second();
    let nanos = time.subsec_nanosecond();
    let decimal = if seconds < 0 && nanos != 0 {
        let whole = seconds + 1;
        let frac = 1_000_000_000 - nanos;
        format!("{}{}.{:09}", if whole == 0 { "-" } else { "" }, whole, frac)
    } else {
        format!("{seconds}.{nanos:09}")
    };
    let to_f: f64 = decimal.parse().unwrap_or(seconds as f64);
    (to_f * 1000.0) as i64
}

/// `time.as_json` with Active Support's default precision: `2026-09-26T12:26:46.848Z`.
pub fn json_time(time: Timestamp) -> String {
    let millis = time.subsec_nanosecond() / 1_000_000;
    format!("{}.{millis:03}Z", time.strftime("%Y-%m-%dT%H:%M:%S"))
}

/// A number as Ruby prints it: integers bare, floats always with a fractional part
/// (`600.0`) and in exponent form outside `1e-4..1e16` (`1.0e+16`).
#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(untagged)]
pub enum RubyNumber {
    Int(i64),
    Float(f64),
}

impl RubyNumber {
    pub fn to_f(self) -> f64 {
        match self {
            RubyNumber::Int(value) => value as f64,
            RubyNumber::Float(value) => value,
        }
    }

    /// `number / 2`: integer division for integers.
    pub fn half(self) -> RubyNumber {
        match self {
            RubyNumber::Int(value) => RubyNumber::Int(value.div_euclid(2)),
            RubyNumber::Float(value) => RubyNumber::Float(value / 2.0),
        }
    }
}

impl std::fmt::Display for RubyNumber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RubyNumber::Int(value) => write!(f, "{value}"),
            RubyNumber::Float(value) => f.write_str(&ruby_float(*value)),
        }
    }
}

/// `Float#to_s`.
pub fn ruby_float(value: f64) -> String {
    if value.is_nan() {
        return "NaN".into();
    }
    if value.is_infinite() {
        return if value > 0.0 { "Infinity".into() } else { "-Infinity".into() };
    }
    let magnitude = value.abs();
    if magnitude != 0.0 && !(1e-4..1e16).contains(&magnitude) {
        // Rust: "1.5e16", Ruby: "1.5e+16"; Rust: "1e16", Ruby: "1.0e+16".
        let formatted = format!("{value:e}");
        let (mantissa, exponent) = formatted.split_once('e').unwrap();
        let mantissa = if mantissa.contains('.') { mantissa.to_string() } else { format!("{mantissa}.0") };
        let exponent: i32 = exponent.parse().unwrap();
        let sign = if exponent < 0 { '-' } else { '+' };
        return format!("{mantissa}e{sign}{:02}", exponent.abs());
    }
    let formatted = format!("{value}");
    if formatted.contains('.') { formatted } else { format!("{formatted}.0") }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_truncates_through_a_float_like_ruby() {
        let time: Timestamp = "2026-09-26T12:23:46.483521Z".parse().unwrap();
        assert_eq!(epoch_ms(time), 1790425426483);
        let time: Timestamp = "2026-09-26T11:23:46Z".parse().unwrap();
        assert_eq!(epoch_ms(time), 1790421826000);
    }

    #[test]
    fn formats_floats_like_ruby() {
        assert_eq!(ruby_float(600.0), "600.0");
        assert_eq!(ruby_float(1.0), "1.0");
        assert_eq!(ruby_float(16.0 / 9.0), "1.7777777777777777");
        assert_eq!(ruby_float(1e16), "1.0e+16");
        assert_eq!(ruby_float(0.00001), "1.0e-05");
        assert_eq!(RubyNumber::Int(641).half().to_string(), "320");
    }

    #[test]
    fn formats_json_times_with_milliseconds() {
        let time: Timestamp = "2026-09-26T12:26:46.848999Z".parse().unwrap();
        assert_eq!(json_time(time), "2026-09-26T12:26:46.848Z");
    }
}
