//! app/services/slash_commands/time_parser.rb. Civil days/weeks preserve local clock time;
//! minutes/hours are elapsed time. Rails preserves periods for civil changes;
//! fresh local parses prefer DST at folds and advance hourly through gaps.
use crate::Timestamp;
use bnum::types::I512;
use jiff::{
    SignedDuration, Span,
    civil::{Date, DateTime, Time},
    tz::TimeZone,
};
use regex::{Captures, Regex};
use std::sync::OnceLock;

pub const WEEKDAYS: [&str; 7] = [
    "sunday",
    "monday",
    "tuesday",
    "wednesday",
    "thursday",
    "friday",
    "saturday",
];
const CLOCK: &str = r"(?P<hour>[0-9]{1,2})(?::(?P<minute>[0-9]{2}))?\s*(?P<meridiem>am|pm)?";
const ISO: &str = r"(?P<iso>[0-9]{4}-[0-9]{2}-[0-9]{2}[ T][0-9]{1,2}:[0-9]{2}(?::[0-9]{2})?)";
fn patterns(trailing: bool) -> &'static Vec<Regex> {
    static LEADING: OnceLock<Vec<Regex>> = OnceLock::new();
    static TRAILING: OnceLock<Vec<Regex>> = OnceLock::new();
    let cell = if trailing { &TRAILING } else { &LEADING };
    cell.get_or_init(|| {
        let days = WEEKDAYS.join("|");
        [
            ISO.to_string(),
            r"in\s+(?P<amount>[0-9]+)\s*(?P<unit>minutes?|mins?|hours?|hrs?|days?|weeks?)\b".into(),
            format!(r"tomorrow(?:\s+at)?\s+{CLOCK}"),
            r"tomorrow\b".into(),
            format!(r"today(?:\s+at)?\s+{CLOCK}"),
            format!(r"at\s+{CLOCK}"),
            format!(r"next\s+(?P<weekday>{days})(?:\s+{CLOCK})?"),
            format!(r"(?P<weekday>{days})(?:\s+{CLOCK})?"),
        ]
        .into_iter()
        .enumerate()
        .map(|(i, p)| {
            // Ruby trailing patterns use a word boundary, except for ISO, and no final
            // boundary on weekdays. Preserve their precedence instead of longest-match.
            let flags = if i == 0 { "" } else { "(?i)" };
            let p = if trailing {
                format!(r"{flags}{}{p}\s*\z", if i == 0 { "" } else { r"\b" })
            } else {
                format!(r"{flags}\A{p}")
            };
            re(&p)
        })
        .collect()
    })
}
pub(crate) fn re(pattern: &str) -> Regex {
    // Ruby's \s is ASCII, while its word boundaries are Unicode aware.
    Regex::new(&pattern.replace(r"\s", r"(?-u:\s)")).expect("static Ruby pattern")
}
pub(crate) fn present(text: &str) -> Option<String> {
    (!text.chars().all(char::is_whitespace)).then(|| text.to_owned())
}
// Ruby String#strip removes ASCII NUL and whitespace, not NBSP; blank? is Unicode aware.
pub(crate) fn strip(text: &str) -> &str {
    text.trim_matches(|c: char| matches!(c, '\0' | '\t' | '\n' | '\x0b' | '\x0c' | '\r' | ' '))
}
pub(crate) fn known_zone(name: &str) -> Option<TimeZone> {
    // ActiveSupport::TimeZone[] resolves its exact aliases or TZInfo's case-sensitive
    // identifiers. Jiff alone accepts wrong-case names and additional host zones.
    // Generated from pinned Rails by reference-tools/auth/round_four.rb.
    #[derive(serde::Deserialize)]
    struct Names {
        identifiers: std::collections::HashSet<String>,
        mapping: std::collections::HashMap<String, String>,
    }
    static NAMES: OnceLock<Names> = OnceLock::new();
    let names = NAMES.get_or_init(|| {
        serde_json::from_str(include_str!("../rails_time_zones.json")).expect("pinned Rails zones")
    });
    let identifier = names.mapping.get(name).map(String::as_str).unwrap_or(name);
    if !names.identifiers.contains(identifier) {
        return None;
    }
    TimeZone::get(identifier).ok()
}
/// Rails request-local zone resolution, shared with authenticated integration services.
pub fn zone(name: &str) -> TimeZone {
    known_zone(name).unwrap_or(TimeZone::UTC)
}
pub(crate) fn local(
    date: Date,
    hour: i8,
    minute: i8,
    second: i8,
    nanosecond: i32,
    zone: &TimeZone,
) -> Option<Timestamp> {
    let time = Time::new(hour, minute, second, nanosecond).ok()?;
    resolve(DateTime::from_parts(date, time), zone, None)
}
/// Resolve a freshly parsed local datetime like ActiveSupport::TimeZone: prefer DST at
/// folds and advance one hour at a time through gaps, including non-hour transitions.
pub fn local_datetime(datetime: DateTime, zone: &TimeZone) -> Option<Timestamp> {
    resolve(datetime, zone, None)
}

fn resolve(
    mut dt: DateTime,
    zone: &TimeZone,
    preferred: Option<jiff::Timestamp>,
) -> Option<Timestamp> {
    // TimeWithZone retries missing local times one hour at a time. Advancing by the
    // gap size differs at Lord Howe's half-hour jump and Apia's skipped date.
    loop {
        let candidate = zone.to_ambiguous_zoned(dt);
        match candidate.offset() {
            jiff::tz::AmbiguousOffset::Gap { .. } => {
                dt = dt.checked_add(Span::new().hours(1)).ok()?
            }
            jiff::tz::AmbiguousOffset::Fold { .. } => {
                let earlier = candidate.clone().earlier().ok()?;
                let later = candidate.later().ok()?;
                let chosen = if preferred
                    .is_some_and(|source| same_period(zone, source, earlier.timestamp()))
                {
                    earlier
                } else if preferred
                    .is_some_and(|source| same_period(zone, source, later.timestamp()))
                {
                    later
                } else if zone.to_offset_info(earlier.timestamp()).dst().is_dst() {
                    earlier
                } else {
                    later
                };
                return Some(Timestamp::from_jiff(chosen.timestamp()));
            }
            _ => {
                return Some(Timestamp::from_jiff(
                    candidate.unambiguous().ok()?.timestamp(),
                ));
            }
        }
    }
}
fn same_period(zone: &TimeZone, a: jiff::Timestamp, b: jiff::Timestamp) -> bool {
    // TZInfo::TransitionsTimezonePeriod equality compares transition boundaries,
    // so an identical offset from an earlier season is not a preferred period.
    zone.following(a.min(b))
        .next()
        .is_none_or(|t| t.timestamp() > a.max(b))
}
pub(crate) fn add_days(time: Timestamp, days: i64, zone: &TimeZone) -> Option<Timestamp> {
    let original = time.jiff().to_zoned(zone.clone());
    resolve(
        original
            .datetime()
            .checked_add(Span::new().days(days))
            .ok()?,
        zone,
        Some(original.timestamp()),
    )
}
fn change_time(
    original: Timestamp,
    hour: i8,
    minute: i8,
    second: i8,
    nanosecond: i32,
    zone: &TimeZone,
) -> Option<Timestamp> {
    let original = original.jiff().to_zoned(zone.clone());
    resolve(
        DateTime::from_parts(
            original.date(),
            Time::new(hour, minute, second, nanosecond).ok()?,
        ),
        zone,
        Some(original.timestamp()),
    )
}
pub(crate) fn end_of_day(time: Timestamp, zone: &TimeZone) -> Option<Timestamp> {
    // TimeWithZone#end_of_day uses change, retaining the original period at folds.
    change_time(time, 23, 59, 59, 999_999_000, zone)
}
pub(crate) fn date_end_of_day(date: Date, zone: &TimeZone) -> Option<Timestamp> {
    // OOO parses a fresh midnight before applying end_of_day; status changes now.
    end_of_day(local(date, 0, 0, 0, 0, zone)?, zone)
}
pub(crate) fn normalized_date(year: i16, month: i8, day: i8) -> Option<Date> {
    // Ruby Time.new normalizes Feb 30, but rejects a day outside 1..31.
    if !(1..=31).contains(&day) {
        return None;
    }
    Date::new(year, month, 1)
        .ok()?
        .checked_add(Span::new().days(i64::from(day) - 1))
        .ok()
}
fn clock(c: &Captures<'_>) -> Option<(i8, i8)> {
    clock_parts(c, false)
}
fn clock_parts(c: &Captures<'_>, allow_midnight: bool) -> Option<(i8, i8)> {
    let Some(h) = c.name("hour") else {
        return Some((9, 0));
    };
    let mut h = h.as_str().parse::<i8>().ok()?;
    let m = c
        .name("minute")
        .map(|m| m.as_str().parse::<i8>())
        .transpose()
        .ok()?
        .unwrap_or(0);
    if h > if allow_midnight { 24 } else { 23 } || m > 59 || h == 24 && m != 0 {
        return None;
    }
    if let Some(meridiem) = c.name("meridiem") {
        if !(1..=12).contains(&h) {
            return None;
        }
        h %= 12;
        if meridiem.as_str().eq_ignore_ascii_case("pm") {
            h += 12;
        }
    }
    Some((h, m))
}
fn from_match(c: &Captures<'_>, zone: &TimeZone, now: Timestamp) -> Option<Timestamp> {
    if let Some(iso) = c.name("iso") {
        return fallback(iso.as_str(), zone, now);
    }
    let zoned = now.jiff().to_zoned(zone.clone());
    if let Some(amount) = c.name("amount") {
        let n = amount.as_str().parse::<I512>().ok()?;
        if n <= I512::ZERO {
            return None;
        }
        let unit = c.name("unit")?.as_str().to_ascii_lowercase();
        return if unit.starts_with("min") || unit.starts_with("h") {
            let multiplier = if unit.starts_with("min") { 60 } else { 3600 };
            // Ruby keeps Integer durations exact, including beyond i64 and Jiff's
            // civil range. Split only at Gregorian cycles, using the shared
            // Timestamp encoding rather than narrowing the requested duration.
            let seconds = n.checked_mul(I512::from(multiplier))?;
            let cycle = I512::from(146_097_i64 * 86_400);
            let remainder = i64::try_from(seconds % cycle).ok()?;
            let (proxy, shift) = now
                .since(SignedDuration::from_secs(remainder))
                .calendar_proxy();
            let year_shift = (seconds / cycle).checked_mul(I512::from(400))?;
            Timestamp::from_wide_shifted_jiff(proxy, shift.checked_add(year_shift)?)
        } else {
            relative_days(
                now,
                n.checked_mul(I512::from(if unit.starts_with("week") { 7 } else { 1 }))?,
                zone,
            )
        };
    }
    let (h, m) = clock(c)?;
    let matched = c.get(0)?.as_str().to_ascii_lowercase();
    let mut day = change_time(now, 0, 0, 0, 0, zone)?;
    if let Some(day) = c.name("weekday") {
        let target = WEEKDAYS
            .iter()
            .position(|d| d.eq_ignore_ascii_case(day.as_str()))? as i8;
        let mut delta =
            (target - zoned.date().weekday().to_sunday_zero_offset()).rem_euclid(7) as i64;
        let midnight = change_time(now, 0, 0, 0, 0, zone)?;
        if matched.starts_with("next")
            || delta == 0 && change_time(midnight, h, m, 0, 0, zone)? <= now
        {
            delta += 7;
        }
        return change_time(add_days(midnight, delta, zone)?, h, m, 0, 0, zone);
    } else if matched.contains("tomorrow") {
        day = change_time(add_days(now, 1, zone)?, 0, 0, 0, 0, zone)?;
    }
    let time = change_time(day, h, m, 0, 0, zone)?;
    if !matched.contains("tomorrow") && c.name("weekday").is_none() && time <= now {
        add_days(time, 1, zone)
    } else {
        Some(time)
    }
}
/// ActiveSupport::Duration applies civil days before resolving the target period.
/// A bounded 400-year remainder avoids Span's panic on large day counts. Use
/// TZInfo's real transition boundary for the actual year, never the proxy year.
fn relative_days(now: Timestamp, days: I512, zone: &TimeZone) -> Option<Timestamp> {
    let original = now.jiff().to_zoned(zone.clone());
    let cycle = I512::from(146_097);
    let target = original
        .datetime()
        .checked_add(Span::new().days(i64::try_from(days % cycle).ok()?))
        .ok()?;
    let naive = Timestamp::from_wide_shifted_jiff(
        target.to_zoned(TimeZone::UTC).ok()?.timestamp(),
        (days / cycle).checked_mul(I512::from(400))?,
    )?;
    if let Some(offset) = boundary_offset(zone, naive.transition_second(), true) {
        Some(naive.ago(SignedDuration::from_secs(offset)))
    } else {
        let dt = naive.try_jiff()?.to_zoned(TimeZone::UTC).datetime();
        resolve(dt, zone, Some(original.timestamp()))
    }
}
pub fn parse(text: &str, zone_name: &str, now: Timestamp) -> Option<Timestamp> {
    parse_checked(text, zone_name, now).ok().flatten()
}
/// Slash TimeParser rescues ArgumentError/TypeError, while RangeError propagates.
pub fn parse_checked(
    text: &str,
    zone_name: &str,
    now: Timestamp,
) -> crate::Result<Option<Timestamp>> {
    let text = strip(text);
    let zone = zone(zone_name);
    if let Some(c) = patterns(false).iter().find_map(|p| p.captures(text)) {
        return Ok(from_match(&c, &zone, now));
    }
    match super::calendar::parse(text, &zone, now) {
        Err(crate::Error::Other(message)) if message.starts_with("ArgumentError:") => Ok(None),
        result => result,
    }
}
pub fn boundary_offset(zone: &TimeZone, seconds: i64, local: bool) -> Option<i64> {
    super::calendar::boundary_offset(zone, seconds, local)
}
/// Builder inputs use Time.zone.parse, separately from slash-relative phrases.
/// Ruby's invalid calendar exception differs from an unrecognized string.
pub fn parse_calendar(
    text: &str,
    zone: &TimeZone,
    now: Timestamp,
) -> crate::Result<Option<Timestamp>> {
    super::calendar::parse(text, zone, now)
}
/// Calendar forms use TimeZone#parse directly, without the slash command grammar.
pub fn parse_calendar_time(
    text: &str,
    zone_name: &str,
    viewer_zone: &str,
    now: Timestamp,
) -> Option<Timestamp> {
    let zone = known_zone(zone_name).unwrap_or_else(|| zone(viewer_zone));
    fallback(text, &zone, now)
}

pub fn known_calendar_zone(name: &str) -> bool {
    known_zone(name).is_some()
}
pub fn split_leading_time(
    text: &str,
    zone_name: &str,
    now: Timestamp,
) -> Option<(Timestamp, Option<String>)> {
    let text = strip(text);
    let zone = zone(zone_name);
    let c = patterns(false).iter().find_map(|p| p.captures(text))?;
    Some((
        from_match(&c, &zone, now)?,
        present(strip(&text[c.get(0)?.end()..])),
    ))
}
pub fn split_trailing_time(
    text: &str,
    zone_name: &str,
    now: Timestamp,
) -> (Option<String>, Option<Timestamp>) {
    let text = strip(text);
    let zone = zone(zone_name);
    if let Some(c) = patterns(true).iter().find_map(|p| p.captures(text)) {
        let title = strip(&text[..c.get(0).unwrap().start()]);
        if present(title).is_none() {
            (Some(text.to_owned()), None)
        } else {
            (Some(title.to_owned()), from_match(&c, &zone, now))
        }
    } else {
        (present(text), None)
    }
}
pub(crate) fn month(name: &str) -> Option<i8> {
    [
        "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
    ]
    .iter()
    .position(|m| name.get(..3).is_some_and(|s| s.eq_ignore_ascii_case(m)))
    .map(|m| m as i8 + 1)
}
// ActiveSupport::TimeZone#parse delegates to Date._parse, with omitted date parts
// filled from now. Explicit offsets denote absolute instants; clock-only forms stay today.
pub(crate) fn fallback(text: &str, zone: &TimeZone, now: Timestamp) -> Option<Timestamp> {
    super::calendar::parse(text, zone, now).ok().flatten()
}
