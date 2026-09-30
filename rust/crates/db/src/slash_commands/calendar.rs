//! Calendar fields used by Time.zone.parse (Date._parse with comp=false).
//! Omitted civil parts use the request zone's date; week/ordinal fields are
//! recognized but TimeZone#parts_to_time ignores them, just as Rails does.
use super::time_parser::{local_datetime, month, normalized_date, parsed_offset};
use crate::{Error, Result, Timestamp};
use jiff::{Span, tz::TimeZone};
use regex::Regex;

fn re(pattern: &str) -> Regex {
    Regex::new(pattern).expect("calendar pattern")
}
#[derive(Default)]
struct Parts {
    year: Option<i16>,
    month: Option<i8>,
    day: Option<i8>,
    hour: i64,
    minute: i64,
    second: i64,
    nanos: i32,
    offset: Option<i32>,
    found: bool,
}
fn invalid() -> Error {
    Error::Other("invalid date".into())
}
fn number<T: std::str::FromStr>(s: &str) -> Result<T> {
    s.parse().map_err(|_| invalid())
}
pub(super) fn parse(text: &str, zone: &TimeZone, now: Timestamp) -> Result<Option<Timestamp>> {
    let text = campfire_richtext::ruby::strip(text);
    let mut p = Parts::default();
    // Date removes its clock before searching the rest for calendar fields.
    let clock = re(
        r"(?i)(?:\b|T)(?P<h>[0-9]{1,2})(?:(?::|h)(?P<m>[0-9]{2})(?:(?::|m)(?P<s>[0-9]{2})(?:[.,](?P<f>[0-9]+))?)?)\s*(?P<a>(?:am|pm)\b)?",
    );
    let ampm = re(r"(?i)\b(?P<h>[0-9]{1,2})\s*(?P<a>am|pm)\b");
    let c = clock.captures(text).or_else(|| ampm.captures(text));
    let date_text = if let Some(c) = c {
        p.hour = number(&c["h"])?;
        p.minute = c
            .name("m")
            .map(|m| number(m.as_str()))
            .transpose()?
            .unwrap_or(0);
        p.second = c
            .name("s")
            .map(|s| number(s.as_str()))
            .transpose()?
            .unwrap_or(0);
        if let Some(a) = c.name("a") {
            p.hour = p.hour % 12
                + if a.as_str().eq_ignore_ascii_case("pm") {
                    12
                } else {
                    0
                };
        }
        if let Some(f) = c.name("f") {
            let mut digits = f.as_str().chars().take(6).collect::<String>();
            while digits.len() < 6 {
                digits.push('0');
            }
            p.nanos = number::<i32>(&digits)? * 1000;
        }
        let matched = c.get(0).unwrap();
        let rest = campfire_richtext::ruby::strip(&text[matched.end()..]);
        let offset = super::time_parser::offset_pattern().find(rest);
        if let Some(offset) = offset {
            p.offset = parsed_offset(offset.as_str());
        }
        p.found = true;
        format!(
            "{} {}",
            &text[..matched.start()],
            offset.map_or(rest, |m| &rest[m.end()..])
        )
    } else {
        text.to_owned()
    };
    let numeric = re(r"\b(?P<a>[0-9]{1,4})(?P<sep>[-/.])(?P<b>[0-9]{1,2})[-/.](?P<c>[0-9]{1,4})\b");
    let two = re(r"\b(?P<m>[0-9]{1,2})/(?P<d>[0-9]{1,2})\b");
    if let Some(c) = numeric.captures(&date_text) {
        let a = &c["a"];
        let b = &c["b"];
        let z = &c["c"];
        if z.len() == 4 && a.len() < 4 {
            p.year = Some(number(z)?);
            p.month = Some(number(b)?);
            p.day = Some(number(a)?);
        } else {
            p.year = Some(number(a)?);
            p.month = Some(number(b)?);
            p.day = Some(number(z)?);
        }
        p.found = true;
    } else if let Some(c) = two.captures(&date_text) {
        p.month = Some(number(&c["m"])?);
        p.day = Some(number(&c["d"])?);
        p.found = true;
    } else {
        let names = r"jan[a-z]*|feb[a-z]*|mar[a-z]*|apr[a-z]*|may|jun[a-z]*|jul[a-z]*|aug[a-z]*|sep[a-z]*|oct[a-z]*|nov[a-z]*|dec[a-z]*";
        let patterns = [
            re(&format!(
                r"(?i)\b(?P<d>[0-9]{{1,2}})(?:st|nd|rd|th)?[ -]+(?P<m>{names})(?:[, -]+(?P<y>[0-9]{{2,4}}))?\b"
            )),
            re(&format!(
                r"(?i)\b(?P<m>{names})[ -]+(?P<d>[0-9]{{1,2}})(?:st|nd|rd|th)?\b(?:[, -]+(?P<y>[0-9]{{2,4}}))?"
            )),
        ];
        let month_year = re(&format!(r"(?i)\b(?P<m>{names})\s+(?P<y>[0-9]{{4}})\b"));
        if let Some(c) = patterns.iter().find_map(|r| r.captures(&date_text)) {
            p.month = month(&c["m"]);
            p.day = Some(number(&c["d"])?);
            p.year = c.name("y").map(|y| number(y.as_str())).transpose()?;
            p.found = true;
        } else if let Some(c) = month_year.captures(&date_text) {
            p.month = month(&c["m"]);
            p.year = Some(number(&c["y"])?);
            p.found = true;
        } else if let Some(c) = re(&format!(r"(?i)\b(?P<m>{names})\b")).captures(&date_text) {
            p.month = month(&c["m"]);
            p.found = true;
        } else if let Some(c) = re(r"\b(?P<y>[0-9]{4})-[0-9]{3}\b").captures(&date_text) {
            p.year = Some(number(&c["y"])?);
            p.found = true;
        } else if re(r"(?i)\b[0-9]{4}-W[0-9]{2}-[0-9]\b").is_match(&date_text) {
            p.found = true;
        } else if let Some(c) = re(r"(?i)\b(?P<digits>[0-9]{2,14})(?:[T ](?P<clock>[0-9]{4}(?:[0-9]{2})?))?(?:\.(?P<fraction>[0-9]+))?(?P<zone>Z|[+-][0-9]{4})?\b").captures(&date_text) {
            let digits = &c["digits"];
            match digits.len() {
                14 | 12 | 10 | 8 => {
                    p.year = Some(number(&digits[..4])?);
                    p.month = Some(number(&digits[4..6])?);
                    p.day = Some(number(&digits[6..8])?);
                    if digits.len() > 8 {
                        p.hour = number(&digits[8..10])?;
                        p.minute = if digits.len() > 10 { number(&digits[10..12])? } else { 0 };
                        p.second = if digits.len() > 12 { number(&digits[12..])? } else { 0 };
                    }
                    p.found = true;
                }
                7 => {
                    p.year = Some(number(&digits[..4])?);
                    p.found = true; // yday is ignored by parts_to_time.
                }
                6 => {
                    p.year = Some(number(&digits[..2])?);
                    p.month = Some(number(&digits[2..4])?);
                    p.day = Some(number(&digits[4..])?);
                    p.found = true;
                }
                4 => {
                    p.month = Some(number(&digits[..2])?);
                    p.day = Some(number(&digits[2..])?);
                    p.found = true;
                }
                3 => p.found = true, // yday is ignored by parts_to_time.
                2 => {
                    p.day = Some(number(digits)?);
                    p.found = true;
                }
                _ => {}
            }
            if let Some(clock) = c.name("clock") {
                let digits = clock.as_str();
                p.hour = number(&digits[..2])?;
                p.minute = number(&digits[2..4])?;
                p.second = if digits.len() > 4 { number(&digits[4..])? } else { 0 };
            }
            if let Some(fraction) = c.name("fraction") {
                let mut digits = fraction.as_str().chars().take(6).collect::<String>();
                while digits.len() < 6 { digits.push('0'); }
                p.nanos = number::<i32>(&digits)? * 1000;
            }
            // Date's compact-number pass also consumes a trailing numeric zone
            // or Z, including a separated zone after a fourteen-digit clock.
            if p.offset.is_none() {
                p.offset = c.name("zone").and_then(|z| parsed_offset(z.as_str()));
                if p.offset.is_none() && (digits.len() > 8 || c.name("clock").is_some()) {
                    let rest = campfire_richtext::ruby::strip(&date_text[c.get(0).unwrap().end()..]);
                    if rest.starts_with(['+', '-']) || rest.starts_with(['Z', 'z']) {
                        p.offset = parsed_offset(rest);
                    }
                }
            }
        }
    }
    if !p.found
        && re(r"(?i)\b(?:mon|tue|wed|thu|fri|sat|sun)(?:day|sday|nesday|rsday|urday)?\b")
            .is_match(text)
    {
        p.found = true;
    }
    if !p.found {
        return Ok(None);
    }
    if p.hour > 24
        || p.minute > 59
        || p.second > 60
        || p.hour == 24 && (p.minute != 0 || p.second != 0)
    {
        return Err(invalid());
    }
    let today = now.jiff().to_zoned(zone.clone()).date();
    let day = p.day.unwrap_or(if p.year.is_some() || p.month.is_some() {
        1
    } else {
        today.day()
    });
    let date = normalized_date(
        p.year.unwrap_or(today.year()),
        p.month.unwrap_or(today.month()),
        day,
    )
    .ok_or_else(invalid)?;
    let datetime = date
        .at(0, 0, 0, p.nanos)
        .checked_add(Span::new().seconds(p.hour * 3600 + p.minute * 60 + p.second))
        .map_err(|_| invalid())?;
    Ok(if let Some(offset) = p.offset {
        Some(Timestamp::from_jiff(
            jiff::tz::Offset::from_seconds(offset)
                .map_err(|_| invalid())?
                .to_timestamp(datetime)
                .map_err(|_| invalid())?,
        ))
    } else {
        local_datetime(datetime, zone)
    })
}
