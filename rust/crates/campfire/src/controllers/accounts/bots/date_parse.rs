//! Ruby date 3.4.1's non-TIGHT Date._parse passes, in their original order.
//! Source: ruby/date v3.4.1 ext/date/date_parse.c (BSD-2-Clause / Ruby).
//! Time.zone.parse passes comp=false and ignores week/ordinal/weekday fields.
use regex::Regex;
use std::sync::LazyLock;

#[derive(Debug, Default)]
pub(super) struct Parts {
    pub year: Option<i64>,
    pub mon: Option<i64>,
    pub mday: Option<i64>,
    pub hour: Option<i64>,
    pub min: Option<i64>,
    pub sec: Option<i64>,
    pub nanosecond: i64,
    pub offset_nanoseconds: Option<i64>,
    pub present: bool,
    zone: Option<String>,
    bc: bool,
}

fn regex(pattern: &str) -> Regex {
    // Onig's digits and whitespace are ASCII; word boundaries are Unicode.
    Regex::new(
        &pattern
            .replace(r"\d", "[0-9]")
            .replace(r"\D", "[^0-9]")
            .replace(r"\s", r"[ \t\r\n\x0b\x0c]")
            .replace(r"\S", r"[^ \t\r\n\x0b\x0c]"),
    )
    .expect("Ruby date regex")
}
macro_rules! re {
    ($name:ident, $pattern:expr) => {
        static $name: LazyLock<Regex> = LazyLock::new(|| regex($pattern));
    };
}
const MONTHS: [&str; 12] = [
    "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
];
fn month(s: &str) -> String {
    (MONTHS
        .iter()
        .position(|m| s.eq_ignore_ascii_case(m))
        .unwrap_or(MONTHS.len())
        + 1)
    .to_string()
}
fn number(s: &str) -> Option<i64> {
    let start = s.find(|c: char| c.is_ascii_digit() || c == '-' || c == '+')?;
    let s = &s[start..];
    let skip = usize::from(s.starts_with(['-', '+']));
    let len = skip + s[skip..].bytes().take_while(u8::is_ascii_digit).count();
    s[..len].parse().ok()
}
fn unsigned(s: &str) -> Option<i64> {
    number(s.trim_start_matches(|c: char| !c.is_ascii_digit()))
}
fn nanos(s: &str) -> i64 {
    let mut digits: String = s.chars().take(9).collect();
    while digits.len() < 9 {
        digits.push('0');
    }
    digits.parse().unwrap_or(0)
}
/// sub! replaces the first match with one space. NUMBER's negative lookbehind
/// is checked at its numeric capture, without a backtracking regex engine.
fn take(s: &mut String, re: &Regex, numeric: Option<usize>) -> Option<Vec<Option<String>>> {
    let c = re.captures_iter(s).find(|c| {
        numeric.is_none_or(|index| {
            let at = c.get(index).unwrap().start();
            !c.get(index)
                .unwrap()
                .as_str()
                .starts_with(|c: char| c.is_ascii_digit())
                || at == 0
                || !s.as_bytes()[at - 1].is_ascii_digit()
        })
    })?;
    let m = c.get(0).unwrap();
    let range = m.range();
    let parts = c.iter().map(|m| m.map(|m| m.as_str().to_owned())).collect();
    s.replace_range(range, " ");
    Some(parts)
}
fn s3e(
    p: &mut Parts,
    mut y: Option<String>,
    mut m: Option<String>,
    mut d: Option<String>,
    bc: bool,
) {
    if y.is_some() && m.is_some() && d.is_none() {
        (y, m, d) = (d, y, m);
    }
    if y.is_none()
        && d.as_ref()
            .is_some_and(|d| d.len() > 2 || d.starts_with('\''))
    {
        y = d.take();
    }
    if let Some(year) = &y
        && let Some(start) = year.find(|c: char| c.is_ascii_digit() || c == '-' || c == '+')
    {
        let s = &year[start..];
        let skip = usize::from(s.starts_with(['-', '+']));
        let end = skip + s[skip..].bytes().take_while(u8::is_ascii_digit).count();
        if end < s.len() {
            let day = s[..end].to_owned();
            y = d.take();
            d = Some(day);
        }
    }
    if m.as_ref()
        .is_some_and(|m| m.len() > 2 || m.starts_with('\''))
    {
        (y, m, d) = (m, d, y);
    }
    if d.as_ref()
        .is_some_and(|d| d.len() > 2 || d.starts_with('\''))
    {
        std::mem::swap(&mut y, &mut d);
    }
    p.year = y.as_deref().and_then(number);
    p.mon = m.as_deref().and_then(unsigned);
    p.mday = d.as_deref().and_then(unsigned);
    p.bc |= bc;
    p.present = true;
}

pub(super) fn parse(input: &str) -> Option<Parts> {
    // date_core.c rb_date_s__parse's default limit is RSTRING_LEN, not chars.
    if input.len() > 128 {
        return None;
    }
    re!(CLEAN, r"[^\-+',./:@\p{L}\p{N}\[\]]+");
    let mut text = CLEAN.replace_all(input, " ").into_owned();
    let mut p = Parts::default();
    re!(DAY, r"(?i)\b(sun|mon|tue|wed|thu|fri|sat)[^-/\d\s]*");
    if take(&mut text, &DAY, None).is_some() {
        p.present = true;
    }
    time(&mut text, &mut p);
    date(&mut text, &mut p);
    re!(BC, r"(?i)\b(bc\b|bce\b|b\.c\.|b\.c\.e\.)");
    if take(&mut text, &BC, None).is_some() {
        p.bc = true;
    }
    re!(FRAG, r"\A\s*(\d{1,2})\s*\z");
    if let Some(c) = take(&mut text, &FRAG, None) {
        let n = number(c[1].as_deref().unwrap());
        if p.hour.is_some() && p.mday.is_none() && n.is_some_and(|n| (1..=31).contains(&n)) {
            p.mday = n;
        }
        if p.mday.is_some() && p.hour.is_none() && n.is_some_and(|n| (0..=24).contains(&n)) {
            p.hour = n;
        }
    }
    if p.bc {
        p.year = p.year.and_then(|y| 1_i64.checked_sub(y));
    }
    if p.offset_nanoseconds.is_none() {
        p.offset_nanoseconds = p.zone.as_deref().and_then(zone_offset);
    }
    Some(p)
}
fn time(s: &mut String, p: &mut Parts) {
    re!(
        TIME,
        r"(?i)(\d+\s*(?:(?::\s*\d+(?:\s*:\s*\d+(?:[,.]\d*)?)?|h(?:\s*\d+m?(?:\s*\d+s?)?)?)(?:\s*[ap](?:m\b|\.m\.))?|[ap](?:m\b|\.m\.)))(?:\s*((?:gmt|utc?)?[-+]\d+(?:[,.:]\d+(?::\d+)?)?|(?-i:[\p{L}.\s]+)(?:standard|daylight)\stime\b|(?-i:[\p{L}]+)(?:\sdst)?\b))?"
    );
    let Some(c) = take(s, &TIME, Some(1)) else {
        return;
    };
    re!(
        CLOCK,
        r"(?i)\A(\d+)h?(?:\s*:?\s*(\d+)m?(?:\s*:?\s*(\d+)(?:[,.](\d+))?s?)?)?(?:\s*([ap])(?:m\b|\.m\.))?"
    );
    let clock = CLOCK.captures(c[1].as_deref().unwrap()).unwrap();
    let mut hour = number(&clock[1]);
    if let Some(ap) = clock.get(5) {
        hour = hour.map(|h| {
            h % 12
                + if ap.as_str().eq_ignore_ascii_case("p") {
                    12
                } else {
                    0
                }
        });
    }
    p.hour = hour;
    p.min = clock.get(2).and_then(|m| number(m.as_str()));
    p.sec = clock.get(3).and_then(|m| number(m.as_str()));
    p.nanosecond = clock.get(4).map_or(0, |m| nanos(m.as_str()));
    p.zone = c[2].clone();
    p.present = true;
}
fn date(s: &mut String, p: &mut Parts) {
    re!(
        EU,
        r"(?i)('?\d+)[^-\d\s]*\s*(jan|feb|mar|apr|may|jun|jul|aug|sep|oct|nov|dec)[^-\d\s']*(?:\s*(?:\b(c(?:e|\.e\.)|b(?:ce|\.c\.e\.)|a(?:d|\.d\.)|b(?:c|\.c\.)))?\s*('?-?\d+(?:(?:st|nd|rd|th)\b)?))?"
    );
    if let Some(c) = take(s, &EU, Some(1)) {
        s3e(
            p,
            c[4].clone(),
            Some(month(c[2].as_deref().unwrap())),
            c[1].clone(),
            c[3].as_ref().is_some_and(|b| b.starts_with(['b', 'B'])),
        );
        return;
    }
    re!(
        US,
        r"(?i)\b(jan|feb|mar|apr|may|jun|jul|aug|sep|oct|nov|dec)[^-\d\s']*\s*('?\d+)[^-\d\s']*(?:\s*,?\s*(c(?:e|\.e\.)|b(?:ce|\.c\.e\.)|a(?:d|\.d\.)|b(?:c|\.c\.))?\s*('?-?\d+))?"
    );
    if let Some(c) = take(s, &US, None) {
        s3e(
            p,
            c[4].clone(),
            Some(month(c[1].as_deref().unwrap())),
            c[2].clone(),
            c[3].as_ref().is_some_and(|b| b.starts_with(['b', 'B'])),
        );
        return;
    }
    re!(ISO, r"('?[-+]?\d+)-(\d+)-('?-?\d+)");
    if let Some(c) = take(s, &ISO, Some(1)) {
        s3e(p, c[1].clone(), c[2].clone(), c[3].clone(), false);
        return;
    }
    re!(JIS, r"(?i)\b([mtshr])(\d+)\.(\d+)\.(\d+)");
    if let Some(c) = take(s, &JIS, None) {
        let era = match c[1].as_deref().unwrap().to_ascii_lowercase().as_str() {
            "m" => 1867,
            "t" => 1911,
            "s" => 1925,
            "h" => 1988,
            "r" => 2018,
            _ => 0,
        };
        p.year = c[2]
            .as_deref()
            .and_then(number)
            .and_then(|y| y.checked_add(era));
        p.mon = c[3].as_deref().and_then(number);
        p.mday = c[4].as_deref().and_then(number);
        p.present = true;
        return;
    }
    re!(
        VMS11,
        r"(?i)('?-?\d+)-(jan|feb|mar|apr|may|jun|jul|aug|sep|oct|nov|dec)[^-/.]*-('?-?\d+)"
    );
    if let Some(c) = take(s, &VMS11, Some(1)) {
        s3e(
            p,
            c[3].clone(),
            Some(month(c[2].as_deref().unwrap())),
            c[1].clone(),
            false,
        );
        return;
    }
    re!(
        VMS12,
        r"(?i)\b(jan|feb|mar|apr|may|jun|jul|aug|sep|oct|nov|dec)[^-/.]*-('?-?\d+)(?:-('?-?\d+))?"
    );
    if let Some(c) = take(s, &VMS12, None) {
        s3e(
            p,
            c[3].clone(),
            Some(month(c[1].as_deref().unwrap())),
            c[2].clone(),
            false,
        );
        return;
    }
    re!(SLA, r"('?-?\d+)/\s*('?\d+)(?:\D\s*('?-?\d+))?");
    if let Some(c) = take(s, &SLA, Some(1)) {
        s3e(p, c[1].clone(), c[2].clone(), c[3].clone(), false);
        return;
    }
    re!(DOT, r"('?-?\d+)\.\s*('?\d+)\.\s*('?-?\d+)");
    if let Some(c) = take(s, &DOT, Some(1)) {
        s3e(p, c[1].clone(), c[2].clone(), c[3].clone(), false);
        return;
    }
    if iso2(s, p) {
        return;
    }
    re!(YEAR, r"'(\d+)\b");
    if let Some(c) = take(s, &YEAR, None) {
        p.year = c[1].as_deref().and_then(number);
        p.present = true;
        return;
    }
    re!(
        MON,
        r"(?i)\b(jan|feb|mar|apr|may|jun|jul|aug|sep|oct|nov|dec)\S*"
    );
    if let Some(c) = take(s, &MON, None) {
        p.mon = Some(month(c[1].as_deref().unwrap()).parse().unwrap());
        p.present = true;
        return;
    }
    re!(MDAY, r"(?i)(\d+)(st|nd|rd|th)\b");
    if let Some(c) = take(s, &MDAY, Some(1)) {
        p.mday = c[1].as_deref().and_then(number);
        p.present = true;
        return;
    }
    ddd(s, p);
}
fn iso2(s: &mut String, p: &mut Parts) -> bool {
    re!(WEEK, r"(?i)\b(\d{2}|\d{4})?-?w(\d{2})(?:-?(\d))?\b");
    re!(WDAY, r"(?i)-w-(\d)\b");
    if take(s, &WEEK, None).is_some() || take(s, &WDAY, None).is_some() {
        p.present = true;
        return true;
    }
    re!(ISO23, r"--(\d{2})?-(\d{2})\b");
    if let Some(c) = take(s, &ISO23, None) {
        p.mon = c[1].as_deref().and_then(number);
        p.mday = c[2].as_deref().and_then(number);
        p.present = true;
        return true;
    }
    re!(ISO24, r"--(\d{2})(\d{2})?\b");
    if let Some(c) = take(s, &ISO24, None) {
        p.mon = c[1].as_deref().and_then(number);
        p.mday = c[2].as_deref().and_then(number);
        p.present = true;
        return true;
    }
    re!(NOT25, r"[,.](\d{2}|\d{4})-\d{3}\b");
    re!(ISO25, r"\b(\d{2}|\d{4})-(\d{3})\b");
    if !NOT25.is_match(s)
        && let Some(c) = take(s, &ISO25, None)
    {
        p.year = c[1].as_deref().and_then(number);
        p.present = true;
        return true;
    }
    re!(NOT26, r"\d-\d{3}\b");
    re!(ISO26, r"\b-(\d{3})\b");
    if !NOT26.is_match(s) && take(s, &ISO26, None).is_some() {
        p.present = true;
        return true;
    }
    false
}
fn ddd(s: &mut String, p: &mut Parts) {
    re!(
        DDD,
        r"(?i)([-+]?)(\d{2,14})(?:\s*t?\s*(\d{2,6})?(?:[,.](\d*))?)?(?:\s*(z\b|[-+]\d{1,4}\b|\[[-+]?\d[^\]]*\]))?"
    );
    let Some(c) = take(s, &DDD, Some(2)) else {
        return;
    };
    let digits = c[2].as_deref().unwrap();
    let len = digits.len();
    let n = |start, count| digits.get(start..start + count).and_then(number);
    let sign = if c[1].as_deref() == Some("-") { -1 } else { 1 };
    if c[3].is_none() && c[4].is_some() && !matches!(len, 9 | 11 | 13) {
        p.sec = n(len - 2, 2);
        if len >= 4 {
            p.min = n(len - 4, 2);
        } else if len == 3 {
            p.min = n(0, 1);
        }
        if len >= 6 {
            p.hour = n(len - 6, 2);
        } else if len == 5 {
            p.hour = n(0, 1);
        }
        if len >= 8 {
            p.mday = n(len - 8, 2);
        } else if len == 7 {
            p.mday = n(0, 1);
        }
        if len >= 10 {
            p.mon = n(len - 10, 2);
        }
        if len == 12 {
            p.year = n(0, 2).map(|n| n * sign);
        }
        if len == 14 {
            p.year = n(0, 4).map(|n| n * sign);
        }
    } else {
        match len {
            2 => p.mday = n(0, 2),
            4 => {
                p.mon = n(0, 2);
                p.mday = n(2, 2);
            }
            6 => {
                p.year = n(0, 2).map(|n| n * sign);
                p.mon = n(2, 2);
                p.mday = n(4, 2);
            }
            8 | 10 | 12 | 14 => {
                p.year = n(0, 4).map(|n| n * sign);
                p.mon = n(4, 2);
                p.mday = n(6, 2);
                if len >= 10 {
                    p.hour = n(8, 2);
                }
                if len >= 12 {
                    p.min = n(10, 2);
                }
                if len >= 14 {
                    p.sec = n(12, 2);
                }
            }
            5 => p.year = n(0, 2).map(|n| n * sign),
            7 => p.year = n(0, 4).map(|n| n * sign),
            _ => (),
        }
    }
    if let Some(time) = &c[3] {
        let len = time.len();
        let n = |start, count| time.get(start..start + count).and_then(number);
        if matches!(len, 2 | 4 | 6) {
            if c[4].is_some() {
                p.sec = n(len - 2, 2);
                if len >= 4 {
                    p.min = n(len - 4, 2);
                }
                if len >= 6 {
                    p.hour = n(len - 6, 2);
                }
            } else {
                p.hour = n(0, 2);
                if len >= 4 {
                    p.min = n(2, 2);
                }
                if len >= 6 {
                    p.sec = n(4, 2);
                }
            }
        }
    }
    if let Some(fraction) = &c[4] {
        p.nanosecond = nanos(fraction);
    }
    if let Some(zone) = &c[5] {
        if let Some(inner) = zone.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            let (offset, zone) = if let Some((a, b)) = inner.split_once(':') {
                (format!("{a}:"), b.to_owned())
            } else {
                (
                    if inner.starts_with(|c: char| c.is_ascii_digit()) {
                        format!("+{inner}")
                    } else {
                        inner.to_owned()
                    },
                    inner.to_owned(),
                )
            };
            p.offset_nanoseconds = zone_offset(&offset);
            p.zone = Some(zone);
        } else {
            p.zone = Some(zone.clone());
        }
    }
    p.present |= !matches!(len, 9 | 11 | 13) || c[3].is_some() || c[4].is_some() || c[5].is_some();
}
fn zone_offset(zone: &str) -> Option<i64> {
    static ZONES: LazyLock<std::collections::HashMap<&str, i64>> = LazyLock::new(|| {
        include_str!("date_zones.tsv")
            .lines()
            .map(|line| {
                let (name, offset) = line.split_once('\t').unwrap();
                (name, offset.parse().unwrap())
            })
            .collect()
    });
    let mut zone = zone
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase();
    let mut dst = 0;
    for suffix in [" daylight time", " standard time", " dst"] {
        if zone.ends_with(suffix) {
            zone.truncate(zone.len() - suffix.len());
            if suffix != " standard time" {
                dst = 3600;
            }
            break;
        }
    }
    if let Some(offset) = ZONES.get(zone.as_str()) {
        return Some((offset + dst) * 1_000_000_000);
    }
    let zone = zone
        .strip_prefix("gmt")
        .or_else(|| zone.strip_prefix("utc"))
        .unwrap_or(&zone);
    let sign = if zone.starts_with('-') {
        -1
    } else if zone.starts_with('+') {
        1
    } else {
        return None;
    };
    let digits = &zone[1..];
    let first = digits.bytes().take_while(u8::is_ascii_digit).count();
    let hour = digits.get(..first)?.parse::<i64>().ok()?;
    let rest = &digits[first..];
    let seconds = if let Some(rest) = rest.strip_prefix(':') {
        let mut fields = rest.split(':');
        let min = fields.next().and_then(unsigned).unwrap_or(0);
        let sec = fields.next().and_then(unsigned).unwrap_or(0);
        if hour > 23 || min > 59 || sec > 59 {
            return None;
        }
        (hour * 3600 + min * 60 + sec) * 1_000_000_000
    } else if rest.starts_with(['.', ',']) {
        if hour > 23 {
            return None;
        }
        let fraction = &rest[1..];
        let count = fraction
            .bytes()
            .take_while(u8::is_ascii_digit)
            .count()
            .min(7);
        let mut n = fraction.get(..count)?.parse::<i64>().unwrap_or(0);
        if fraction
            .as_bytes()
            .get(count)
            .is_some_and(|b| *b >= b'5' + u8::from(n % 2 == 0) && *b <= b'9')
        {
            n += 1;
        }
        hour * 3_600_000_000_000
            + (i128::from(n) * 3_600_000_000_000 / 10_i128.pow(count as u32)) as i64
    } else if digits.len() > 2 {
        let width = 2 - digits.len() % 2;
        let hour = digits.get(..width).and_then(unsigned).unwrap_or(0);
        let min = digits.get(width..width + 2).and_then(unsigned).unwrap_or(0);
        let sec = digits
            .get(width + 2..width + 4)
            .and_then(unsigned)
            .unwrap_or(0);
        (hour * 3600 + min * 60 + sec) * 1_000_000_000
    } else {
        hour.checked_mul(3_600_000_000_000)?
    };
    seconds.checked_mul(sign)
}
