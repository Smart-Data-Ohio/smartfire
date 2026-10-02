//! Request-only Ruby coercions; writes and validation remain with the model owners.
use bnum::types::I512;
use campfire_db::{Timestamp, slash_commands::time_parser};
use campfire_kit::Param;
use campfire_views::time::Zone;

#[derive(Debug, thiserror::Error)]
#[error("RangeError")]
pub(super) struct DateRangeError;

/// TimeZoneConverter rescues ArgumentError (nil), but propagates RangeError.
/// Ruby Time.new converts components through NUM2INT and unsigned bit fields
/// in this order, before validating the combined date and clock.
pub(super) fn datetime(
    param: Option<&Param>,
    zone: &Zone,
    now: Timestamp,
) -> Result<Option<Timestamp>, DateRangeError> {
    let Some(Param::Str(value)) = param else {
        return Ok(None);
    };
    let Some(parts) = super::date_parse::parse(value) else {
        return Ok(None);
    };
    if parts.range_error {
        return Err(DateRangeError);
    }
    if !parts.present {
        return Ok(None);
    }
    let current = now.jiff().to_zoned(zone.tz().clone());
    let year = parts.year.unwrap_or(I512::from(current.year()));
    let proxy_year = i16::try_from(year)
        .ok()
        .filter(|y| (-9998..=9998).contains(y))
        .unwrap_or_else(|| {
            2000 + i16::try_from(year.rem_euclid(I512::from(400))).expect("Gregorian remainder")
        });
    let component =
        |value: Option<I512>, default: i64, bits: u32| -> Result<Option<i64>, DateRangeError> {
            let n =
                i32::try_from(value.unwrap_or(I512::from(default))).map_err(|_| DateRangeError)?;
            Ok((n >= 0 && n < (1 << bits)).then_some(i64::from(n)))
        };
    let Some(mon) = component(parts.mon, i64::from(current.month()), 4)? else {
        return Ok(None);
    };
    let default_day = if parts.year.is_some() || parts.mon.is_some() {
        1
    } else {
        i64::from(current.day())
    };
    let Some(day) = component(parts.mday, default_day, 5)? else {
        return Ok(None);
    };
    let Some(hour) = component(parts.hour, 0, 5)? else {
        return Ok(None);
    };
    let Some(min) = component(parts.min, 0, 6)? else {
        return Ok(None);
    };
    let Some(sec) = component(parts.sec, 0, 6)? else {
        return Ok(None);
    };
    if !(1..=12).contains(&mon)
        || !(1..=31).contains(&day)
        || !(0..=24).contains(&hour)
        || !(0..=59).contains(&min)
        || !(0..=60).contains(&sec)
        || (hour == 24 && (min != 0 || sec != 0))
    {
        return Ok(None);
    }
    Ok((|| {
        // Time.new normalizes month-end days, hour 24, and leap seconds.
        let date = jiff::civil::Date::new(proxy_year, mon as i8, 1)
            .ok()?
            .checked_add(jiff::Span::new().days(day - 1))
            .ok()?;
        let dt = date
            .at(0, 0, 0, 0)
            .checked_add(
                jiff::Span::new()
                    .hours(hour)
                    .minutes(min)
                    .seconds(sec)
                    .nanoseconds(parts.nanosecond),
            )
            .ok()?;
        let shift = year - I512::from(proxy_year);
        let naive = Timestamp::from_wide_shifted_jiff(
            dt.to_zoned(jiff::tz::TimeZone::UTC).ok()?.timestamp(),
            shift,
        )?;
        let offset = parts.offset_nanoseconds.or_else(|| {
            boundary_offset(zone, naive.transition_second(), true).map(|s| s * 1_000_000_000)
        });
        if let Some(offset) = offset {
            if offset.abs() >= 86_400_000_000_000 {
                return None;
            }
            let utc = dt
                .to_zoned(jiff::tz::TimeZone::UTC)
                .ok()?
                .timestamp()
                .checked_sub(jiff::SignedDuration::from_nanos(offset))
                .ok()?;
            Timestamp::from_wide_shifted_jiff(utc, shift)
        } else {
            let local = time_parser::local_datetime(dt, zone.tz())?;
            Timestamp::from_wide_shifted_jiff(local.jiff(), shift)
        }
    })())
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
    if depth >= 128 {
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
            let value = text.parse::<f64>().map_err(serde::de::Error::custom)?;
            if value.is_infinite() {
                if value.is_sign_negative() {
                    "-Infinity".into()
                } else {
                    "Infinity".into()
                }
            } else {
                ruby_number(
                    &serde_json::Number::from_f64(value).expect("JSON finite or infinite float"),
                )
            }
        }
    })
}

fn inspect(input: &Param, parameters: bool) -> String {
    match input {
        Param::Null => "nil".into(),
        Param::Bool(value) => value.to_string(),
        Param::Number(value) => ruby_number(value),
        Param::Str(value) => ruby_string(value),
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

/// rb_float_to_s (Ruby 3.4 numeric.c): shortest digits, decimal notation when
/// digits extend past a positive point, or -4 < point <= DBL_DIG; else an exponent.
fn ruby_number(value: &serde_json::Number) -> String {
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

// Pinned TZInfo's explicit transition table does not extrapolate Jiff's POSIX
// future DST rules. Keep this UI cast seam local; WS8 owns general time parsing.
#[derive(serde::Deserialize)]
struct ZoneBoundary {
    offset: Option<i64>,
    first_at: Option<i64>,
    first_offset: Option<i64>,
    first_next_offset: Option<i64>,
    last_at: Option<i64>,
    last_offset: Option<i64>,
    last_previous_offset: Option<i64>,
}
fn boundary_offset(zone: &Zone, seconds: i64, local: bool) -> Option<i64> {
    static ZONES: std::sync::LazyLock<std::collections::HashMap<String, ZoneBoundary>> =
        std::sync::LazyLock::new(|| {
            serde_json::from_str(include_str!("date_zone_boundaries.json"))
                .expect("pinned TZInfo boundaries")
        });
    let boundary = ZONES.get(zone.tz().iana_name().unwrap_or("UTC"))?;
    if let Some(offset) = boundary.offset {
        return Some(offset);
    }
    let delta = |offset: Option<i64>| if local { offset.unwrap_or(0) } else { 0 };
    if seconds
        < boundary.first_at?
            + delta(
                boundary
                    .first_offset
                    .zip(boundary.first_next_offset)
                    .map(|(a, b)| a.min(b)),
            )
    {
        return boundary.first_offset;
    }
    if seconds
        >= boundary.last_at?
            + delta(
                boundary
                    .last_offset
                    .zip(boundary.last_previous_offset)
                    .map(|(a, b)| a.max(b)),
            )
    {
        return boundary.last_offset;
    }
    None
}
/// Extended-year rendering uses the same pinned zone periods as the request cast.
pub(crate) fn extended_datetime(at: Timestamp, zone: &Zone, suffix: bool) -> String {
    let (proxy, shift) = at.calendar_proxy();
    let offset = boundary_offset(zone, at.transition_second(), false)
        .unwrap_or_else(|| i64::from(zone.tz().to_offset_info(proxy).offset().seconds()));
    let local = proxy
        .checked_add(jiff::SignedDuration::from_secs(offset))
        .expect("bounded zone offset")
        .to_zoned(jiff::tz::TimeZone::UTC);
    let year = I512::from(local.year()) + shift;
    let year = if year.is_negative() {
        format!("-{:04}", year.unsigned_abs())
    } else {
        format!("{year:04}")
    };
    let mut result = format!("{year}{}", local.strftime("-%m-%dT%H:%M:%S"));
    if suffix {
        if zone
            .tz()
            .iana_name()
            .is_none_or(|name| name == "Etc/UTC" || name == "UTC")
        {
            result.push('Z');
        } else {
            result.push_str(&format!(
                "{}{:02}:{:02}",
                if offset < 0 { '-' } else { '+' },
                offset.abs() / 3600,
                offset.abs() % 3600 / 60
            ));
        }
    }
    result
}
