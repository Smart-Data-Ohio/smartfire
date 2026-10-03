//! Shared Rails datetime assignment and rendering, verified against the pinned corpora.
use bnum::types::I512;

/// The database supplies its wide UTC value and existing local-time resolver.
/// This keeps the compatibility crate independent of database and view crates.
pub trait TimeValue: Copy {
    fn jiff(self) -> jiff::Timestamp;
    fn from_wide_shifted_jiff(at: jiff::Timestamp, year_shift: I512) -> Option<Self>;
    fn local_datetime(at: jiff::civil::DateTime, zone: &jiff::tz::TimeZone) -> Option<Self>;
    fn transition_second(self) -> i64;
    fn calendar_proxy(self) -> (jiff::Timestamp, I512);
    fn since(self, duration: jiff::SignedDuration) -> Self;
}

#[derive(Debug, thiserror::Error)]
#[error("RangeError")]
pub struct DateRangeError;

/// TimeZoneConverter rescues ArgumentError (nil), but propagates RangeError.
/// Ruby Time.new converts components through NUM2INT and unsigned bit fields
/// in this order, before validating the combined date and clock.
pub fn cast<T: TimeValue>(
    value: Option<&str>,
    zone: &jiff::tz::TimeZone,
    now: T,
) -> Result<Option<T>, DateRangeError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let Some(parts) = crate::date_parse::parse(value) else {
        return Ok(None);
    };
    cast_parts(parts, zone, now)
}

/// ActiveModel::Type::DateTime's UTC database fallback. Unlike assignment via
/// Time.zone.parse, a stored string needs a year; missing month/day default to 1.
/// Numeric database values are retained by Rails and handled by the caller.
pub fn deserialize<T: TimeValue>(value: &str) -> Option<T> {
    let parts = crate::date_parse::parse_with_completion(value, true)?;
    parts.year?;
    let january = jiff::civil::Date::new(2000, 1, 1)
        .ok()?
        .at(0, 0, 0, 0)
        .to_zoned(jiff::tz::TimeZone::UTC)
        .ok()?
        .timestamp();
    let now = T::from_wide_shifted_jiff(january, I512::ZERO)?;
    cast_parts(parts, &jiff::tz::TimeZone::UTC, now)
        .ok()
        .flatten()
}

/// SQLite3 returns BLOBs as Ruby ASCII-8BIT strings. Date._parse replaces
/// non-ASCII bytes with separators, including invalid UTF-8. Preserve the
/// original byte length for its 128-byte limit before applying the shared cast.
pub fn deserialize_sqlite_blob<T: TimeValue>(value: &[u8]) -> Option<T> {
    if value.len() > 128 {
        return None;
    }
    let text: String = value
        .iter()
        .map(|byte| if byte.is_ascii() { char::from(*byte) } else { ' ' })
        .collect();
    deserialize(&text)
}

fn cast_parts<T: TimeValue>(
    parts: crate::date_parse::Parts,
    zone: &jiff::tz::TimeZone,
    now: T,
) -> Result<Option<T>, DateRangeError> {
    if parts.range_error {
        return Err(DateRangeError);
    }
    if !parts.present {
        return Ok(None);
    }
    let current = now.jiff().to_zoned(zone.clone());
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
        let naive = T::from_wide_shifted_jiff(
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
            T::from_wide_shifted_jiff(utc, shift)
        } else {
            let local = T::local_datetime(dt, zone)?;
            T::from_wide_shifted_jiff(local.jiff(), shift)
        }
    })())
}

// Pinned TZInfo's explicit transition table does not extrapolate Jiff's POSIX
// future DST rules. Shared request casts and rendering use the pinned periods.
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
fn boundary_offset(zone: &jiff::tz::TimeZone, seconds: i64, local: bool) -> Option<i64> {
    static ZONES: std::sync::LazyLock<std::collections::HashMap<String, ZoneBoundary>> =
        std::sync::LazyLock::new(|| {
            serde_json::from_str(include_str!("date_zone_boundaries.json"))
                .expect("pinned TZInfo boundaries")
        });
    let boundary = ZONES.get(zone.iana_name().unwrap_or("UTC"))?;
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
pub fn render<T: TimeValue>(at: T, zone: &jiff::tz::TimeZone, suffix: bool) -> String {
    let (proxy, _) = at.calendar_proxy();
    let offset = boundary_offset(zone, at.transition_second(), false)
        .unwrap_or_else(|| i64::from(zone.to_offset_info(proxy).offset().seconds()));
    // Shift the wide instant before choosing the safe calendar proxy. A valid
    // local +/-9999 date can lie outside Jiff's offset-reserved UTC envelope.
    let (local, shift) = at
        .since(jiff::SignedDuration::from_secs(offset))
        .calendar_proxy();
    let local = local.to_zoned(jiff::tz::TimeZone::UTC);
    let year = I512::from(local.year()) + shift;
    let year = if year.is_negative() {
        format!("-{:04}", year.unsigned_abs())
    } else {
        format!("{year:04}")
    };
    let mut result = format!("{year}{}", local.strftime("-%m-%dT%H:%M:%S"));
    if suffix {
        if zone
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
