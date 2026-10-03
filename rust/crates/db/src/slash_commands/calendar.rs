//! Shared Time.zone.parse assembly; Date._parse grammar is WS11's rails_compat port.
use crate::{Error, Result, Timestamp};
use bnum::types::I512;
use jiff::tz::TimeZone;
fn argument(message: &str) -> Error {
    Error::Other(format!("ArgumentError: {message}"))
}
fn range(message: String) -> Error {
    Error::Other(format!("RangeError: {message}"))
}
pub(super) fn parse(text: &str, zone: &TimeZone, now: Timestamp) -> Result<Option<Timestamp>> {
    if text.len() > 128 {
        return Err(argument(&format!(
            "string length ({}) exceeds the limit 128",
            text.len()
        )));
    }
    let Some(parts) = rails_compat::date_parse::parse(text) else {
        return Ok(None);
    };
    if parts.range_error {
        return Err(range("bignum too big to convert into `long long'".into()));
    }
    if !parts.present {
        return Ok(None);
    };
    let current = now.jiff().to_zoned(zone.clone());
    let year = parts.year.unwrap_or(I512::from(current.year()));
    let proxy_year = i16::try_from(year)
        .ok()
        .filter(|y| (-9998..=9998).contains(y))
        .unwrap_or_else(|| {
            2000 + i16::try_from(year.rem_euclid(I512::from(400))).expect("Gregorian remainder")
        });
    let component = |value: Option<I512>, default: i64, bits: u32| -> Result<i64> {
        let wide = value.unwrap_or(I512::from(default));
        let n = i32::try_from(wide)
            .map_err(|_| range(format!("integer {wide} too big to convert to 'int'")))?;
        if n < 0 || n >= (1 << bits) {
            return Err(argument("argument out of range"));
        }
        Ok(i64::from(n))
    };
    let mon = component(parts.mon, i64::from(current.month()), 4)?;
    let day = component(
        parts.mday,
        if parts.year.is_some() || parts.mon.is_some() {
            1
        } else {
            i64::from(current.day())
        },
        5,
    )?;
    let hour = component(parts.hour, 0, 5)?;
    let min = component(parts.min, 0, 6)?;
    let sec = component(parts.sec, 0, 6)?;
    for (valid, message) in [
        ((1..=12).contains(&mon), "mon out of range"),
        ((1..=31).contains(&day), "mday out of range"),
        ((0..=24).contains(&hour), "hour out of range"),
        ((0..=59).contains(&min), "min out of range"),
        ((0..=60).contains(&sec), "sec out of range"),
        (hour != 24 || (min == 0 && sec == 0), "min out of range"),
    ] {
        if !valid {
            return Err(argument(message));
        }
    }
    let date = jiff::civil::Date::new(proxy_year, mon as i8, 1)
        .map_err(|_| argument("argument out of range"))?
        .checked_add(jiff::Span::new().days(day - 1))
        .map_err(|_| argument("argument out of range"))?;
    let dt = date
        .at(0, 0, 0, 0)
        .checked_add(
            jiff::Span::new()
                .hours(hour)
                .minutes(min)
                .seconds(sec)
                .nanoseconds(parts.nanosecond),
        )
        .map_err(|_| argument("argument out of range"))?;
    let shift = year - I512::from(proxy_year);
    let naive = Timestamp::from_wide_shifted_jiff(
        dt.to_zoned(TimeZone::UTC)
            .map_err(|_| argument("argument out of range"))?
            .timestamp(),
        shift,
    )
    .ok_or_else(|| argument("argument out of range"))?;
    let offset = parts.offset_nanoseconds.or_else(|| {
        boundary_offset(zone, naive.transition_second(), true).map(|s| s * 1_000_000_000)
    });
    let result = if let Some(offset) = offset {
        if offset.unsigned_abs() >= 86_400_000_000_000 {
            return Err(argument("utc_offset out of range"));
        }
        let utc = dt
            .to_zoned(TimeZone::UTC)
            .map_err(|_| argument("argument out of range"))?
            .timestamp()
            .checked_sub(jiff::SignedDuration::from_nanos(offset))
            .map_err(|_| argument("argument out of range"))?;
        Timestamp::from_wide_shifted_jiff(utc, shift)
    } else {
        super::time_parser::local_datetime(dt, zone)
            .and_then(|local| Timestamp::from_wide_shifted_jiff(local.jiff(), shift))
    };
    result
        .map(Some)
        .ok_or_else(|| argument("argument out of range"))
}
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
pub fn boundary_offset(zone: &jiff::tz::TimeZone, seconds: i64, local: bool) -> Option<i64> {
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
