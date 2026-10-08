//! Board automation settings, with the classic models, authorization and audit payloads.

use std::collections::BTreeMap;

use campfire_api_types as api;
use campfire_app::app::AppCtx;
use campfire_db::{BoardSlaRule, BoardTagAssignment, NewBoardSlaRule, Room, Tx, User};
use campfire_kit::{Ctx, Error, Result, StatusCode};
use campfire_rooms::controllers::rooms::audit_room;
use campfire_web::concerns;
use campfire_web::controllers::presenters::page::db_error;

use crate::agents::human;
use crate::dto;
use crate::endpoints::{before_actions, body, now};
use crate::error::{fail, not_found, record_invalid};

const SLA_STATUSES: [(api::WorkStatus, &str, &str); 3] = [
    (api::WorkStatus::Planned, "planned", "Planned"),
    (api::WorkStatus::InProgress, "inProgress", "In progress"),
    (api::WorkStatus::Blocked, "blocked", "Blocked"),
];

endpoint!(
    /// `GET /api/v1/rooms/:room_id/automations`
    show => show_automations
);
endpoint!(
    /// `POST /api/v1/rooms/:room_id/automations/tag_rules`
    create_tag_rule => post_tag_rule
);
endpoint!(
    /// `DELETE /api/v1/rooms/:room_id/automations/tag_rules/:id`
    destroy_tag_rule => delete_tag_rule
);
endpoint!(
    /// `PUT /api/v1/rooms/:room_id/automations/sla_timers`
    update_sla_timers => put_sla_timers
);

/// `rooms/boards/automations#board`: membership and board kind precede administration.
async fn board(c: &mut Ctx) -> Result<Room> {
    before_actions(c).await?;
    let Some(viewer) = human(c)? else {
        return Err(fail(c, not_found()));
    };
    let id = c.param_str("room_id").and_then(concerns::cast_integer);
    let viewer_id = viewer.id;
    let room = c
        .app()
        .db
        .read(move |conn| match id {
            Some(id) => Room::find_for_user(conn, viewer_id, id),
            None => Ok(None),
        })
        .await
        .map_err(db_error)?
        .filter(Room::board)
        .ok_or(Error::NotFound)?;
    if !viewer.can_administer(Some(room.creator_id), false) {
        return Err(fail(
            c,
            api::ApiError::Forbidden {
                message: "Not allowed".into(),
            },
        ));
    }
    Ok(room)
}

async fn render(c: &mut Ctx, room_id: i64, status: StatusCode) -> Result {
    let (secrets, now) = (c.app().secrets.clone(), now(c));
    let settings = c
        .app()
        .db
        .read(move |conn| {
            let assignments = BoardTagAssignment::for_room(conn, room_id)?;
            let mut candidates = conn
                .prepare("SELECT u.* FROM memberships m JOIN users u ON u.id=m.user_id WHERE m.room_id=?")?
                .query_map([room_id], User::from_row)?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            candidates.retain(User::is_active);
            candidates.sort_by_key(|user| rails_compat::unicode::downcase(&user.name));
            let users = dto::users(
                conn,
                &secrets,
                assignments
                    .iter()
                    .map(|assignment| assignment.assignee_id)
                    .chain(candidates.iter().map(|user| user.id)),
                now,
            )?;
            let rules = BoardSlaRule::for_room(conn, room_id)?;
            let sla_timers = SLA_STATUSES
                .into_iter()
                .filter_map(|(status, _, _)| {
                    rules
                        .iter()
                        .find(|rule| rule.work_status == crate::work::stored_status(status))
                        .map(|rule| api::BoardSlaTimer {
                            status,
                            nudge_after_minutes: rule.nudge_after_minutes,
                            escalate_after_minutes: rule.escalate_after_minutes,
                        })
                })
                .collect();
            Ok(api::BoardAutomations {
                room_id,
                tag_rules: assignments
                    .into_iter()
                    .map(|assignment| api::BoardTagRule {
                        id: assignment.id,
                        tag: assignment.tag,
                        assignee_id: assignment.assignee_id,
                    })
                    .collect(),
                sla_timers,
                candidates: candidates.into_iter().map(|user| user.id).collect(),
                users,
            })
        })
        .await
        .map_err(db_error)?;
    c.json(status, &settings)
}

async fn show_automations(c: &mut Ctx) -> Result {
    let room = board(c).await?;
    render(c, room.id, StatusCode::OK).await
}

async fn post_tag_rule(c: &mut Ctx) -> Result {
    let room = board(c).await?;
    let input: api::CreateBoardTagRule = body(c).await?;
    let room_id = room.id;
    let actor_id = concerns::require_current_user(c)?.id;
    let saved = c
        .app()
        .db
        .write(move |tx| {
            let rule = BoardTagAssignment::create_from_form(
                tx,
                room_id,
                input.tag,
                input.assignee_id,
                actor_id,
            )?;
            let assignee = User::find(tx.conn(), rule.assignee_id)?.name;
            Ok((rule, assignee))
        })
        .await;
    let (rule, assignee) = match saved {
        Ok(saved) => saved,
        Err(campfire_db::Error::RecordInvalid(errors)) => {
            return Err(fail(
                c,
                record_invalid(
                    &errors,
                    &[
                        ("assignee", "assigneeId"),
                        ("created_by", "createdById"),
                        ("room", "roomId"),
                    ],
                ),
            ));
        }
        Err(error) => return Err(db_error(error)),
    };
    audit_room(
        c,
        &room,
        "board.automation.change",
        serde_json::json!({"tag_rule":"created","tag":rule.tag,"assignee":assignee}),
    )
    .await?;
    render(c, room.id, StatusCode::CREATED).await
}

async fn delete_tag_rule(c: &mut Ctx) -> Result {
    let room = board(c).await?;
    let room_id = room.id;
    let id = c.param_str("id").and_then(concerns::cast_integer);
    let removed = c
        .app()
        .db
        .write(move |tx| {
            let rule = match id {
                Some(id) => BoardTagAssignment::find(tx.conn(), id)?,
                None => None,
            };
            let Some(rule) = rule.filter(|rule| rule.room_id == room_id) else {
                return Ok(None);
            };
            let assignee = User::find(tx.conn(), rule.assignee_id)?.name;
            rule.destroy(tx)?;
            Ok(Some((rule.tag, assignee)))
        })
        .await
        .map_err(db_error)?;
    let Some((tag, assignee)) = removed else {
        return Err(fail(
            c,
            api::ApiError::NotFound {
                message: "Rule not found.".into(),
            },
        ));
    };
    audit_room(
        c,
        &room,
        "board.automation.change",
        serde_json::json!({"tag_rule":"removed","tag":tag,"assignee":assignee}),
    )
    .await?;
    render(c, room.id, StatusCode::OK).await
}

fn blank(input: &NewBoardSlaRule) -> bool {
    input.nudge_after_minutes.is_none() && input.escalate_after_minutes.is_none()
}

/// The classic automation form's `sentence` helper, including its Oxford comma.
fn sentence(messages: &[String]) -> String {
    match messages {
        [] => String::new(),
        [one] => one.clone(),
        [a, b] => format!("{a} and {b}"),
        many => format!(
            "{}, and {}",
            many[..many.len() - 1].join(", "),
            many.last().unwrap()
        ),
    }
}

fn write_sla_rule(
    tx: &mut Tx<'_>,
    input: NewBoardSlaRule,
    prior: Option<BoardSlaRule>,
) -> campfire_db::Result<Option<serde_json::Value>> {
    let status = input.work_status.as_deref().unwrap();
    if blank(&input) {
        let Some(prior) = prior else {
            return Ok(None);
        };
        prior.destroy(tx)?;
        return Ok(Some(
            serde_json::json!({"sla_rule":"removed","status":status}),
        ));
    }
    let nudge: i64 = input.nudge_after_minutes.as_ref().unwrap().parse().unwrap();
    let escalate: i64 = input
        .escalate_after_minutes
        .as_ref()
        .unwrap()
        .parse()
        .unwrap();
    let changes = if let Some(mut prior) = prior {
        if prior.nudge_after_minutes == nudge && prior.escalate_after_minutes == escalate {
            return Ok(None);
        }
        let changes = serde_json::json!({"sla_rule":"updated","status":status,"nudge_after_minutes":{"before":prior.nudge_after_minutes,"after":nudge},"escalate_after_minutes":{"before":prior.escalate_after_minutes,"after":escalate}});
        prior.update(tx, input.nudge_after_minutes, input.escalate_after_minutes)?;
        changes
    } else {
        let changes = serde_json::json!({"sla_rule":"created","status":status,"nudge_after_minutes":nudge,"escalate_after_minutes":escalate});
        BoardSlaRule::create(tx, input)?;
        changes
    };
    Ok(Some(changes))
}

async fn put_sla_timers(c: &mut Ctx) -> Result {
    let room = board(c).await?;
    let input: api::UpdateBoardSlaTimers = body(c).await?;
    let room_id = room.id;
    let result = c
        .app()
        .db
        .write(move |tx| {
            let existing = BoardSlaRule::for_room(tx.conn(), room_id)?;
            let updates: Vec<_> = SLA_STATUSES
                .into_iter()
                .zip([input.planned, input.in_progress, input.blocked])
                .map(|((status, field, label), values)| {
                    (
                        field,
                        label,
                        NewBoardSlaRule {
                            room_id,
                            work_status: Some(crate::work::stored_status(status).into()),
                            nudge_after_minutes: values
                                .nudge_after_minutes
                                .map(|value| value.to_string()),
                            escalate_after_minutes: values
                                .escalate_after_minutes
                                .map(|value| value.to_string()),
                        },
                    )
                })
                .collect();
            let mut fields = BTreeMap::new();
            let mut messages = Vec::new();
            for (field, label, input) in &updates {
                if blank(input) {
                    continue;
                }
                let errors = BoardSlaRule::validate(
                    tx.conn(),
                    input,
                    existing
                        .iter()
                        .find(|rule| Some(&rule.work_status) == input.work_status.as_ref())
                        .map(|rule| rule.id),
                )?;
                if !errors.is_empty() {
                    let full_messages = errors.full_messages();
                    messages.push(format!("{label}: {}", sentence(&full_messages)));
                    fields.insert((*field).to_string(), full_messages);
                }
            }
            if !fields.is_empty() {
                return Ok(Err(api::ApiError::Validation {
                    message: sentence(&messages),
                    fields,
                }));
            }
            let mut audits = Vec::new();
            for (_, _, input) in updates {
                let prior = existing
                    .iter()
                    .find(|rule| Some(&rule.work_status) == input.work_status.as_ref())
                    .cloned();
                if let Some(changes) = write_sla_rule(tx, input, prior)? {
                    audits.push(changes);
                }
            }
            Ok(Ok(audits))
        })
        .await
        .map_err(db_error)?;
    let audits = result.map_err(|error| fail(c, error))?;
    // As classic, audits follow the committed domain writes; an audit failure cannot undo them.
    for changes in audits {
        audit_room(c, &room, "board.automation.change", changes).await?;
    }
    render(c, room.id, StatusCode::OK).await
}
