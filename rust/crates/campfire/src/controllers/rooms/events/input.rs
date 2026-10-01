//! Strong parameters and the two different Rails event time parsing paths.
use campfire_db::{
    CalendarEvent, NewCalendarEvent, Timestamp, models::calendar_event::changes::EventChanges,
    slash_commands::time_parser,
};
use campfire_kit::{
    Error, Result,
    params::{Param, ParamMap, Permit},
};
const KEYS: [&str; 9] = [
    "title",
    "description",
    "starts_at",
    "ends_at",
    "time_zone",
    "recurrence_rule",
    "recurrence_until",
    "venue_room_id",
    "meet_link_requested",
];
fn permit(params: &ParamMap) -> Result<ParamMap> {
    let p = params.require("event")?.as_hash().ok_or_else(|| {
        Error::internal(anyhow::anyhow!(
            "Rails event parameters do not respond to permit"
        ))
    })?;
    Ok(p.permit(
        &KEYS
            .iter()
            .map(|k| Permit::Key((*k).into()))
            .collect::<Vec<_>>(),
    ))
}
fn scalar(p: &Param) -> String {
    match p {
        Param::Str(s) => s.clone(),
        Param::Bool(b) => b.to_string(),
        Param::Number(n) => n.to_string(),
        Param::Null => String::new(),
        _ => String::new(),
    }
}
fn model_string(p: &Param) -> String {
    match p {
        Param::Bool(true) => "t".into(),
        Param::Bool(false) => "f".into(),
        _ => scalar(p),
    }
}
fn text(p: &ParamMap, k: &str) -> Option<String> {
    p.get(k).map(model_string)
}
fn nullable(p: &ParamMap, k: &str) -> Option<Option<String>> {
    p.get(k).map(|v| {
        if v.is_null() {
            None
        } else {
            Some(model_string(v))
        }
    })
}
fn boolean(p: &Param) -> bool {
    !matches!(p, Param::Null | Param::Bool(false))
        && !matches!(
            scalar(p).as_str(),
            "" | "0" | "f" | "F" | "false" | "FALSE" | "off" | "OFF"
        )
}
fn date(p: &ParamMap, k: &str) -> Option<Option<jiff::civil::Date>> {
    p.get(k).map(|v| scalar(v).parse().ok())
}
fn venue(p: &ParamMap) -> Option<Option<i64>> {
    p.get("venue_room_id").map(|v| {
        if v.is_null() || scalar(v).is_empty() {
            None
        } else {
            Some(crate::concerns::ruby_to_i(&scalar(v)))
        }
    })
}
pub fn attributes(
    params: &ParamMap,
    event: Option<&CalendarEvent>,
    viewer_zone: &str,
    now: Timestamp,
) -> Result<EventChanges> {
    let p = permit(params)?;
    let zone = event.map(|e| e.time_zone.clone()).unwrap_or_else(|| {
        text(&p, "time_zone")
            .filter(|s| !campfire_richtext::ruby::is_blank(s))
            .unwrap_or_else(|| "UTC".into())
    });
    let parse = |key| {
        p.get(key)
            .filter(|p| p.is_present())
            .and_then(|v| time_parser::parse_calendar_time(&scalar(v), &zone, viewer_zone, now))
    };
    Ok(EventChanges {
        title: text(&p, "title"),
        description: nullable(&p, "description"),
        starts_at: Some(parse("starts_at")),
        ends_at: Some(parse("ends_at")),
        time_zone: Some(zone),
        venue_room_id: venue(&p),
        recurrence_rule: nullable(&p, "recurrence_rule"),
        recurrence_until: date(&p, "recurrence_until"),
        meet_link_requested: p.get("meet_link_requested").map(boolean),
    })
}
pub fn new_attributes(changes: EventChanges, room_id: i64, user_id: i64) -> NewCalendarEvent {
    NewCalendarEvent {
        room_id,
        organizer_id: user_id,
        title: changes.title.unwrap_or_default(),
        description: changes.description.flatten(),
        starts_at: changes.starts_at.flatten(),
        ends_at: changes.ends_at.flatten(),
        time_zone: changes.time_zone.unwrap_or_else(|| "UTC".into()),
        venue_room_id: changes.venue_room_id.flatten(),
        recurrence_rule: changes.recurrence_rule.flatten(),
        recurrence_until: changes.recurrence_until.flatten(),
        meet_link_requested: changes.meet_link_requested.unwrap_or(false),
    }
}
pub fn attempted(changes: &EventChanges, e: &CalendarEvent) -> NewCalendarEvent {
    NewCalendarEvent {
        room_id: e.room_id,
        organizer_id: e.organizer_id,
        title: changes.title.clone().unwrap_or_else(|| e.title.clone()),
        description: changes
            .description
            .clone()
            .unwrap_or_else(|| e.description.clone()),
        starts_at: changes.starts_at.unwrap_or(Some(e.starts_at)),
        ends_at: changes.ends_at.unwrap_or(e.ends_at),
        time_zone: e.time_zone.clone(),
        venue_room_id: changes.venue_room_id.unwrap_or(e.venue_room_id),
        recurrence_rule: changes
            .recurrence_rule
            .clone()
            .unwrap_or_else(|| e.recurrence_rule.clone())
            .filter(|s| !campfire_richtext::ruby::is_blank(s)),
        recurrence_until: changes.recurrence_until.unwrap_or(e.recurrence_until),
        meet_link_requested: changes.meet_link_requested.unwrap_or(e.meet_link_requested),
    }
}
pub fn prefill(
    params: &ParamMap,
    viewer_zone: &str,
    now: Timestamp,
    room_id: i64,
    user_id: i64,
) -> Result<NewCalendarEvent> {
    let mut a = NewCalendarEvent {
        room_id,
        organizer_id: user_id,
        time_zone: "UTC".into(),
        ..Default::default()
    };
    let Some(p) = params.get("event").and_then(Param::as_hash) else {
        return Ok(a);
    };
    params.require("event")?;
    let p = p.permit(&[
        Permit::Key("title".into()),
        Permit::Key("starts_at".into()),
        Permit::Key("time_zone".into()),
    ]);
    if let Some(z) = text(&p, "time_zone").filter(|s| time_parser::known_calendar_zone(s)) {
        a.time_zone = z;
    }
    if let Some(t) = p.get("title").filter(|v| v.is_present()).map(scalar) {
        a.title = t
            .trim_matches(['\0', ' ', '\t', '\r', '\n', '\u{b}', '\u{c}'])
            .chars()
            .take(255)
            .collect();
    }
    a.starts_at = p
        .get("starts_at")
        .filter(|v| v.is_present())
        .and_then(|v| time_parser::parse_calendar_time(&scalar(v), viewer_zone, viewer_zone, now));
    Ok(a)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};
    fn fact(a: NewCalendarEvent) -> Value {
        json!({"title":a.title,"description":a.description,"starts_at":a.starts_at.map(|t|t.jiff().as_microsecond()),"ends_at":a.ends_at.map(|t|t.jiff().as_microsecond()),"time_zone":a.time_zone,"venue_room_id":a.venue_room_id,"recurrence_rule":a.recurrence_rule.filter(|s|!campfire_richtext::ruby::is_blank(s)),"recurrence_until":a.recurrence_until.map(|d|d.to_string()),"meet_link_requested":a.meet_link_requested})
    }
    #[test]
    fn controller_time_and_parameter_casts_match_pinned_rails() {
        let vectors: Value = serde_json::from_str(include_str!("input.json")).unwrap();
        let now = Timestamp::parse_db("2026-09-22 12:00:00").unwrap();
        let old = CalendarEvent {
            id: 1,
            room_id: 2,
            organizer_id: 3,
            title: "Before".into(),
            description: Some("Before description".into()),
            starts_at: Timestamp::parse_db("2026-10-05 09:00:00").unwrap(),
            ends_at: None,
            time_zone: "Eastern Time (US & Canada)".into(),
            venue_room_id: None,
            series_id: None,
            recurrence_rule: None,
            recurrence_until: None,
            meet_link_requested: false,
            meet_link: None,
            cancelled_at: None,
            reminded_at: None,
            created_at: now,
            updated_at: now,
        };
        let mut failures = Vec::new();
        for (i, v) in vectors.as_array().unwrap().iter().enumerate() {
            let Param::Hash(params) = Param::from_json(json!({"event":v["input"]})) else {
                unreachable!()
            };
            let mode = v["mode"].as_str().unwrap();
            let actual = if mode == "prefill" {
                prefill(&params, "Hawaii", now, 2, 3)
            } else {
                attributes(&params, (mode == "update").then_some(&old), "Hawaii", now).map(|c| {
                    if mode == "update" {
                        attempted(&c, &old)
                    } else {
                        new_attributes(c, 2, 3)
                    }
                })
            };
            let expected = if v.get("status").is_some() {
                json!({"status":v["status"]})
            } else {
                let mut f = v["fact"].clone();
                for k in ["starts_at", "ends_at"] {
                    f[k] = f[k]
                        .as_str()
                        .map(|s| json!(s.parse::<jiff::Timestamp>().unwrap().as_microsecond()))
                        .unwrap_or(Value::Null);
                }
                f
            };
            let actual = actual
                .map(fact)
                .unwrap_or_else(|e| json!({"status":e.status().as_u16()}));
            if actual != expected {
                failures.push(format!(
                    "{i} {mode} {:?}: actual {actual}, expected {expected}",
                    v["input"]
                ));
            }
        }
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    }
}
