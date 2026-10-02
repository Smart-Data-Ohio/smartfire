//! Request-only Ruby coercions; writes and validation remain with the model owners.
use campfire_db::{Timestamp, slash_commands::time_parser};
use campfire_kit::Param;
use campfire_views::time::Zone;

pub(super) fn datetime(param: Option<&Param>, zone: &Zone, now: Timestamp) -> Option<Timestamp> {
    let Param::Str(value) = param? else {
        return None;
    };
    let parts = super::date_parse::parse(value)?;
    if !parts.present {
        return None;
    }
    let current = now.jiff().to_zoned(zone.tz().clone());
    let year = parts.year.unwrap_or(i64::from(current.year()));
    let proxy_year = if (-9998..=9998).contains(&year) {
        year
    } else {
        2000 + year.rem_euclid(400)
    };
    let mon = i8::try_from(parts.mon.unwrap_or(i64::from(current.month()))).ok()?;
    let day = parts
        .mday
        .unwrap_or(if parts.year.is_some() || parts.mon.is_some() {
            1
        } else {
            i64::from(current.day())
        });
    let hour = parts.hour.unwrap_or(0);
    let min = parts.min.unwrap_or(0);
    let sec = parts.sec.unwrap_or(0);
    if !(1..=31).contains(&day)
        || !(0..=24).contains(&hour)
        || !(0..=59).contains(&min)
        || !(0..=60).contains(&sec)
        || (hour == 24 && (min != 0 || sec != 0))
    {
        return None;
    }
    // Time.new normalizes month-end days, hour 24, and leap seconds.
    let date = jiff::civil::Date::new(i16::try_from(proxy_year).ok()?, mon, 1)
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
    let naive = Timestamp::from_shifted_jiff(
        dt.to_zoned(jiff::tz::TimeZone::UTC).ok()?.timestamp(),
        year.checked_sub(proxy_year)?,
    )?;
    let offset = parts
        .offset_nanoseconds
        .or_else(|| boundary_offset(zone, naive.as_second(), true).map(|s| s * 1_000_000_000));
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
        Timestamp::from_shifted_jiff(utc, year.checked_sub(proxy_year)?)
    } else {
        let local = time_parser::local_datetime(dt, zone.tz())?;
        Timestamp::from_shifted_jiff(local.jiff(), year.checked_sub(proxy_year)?)
    }
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
    if seconds < boundary.first_at? + delta(boundary.first_offset.zip(boundary.first_next_offset).map(|(a,b)|a.min(b))) {
        return boundary.first_offset;
    }
    if seconds >= boundary.last_at? + delta(boundary.last_offset.zip(boundary.last_previous_offset).map(|(a,b)|a.max(b))) {
        return boundary.last_offset;
    }
    None
}
/// Extended-year rendering uses the same pinned zone periods as the request cast.
pub(crate) fn extended_datetime(at: Timestamp, zone: &Zone, suffix: bool) -> String {
    let (proxy, shift) = at.calendar_proxy();
    let offset = boundary_offset(zone, at.as_second(), false)
        .unwrap_or_else(|| i64::from(zone.tz().to_offset_info(proxy).offset().seconds()));
    let local = proxy
        .checked_add(jiff::SignedDuration::from_secs(offset))
        .expect("bounded zone offset")
        .to_zoned(jiff::tz::TimeZone::UTC);
    let year = i64::from(local.year()) + shift;
    let year = if year < 0 {
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
