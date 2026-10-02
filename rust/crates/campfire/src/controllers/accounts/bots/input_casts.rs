//! Request-only Ruby coercions; writes and validation remain with the model owners.
use campfire_db::{Timestamp, slash_commands::time_parser};
use campfire_kit::Param;
use campfire_views::time::Zone;
use regex::Regex;
use std::sync::LazyLock;

pub(super) fn datetime(param: Option<&Param>, zone: &Zone, now: Timestamp) -> Option<Timestamp> {
    let Param::Str(value) = param? else {
        return None;
    };
    // Jiff clamps a leap second to 59; Time.new instead rolls it into the next minute.
    static LEAP: LazyLock<Regex> = LazyLock::new(|| Regex::new(r":\d{2}:60\b").unwrap());
    if !LEAP.is_match(value) {
        if let Ok(at) = value.parse::<jiff::Timestamp>() {
            return Some(Timestamp::from_jiff(at));
        }
        if let Ok(at) = value.parse::<jiff::civil::DateTime>() {
            return time_parser::local_datetime(at, zone.tz());
        }
    }
    static YEAR_FIRST: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^(\d{1,4})([-.])(\d{1,2})[-.](\d{1,2})(.*)$").unwrap());
    let text = if let Some(parts) = YEAR_FIRST.captures(value) {
        // Time.new's ISO path preserves a short year literally. Dot-separated dates
        // reach Date._parse; normalize just their separators before the owner parser.
        format!(
            "{:04}-{}-{}{}",
            parts[1].parse::<i16>().ok()?,
            &parts[3],
            &parts[4],
            &parts[5]
        )
    } else {
        value.clone()
    };
    static DAY_MONTH_YEAR: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?i)\b(\d{1,2})[ -]+(jan[a-z]*|feb[a-z]*|mar[a-z]*|apr[a-z]*|may|jun[a-z]*|jul[a-z]*|aug[a-z]*|sep[a-z]*|oct[a-z]*|nov[a-z]*|dec[a-z]*)[, -]+(\d{4})\b").unwrap()
    });
    // The calendar parser's month/year branch otherwise shadows the supplied
    // day in this order. Preserve Date._parse's day by making it unambiguous.
    let text = DAY_MONTH_YEAR.replace(&text, "$2 $1, $3");
    if let Some(at) = time_parser::parse_calendar_time(&text, zone.name(), zone.name(), now) {
        return Some(at);
    }
    let current = now.jiff().to_zoned(zone.tz().clone());
    static WEEK: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?i)^\d{4}-?W\d{2}-?\d$").unwrap());
    static WEEKDAY: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?i)\b(mon|tue|wed|thu|fri|sat|sun)\b").unwrap());
    static ORDINAL: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(\d{4})-\d{3}$").unwrap());
    // TimeZone#parts_to_time ignores Date._parse's cwyear/cweek/cwday/wday keys.
    // A yday supplies only the calendar year; the omitted month comes from now,
    // and an explicit year makes the omitted day default to 1.
    let date = if let Some(parts) = ORDINAL.captures(value) {
        jiff::civil::Date::new(parts[1].parse().ok()?, current.month(), 1).ok()?
    } else if WEEK.is_match(value) || WEEKDAY.is_match(value) {
        current.date()
    } else {
        return None;
    };
    time_parser::local_datetime(date.at(0, 0, 0, 0), zone.tz())
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
        Param::Number(value) => value.to_string(),
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
