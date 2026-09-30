//! app/services/slash_commands/time_parser.rb. Civil days/weeks preserve local clock time;
//! minutes/hours are elapsed time. Rails chooses DST at folds and advances through gaps.
use crate::Timestamp;
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
            let p = if trailing {
                format!(r"(?i){}{p}\s*\z", if i == 0 { "" } else { r"\b" })
            } else {
                format!(r"(?i)\A{p}")
            };
            Regex::new(&p).unwrap()
        })
        .collect()
    })
}
pub(crate) fn re(pattern: &str) -> Regex {
    Regex::new(pattern).expect("static Ruby pattern")
}
pub(crate) fn present(text: &str) -> Option<String> {
    (!text.chars().all(char::is_whitespace)).then(|| text.to_owned())
}
// Ruby String#strip removes ASCII NUL and whitespace, not NBSP; blank? is Unicode aware.
pub(crate) fn strip(text: &str) -> &str {
    text.trim_matches(|c: char| matches!(c, '\0' | '\t' | '\n' | '\x0b' | '\x0c' | '\r' | ' '))
}
pub(crate) fn known_zone(name: &str) -> Option<TimeZone> {
    static ALIASES: OnceLock<std::collections::HashMap<String, String>> = OnceLock::new();
    let aliases = ALIASES.get_or_init(|| {
        serde_json::from_str(include_str!("../tests/ws8_slash_zones.json")).unwrap()
    });
    TimeZone::get(aliases.get(name).map(String::as_str).unwrap_or(name)).ok()
}
pub(crate) fn zone(name: &str) -> TimeZone {
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
fn resolve(
    mut dt: DateTime,
    zone: &TimeZone,
    preferred: Option<jiff::tz::Offset>,
) -> Option<Timestamp> {
    // TimeWithZone retries missing local times one hour at a time. Advancing by the
    // gap size differs at Lord Howe's half-hour jump and Apia's skipped date.
    loop {
        let candidate = zone.to_ambiguous_zoned(dt);
        match candidate.offset() {
            jiff::tz::AmbiguousOffset::Gap { .. } => {
                dt = dt.checked_add(Span::new().hours(1)).ok()?
            }
            jiff::tz::AmbiguousOffset::Fold { before, after } => {
                let earlier = candidate.clone().earlier().ok()?;
                let later = candidate.later().ok()?;
                let chosen = if preferred == Some(before) {
                    earlier
                } else if preferred == Some(after) {
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
pub(crate) fn add_days(time: Timestamp, days: i64, zone: &TimeZone) -> Option<Timestamp> {
    let original = time.jiff().to_zoned(zone.clone());
    resolve(
        original
            .datetime()
            .checked_add(Span::new().days(days))
            .ok()?,
        zone,
        Some(original.offset()),
    )
}
pub(crate) fn end_of_day(date: Date, zone: &TimeZone) -> Option<Timestamp> {
    // End of day is one microsecond before the next midnight, including skipped dates.
    let date = local(date, 0, 0, 0, 0, zone)?
        .jiff()
        .to_zoned(zone.clone())
        .date();
    let next = date.checked_add(Span::new().days(1)).ok()?;
    Some(local(next, 0, 0, 0, 0, zone)?.ago(SignedDuration::from_micros(1)))
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
    if h > 23 || m > 59 {
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
        let n = amount.as_str().parse::<i64>().ok()?;
        if n <= 0 {
            return None;
        }
        let unit = c.name("unit")?.as_str().to_ascii_lowercase();
        return if unit.starts_with("min") || unit.starts_with("h") {
            let multiplier = if unit.starts_with("min") { 60 } else { 3600 };
            Some(Timestamp::from_jiff(
                now.jiff()
                    .checked_add(SignedDuration::from_secs(n.checked_mul(multiplier)?))
                    .ok()?,
            ))
        } else {
            add_days(
                now,
                n.checked_mul(if unit.starts_with("week") { 7 } else { 1 })?,
                zone,
            )
        };
    }
    let (h, m) = clock(c)?;
    let mut date = zoned.date();
    let matched = c.get(0)?.as_str().to_ascii_lowercase();
    if let Some(day) = c.name("weekday") {
        let target = WEEKDAYS
            .iter()
            .position(|d| d.eq_ignore_ascii_case(day.as_str()))? as i8;
        let mut delta = (target - date.weekday().to_sunday_zero_offset()).rem_euclid(7) as i64;
        if matched.starts_with("next") || delta == 0 && local(date, h, m, 0, 0, zone)? <= now {
            delta += 7;
        }
        date = add_days(local(date, 0, 0, 0, 0, zone)?, delta, zone)?
            .jiff()
            .to_zoned(zone.clone())
            .date();
    } else if matched.contains("tomorrow") {
        date = add_days(now, 1, zone)?.jiff().to_zoned(zone.clone()).date();
    }
    let time = local(date, h, m, 0, 0, zone)?;
    if !matched.contains("tomorrow") && c.name("weekday").is_none() && time <= now {
        add_days(time, 1, zone)
    } else {
        Some(time)
    }
}
pub fn parse(text: &str, zone_name: &str, now: Timestamp) -> Option<Timestamp> {
    let text = strip(text);
    let zone = zone(zone_name);
    if let Some(c) = patterns(false).iter().find_map(|p| p.captures(text)) {
        from_match(&c, &zone, now)
    } else {
        fallback(text, &zone, now)
    }
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
fn fallback(text: &str, zone: &TimeZone, now: Timestamp) -> Option<Timestamp> {
    let text = strip(text);
    if text.is_empty() {
        return None;
    }
    if let Ok(ts) = text.parse::<jiff::Timestamp>() {
        return Some(Timestamp::from_jiff(ts));
    }
    let zoned = now.jiff().to_zoned(zone.clone());
    let mut date = zoned.date();
    let mut found = false;
    if let Some(c)=re(r"(?P<year>[0-9]{4})-(?P<month>[0-9]{2})-(?P<day>[0-9]{2})").captures(text) {
        date=normalized_date(c["year"].parse().ok()?,c["month"].parse().ok()?,c["day"].parse().ok()?)?;found=true;
    }else if let Some(c)=re(r"(?i)\b(?P<month>jan[a-z]*|feb[a-z]*|mar[a-z]*|apr[a-z]*|may|jun[a-z]*|jul[a-z]*|aug[a-z]*|sep[a-z]*|oct[a-z]*|nov[a-z]*|dec[a-z]*)\s+(?P<day>[0-9]{1,2})(?:st|nd|rd|th)?(?:[, ]+(?P<year>[0-9]{4}))?").captures(text) {
        date=normalized_date(c.name("year").map(|y|y.as_str().parse::<i16>()).transpose().ok()?.unwrap_or(date.year()),month(&c["month"])?,c["day"].parse().ok()?)?;found=true;
    }
    let pattern = format!(r"(?i)(?:\b|T){CLOCK}(?::(?P<second>[0-9]{{2}}))?");
    let clocks = re(&pattern);
    let c = clocks
        .captures_iter(text)
        .find(|c| c.name("minute").is_some() || c.name("meridiem").is_some());
    if let Some(c) = c {
        let (h, m) = clock(&c)?;
        let s = c
            .name("second")
            .map(|s| s.as_str().parse::<i8>())
            .transpose()
            .ok()?
            .unwrap_or(0);
        return local(date, h, m, s, 0, zone);
    }
    if found
        || WEEKDAYS
            .iter()
            .any(|d| text.to_ascii_lowercase().contains(d))
    {
        local(date, 0, 0, 0, 0, zone)
    } else {
        None
    }
}
