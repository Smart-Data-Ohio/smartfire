//! Validate the full form before any writes; audit each committed rule change separately.
use super::{blank, board, label, redirect, sentence};
use crate::app::AppCtx;
use crate::controllers::presenters::page::db_error;
use campfire_db::{BoardSlaRule, NewBoardSlaRule};
use campfire_kit::{Ctx, Error, Result, StatusCode};

pub async fn update_sla_rules(c: &mut Ctx) -> Result {
    let room = board(c).await?;
    let submitted = c.params.get("sla_rules").map(|p| p.to_json());
    let submitted = submitted
        .as_ref()
        .and_then(|value| permit_nested(value, true))
        .ok_or_else(|| Error::BadRequest("invalid SLA rules".into()))?;
    // Array#to_h accepts an empty filtered array; retained hashes raise in Rails.
    let submitted = match submitted {
        serde_json::Value::Object(fields) => fields,
        serde_json::Value::Array(values) if values.is_empty() => Default::default(),
        _ => return Err(Error::BadRequest("invalid SLA rules array".into())),
    };
    if submitted.values().any(serde_json::Value::is_array) {
        return Err(Error::BadRequest("invalid SLA status array".into()));
    }
    let updates: Vec<_> = campfire_db::models::channel_thread::WORK_STATUSES
        .iter()
        .map(|status| {
            let fields = submitted.get(*status).unwrap_or(&serde_json::Value::Null);
            let field = |name| {
                fields
                    .get(name)
                    .filter(|v| v.is_string() || v.is_number() || v.is_boolean())
                    .map(campfire_richtext::ruby::json_value_to_s)
                    .unwrap_or_default()
            };
            NewBoardSlaRule {
                room_id: room.id,
                work_status: Some((*status).into()),
                nudge_after_minutes: Some(
                    campfire_richtext::ruby::strip(&field("nudge_after_minutes")).into(),
                ),
                escalate_after_minutes: Some(
                    campfire_richtext::ruby::strip(&field("escalate_after_minutes")).into(),
                ),
            }
        })
        .collect();
    let room_id = room.id;
    let existing = c
        .app()
        .db
        .read(move |conn| BoardSlaRule::for_room(conn, room_id))
        .await
        .map_err(db_error)?;
    let validated = updates.clone();
    let before = existing.clone();
    let errors = c
        .app()
        .db
        .read(move |conn| {
            let mut messages = Vec::new();
            for input in validated {
                if blank(&input) {
                    continue;
                }
                let status = input.work_status.as_deref().unwrap();
                let errors = BoardSlaRule::validate(
                    conn,
                    &input,
                    before
                        .iter()
                        .find(|r| r.work_status == status)
                        .map(|r| r.id),
                )?;
                if !errors.is_empty() {
                    messages.push(format!(
                        "{}: {}",
                        label(status),
                        sentence(&errors.full_messages())
                    ));
                }
            }
            Ok(messages)
        })
        .await
        .map_err(db_error)?;
    if !errors.is_empty() {
        return Ok(c.head(StatusCode::UNPROCESSABLE_ENTITY));
    }
    for input in updates {
        let status = input.work_status.as_deref().unwrap().to_string();
        let prior = existing.iter().find(|r| r.work_status == status).cloned();
        let changes = if blank(&input) {
            let Some(prior) = prior else {
                continue;
            };
            c.app()
                .db
                .write(move |tx| prior.destroy(tx))
                .await
                .map_err(db_error)?;
            serde_json::json!({"sla_rule":"removed","status":status})
        } else {
            let nudge = input
                .nudge_after_minutes
                .as_ref()
                .unwrap()
                .parse::<i64>()
                .unwrap();
            let escalate = input
                .escalate_after_minutes
                .as_ref()
                .unwrap()
                .parse::<i64>()
                .unwrap();
            if let Some(mut prior) = prior {
                if prior.nudge_after_minutes == nudge && prior.escalate_after_minutes == escalate {
                    continue;
                }
                let changes = serde_json::json!({"sla_rule":"updated","status":status,"nudge_after_minutes":{"before":prior.nudge_after_minutes,"after":nudge},"escalate_after_minutes":{"before":prior.escalate_after_minutes,"after":escalate}});
                c.app()
                    .db
                    .write(move |tx| {
                        prior.update(tx, input.nudge_after_minutes, input.escalate_after_minutes)
                    })
                    .await
                    .map_err(db_error)?;
                changes
            } else {
                c.app()
                    .db
                    .write(move |tx| BoardSlaRule::create(tx, input))
                    .await
                    .map_err(db_error)?;
                serde_json::json!({"sla_rule":"created","status":status,"nudge_after_minutes":nudge,"escalate_after_minutes":escalate})
            }
        };
        c.app().broadcasts.board_automations_changed(room.id);
        super::super::audit_room(c, &room, "board.automation.change", changes).await?;
    }
    redirect(c, &room, Some("SLA timers saved."), None)
}

// Pinned ActionController::Parameters#permit: a numeric-key hash is a nested
// attributes collection whenever a numeric key has a hash value. In that case
// direct keys are discarded, even when mixed with otherwise permitted statuses.
fn numeric_attribute(key: &str, value: &serde_json::Value) -> bool {
    let key = key.strip_prefix('-').unwrap_or(key);
    !key.is_empty() && key.bytes().all(|b| b.is_ascii_digit()) && value.is_object()
}
fn permit_nested(value: &serde_json::Value, statuses: bool) -> Option<serde_json::Value> {
    use serde_json::Value;
    match value {
        Value::Array(values) => Some(Value::Array(
            values
                .iter()
                .filter_map(|v| v.as_object().map(|fields| permit_fields(fields, statuses)))
                .collect(),
        )),
        Value::Object(fields)
            if fields
                .iter()
                .any(|(key, value)| numeric_attribute(key, value)) =>
        {
            Some(Value::Object(
                fields
                    .iter()
                    .filter(|(key, value)| numeric_attribute(key, value))
                    .map(|(key, value)| {
                        (
                            key.clone(),
                            permit_fields(value.as_object().unwrap(), statuses),
                        )
                    })
                    .collect(),
            ))
        }
        Value::Object(fields) => Some(permit_fields(fields, statuses)),
        _ => None,
    }
}
fn permit_fields(
    fields: &serde_json::Map<String, serde_json::Value>,
    statuses: bool,
) -> serde_json::Value {
    let mut permitted = serde_json::Map::new();
    let keys: &[&str] = if statuses {
        &campfire_db::models::channel_thread::WORK_STATUSES
    } else {
        &["nudge_after_minutes", "escalate_after_minutes"]
    };
    for key in keys {
        if let Some(value) = fields.get(*key) {
            if statuses {
                if let Some(value) = permit_nested(value, false) {
                    permitted.insert((*key).into(), value);
                }
            } else if !value.is_array() && !value.is_object() {
                permitted.insert((*key).into(), value.clone());
            }
        }
    }
    serde_json::Value::Object(permitted)
}
