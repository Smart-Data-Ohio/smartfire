//! `SlackImport.normalize_options` and Ruby's `time.rb#xmlschema` coercions.
use super::oauth::{present, string};
use serde_json::{Value, json};
use std::sync::LazyLock;
pub fn normalize(input: &Value) -> Result<Value, String> {
    let ids = match &input["conversation_ids"] {
        Value::Null => vec![],
        Value::Array(a) => a.clone(),
        Value::Object(o) => o.iter().map(|(k, v)| json!([k, v])).collect(),
        v => vec![v.clone()],
    };
    let mut unique = vec![];
    for id in ids {
        let text = string(&id);
        if present(&json!(text)) && !unique.contains(&text) {
            unique.push(text);
        }
    }
    let mut normalized = json!({"conversation_ids":if unique.is_empty(){Value::Null}else{json!(unique)},"oldest":null,"latest":null,"include_private":input["include_private"]!=false,"room_targets":input["room_targets"].as_object().cloned().unwrap_or_default()});
    for key in ["oldest", "latest"] {
        let value = &input[key];
        if !present(value) {
            continue;
        }
        normalized[key] = json!(time_bound(&string(value)).ok_or_else(|| format!(
            "Slack import {key} bound is not ISO 8601: {}",
            campfire_richtext::ruby::json_value_inspect(value)
        ))?);
    }
    Ok(normalized)
}
fn time_bound(text: &str) -> Option<String> {
    static ISO: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(r"(?i)\A[\t\n\r\x0B\x0C ]*(-?[0-9]+)-([0-9]{2})-([0-9]{2})T([0-9]{2}):([0-9]{2}):([0-9]{2})(?:\.([0-9]+))?(Z|[+-][0-9]{2}(?::?[0-9]{2})?)?[\t\n\r\x0B\x0C ]*\z").unwrap()
    });
    let caps = ISO.captures(text)?;
    let year = caps[1].parse::<i16>().ok()?;
    let month = caps[2].parse::<i8>().ok()?;
    let day = caps[3].parse::<i64>().ok()?;
    let hour = caps[4].parse::<i64>().ok()?;
    let minute = caps[5].parse::<i64>().ok()?;
    let second = caps[6].parse::<i64>().ok()?;
    if !(1..=31).contains(&day)
        || hour > 24
        || minute > 59
        || second > 60
        || (hour == 24 && (minute != 0 || second != 0))
    {
        return None;
    }
    let micros = caps
        .get(7)
        .map(|v| format!("{:<06}", &v.as_str()[..v.as_str().len().min(6)]).replace(' ', "0"))
        .unwrap_or_else(|| "000000".into())
        .parse::<i32>()
        .ok()?;
    let zone = caps.get(8).map(|s| s.as_str()).unwrap_or("+00:00");
    let utc = zone.eq_ignore_ascii_case("z");
    let offset = if utc {
        jiff::tz::Offset::UTC
    } else {
        let raw = zone.replace(':', "");
        let h = raw.get(1..3)?.parse::<i32>().ok()?;
        let m = raw.get(3..5).unwrap_or("00").parse::<i32>().ok()?;
        if h > 23 || m > 59 {
            return None;
        }
        jiff::tz::Offset::from_seconds(
            (h * 3600 + m * 60) * if raw.starts_with('-') { -1 } else { 1 },
        )
        .ok()?
    };
    let civil = jiff::civil::Date::new(year, month, 1)
        .ok()?
        .at(0, 0, 0, 0)
        .checked_add(
            jiff::Span::new()
                .days(day - 1)
                .hours(hour)
                .minutes(minute)
                .seconds(second)
                .microseconds(i64::from(micros)),
        )
        .ok()?;
    let time = civil.to_zoned(jiff::tz::TimeZone::fixed(offset)).ok()?;
    Some(
        time.strftime(if utc {
            "%Y-%m-%dT%H:%M:%S.%6fZ"
        } else {
            "%Y-%m-%dT%H:%M:%S.%6f%:z"
        })
        .to_string(),
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn slack_option_coercions_and_time_bounds_match_rails() {
        let vectors: Value =
            serde_json::from_str(include_str!("../../../../../vectors/slack/options.json"))
                .unwrap();
        for case in vectors.as_array().unwrap() {
            match normalize(&case["input"]) {
                Ok(value) => assert_eq!(value, case["result"], "{}", case["input"]),
                Err(error) => assert_eq!(error, case["error"], "{}", case["input"]),
            }
        }
    }
}
