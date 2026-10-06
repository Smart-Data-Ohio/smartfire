//! app/models/event/recurrence.rb. Generate every slot, including an over-cap range:
//! the model needs the actual count to construct Rails' validation message.
use jiff::{Span, civil::Date};

use crate::Timestamp;
use crate::slash_commands::time_parser::{known_zone, local};

pub const RULES: [&str; 4] = ["daily", "weekly", "biweekly", "monthly"];
pub const SCOPES: [&str; 2] = ["this_event", "this_and_following"];
pub const MAX_OCCURRENCES: usize = 52;

pub fn slots(
    starts_at: Option<Timestamp>,
    ends_at: Option<Timestamp>,
    time_zone: &str,
    rule: &str,
    until_date: Option<Date>,
) -> Vec<(Timestamp, Option<Timestamp>)> {
    let (Some(starts), Some(until), Some(zone)) = (starts_at, until_date, known_zone(time_zone))
    else {
        return Vec::new();
    };
    let first = starts.jiff().to_zoned(zone.clone());
    let duration = ends_at.map(|end| {
        jiff::SignedDuration::from_micros(end.as_microsecond() - starts.as_microsecond())
    });
    let mut date = first.date();
    let mut result = Vec::new();
    let mut month = date.first_of_month();
    while date <= until {
        let Some(start) = local(date, first.hour(), first.minute(), first.second(), 0, &zone)
        else {
            return Vec::new();
        };
        result.push((start, duration.map(|duration| start.since(duration))));
        let next = if rule == "monthly" {
            let Ok(next_month) = month.checked_add(Span::new().months(1)) else {
                break;
            };
            month = next_month;
            Date::new(
                month.year(),
                month.month(),
                first.day().min(month.days_in_month()),
            )
        } else {
            let days = match rule {
                "daily" => 1,
                "biweekly" => 14,
                _ => 7,
            };
            date.checked_add(Span::new().days(days))
        };
        let Ok(next) = next else { break };
        date = next;
    }
    result
}

pub fn phrase(rule: &str) -> &str {
    if rule == "biweekly" {
        "every two weeks"
    } else {
        rule
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(serde::Deserialize)]
    struct Vector {
        name: String,
        time_zone: String,
        starts_at: String,
        ends_at: Option<String>,
        rule: String,
        until_date: Date,
        slots: Vec<(String, Option<String>)>,
    }
    #[test]
    fn pinned_rails_recurrence_vectors() {
        let cases: Vec<Vector> = serde_json::from_str(include_str!("recurrence.json")).unwrap();
        for case in cases {
            let expected: Vec<_> = case
                .slots
                .into_iter()
                .map(|(s, e)| {
                    (
                        Timestamp::parse_db(&s).unwrap(),
                        e.map(|e| Timestamp::parse_db(&e).unwrap()),
                    )
                })
                .collect();
            assert_eq!(
                slots(
                    Timestamp::parse_db(&case.starts_at),
                    case.ends_at.as_deref().and_then(Timestamp::parse_db),
                    &case.time_zone,
                    &case.rule,
                    Some(case.until_date)
                ),
                expected,
                "{}",
                case.name
            );
        }
    }
    #[test]
    fn invalid_inputs_are_empty_and_zones_remain_case_sensitive() {
        let start = Timestamp::parse_db("2026-01-01 09:00:00");
        let until = Some("2026-01-02".parse().unwrap());
        assert!(slots(start, None, "america/new_york", "daily", until).is_empty());
        assert!(slots(None, None, "UTC", "daily", until).is_empty());
        assert!(slots(start, None, "UTC", "daily", None).is_empty());
    }
}
