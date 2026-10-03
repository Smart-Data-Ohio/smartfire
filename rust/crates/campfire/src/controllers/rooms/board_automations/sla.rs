//! Validate the full form before any writes; audit each committed rule change separately.
use super::{blank, board, label, redirect, render, sentence};
use crate::app::AppCtx;
use crate::controllers::presenters::page::db_error;
use campfire_db::{BoardSlaRule, NewBoardSlaRule};
use campfire_kit::{Ctx, Error, Result, StatusCode};

pub async fn update_sla_rules(c: &mut Ctx) -> Result {
    let room = board(c).await?;
    let mut submitted = c.params.get("sla_rules").map(|p| p.to_json());
    if let Some(serde_json::Value::Array(values)) = &submitted {
        // Strong Parameters filters scalar array entries out. The resulting [] converts
        // to an empty hash, whereas retained hashes cannot convert to key/value pairs.
        if values.iter().any(serde_json::Value::is_object) {
            return Err(Error::BadRequest("invalid SLA rules array".into()));
        }
        submitted = Some(serde_json::json!({}));
    }
    // Rails fetches an unpermitted empty Parameters object if no accepted rule is present.
    let accepted = submitted.as_ref().is_some_and(serde_json::Value::is_object);
    if !accepted {
        return Err(Error::internal(anyhow::anyhow!(
            "unable to convert unpermitted parameters to hash"
        )));
    }
    if campfire_db::models::channel_thread::WORK_STATUSES
        .iter()
        .any(|status| {
            submitted
                .as_ref()
                .and_then(|value| value.get(status))
                .is_some_and(serde_json::Value::is_array)
        })
    {
        // Approved difference: Rails raises before applying this malformed form.
        return Err(Error::BadRequest("invalid SLA status array".into()));
    }
    let updates: Vec<_> = campfire_db::models::channel_thread::WORK_STATUSES
        .iter()
        .map(|status| {
            let fields = &submitted.as_ref().unwrap()[status];
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
        return render(
            c,
            room,
            None,
            Some(sentence(&errors)),
            updates,
            StatusCode::UNPROCESSABLE_ENTITY,
        )
        .await;
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
        super::super::audit_room(c, &room, "board.automation.change", changes).await?;
    }
    redirect(c, &room, Some("SLA timers saved."), None)
}
