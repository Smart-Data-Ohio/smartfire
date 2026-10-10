//! Rooms::Boards::AutomationsController: board membership first, then creator/admin.
mod sla;
use crate::app::AppCtx;
use crate::concerns::{Before, before_actions, require_current_user};
use crate::controllers::presenters::page::db_error;
use campfire_db::{BoardSlaRule, BoardTagAssignment, NewBoardSlaRule, Room, User};
use campfire_kit::{Ctx, Redirect, Result, StatusCode};
pub use sla::update_sla_rules;

pub async fn board(c: &mut Ctx) -> Result<Room> {
    before_actions(c, Before::default()).await?;
    let id = c.param_str("board_id").and_then(super::cast_integer);
    let user = require_current_user(c)?.id;
    let room = c
        .app()
        .db
        .read(move |conn| {
            Ok(match id {
                Some(id) => Room::find_for_user(conn, user, id)?,
                None => None,
            })
        })
        .await
        .map_err(db_error)?;
    let Some(room) = room.filter(Room::board) else {
        return super::inaccessible_room(c);
    };
    super::ensure_can_administer(c, &room)?;
    Ok(room)
}
fn path(room: &Room) -> String {
    format!("/rooms/boards/{}/automations", room.id)
}
fn redirect(c: &mut Ctx, room: &Room, notice: Option<&str>, alert: Option<&str>) -> Result {
    c.redirect_to_with(
        &path(room),
        Redirect {
            notice: notice.map(str::to_owned),
            alert: alert.map(str::to_owned),
            ..Default::default()
        },
    )
}
fn text(c: &Ctx, key: &str) -> String {
    c.params
        .get(key)
        .map(|p| match p.to_json() {
            // ActiveModel::Type::String casts booleans before normalize_tag.
            serde_json::Value::Bool(true) => "t".into(),
            serde_json::Value::Bool(false) => "f".into(),
            value => campfire_richtext::ruby::json_value_to_s(&value),
        })
        .unwrap_or_default()
}
pub async fn create_tag_assignment(c: &mut Ctx) -> Result {
    let room = board(c).await?;
    let assignee_id = match c.params.get("assignee_id").map(|p| p.to_json()) {
        Some(serde_json::Value::Bool(value)) => Some(i64::from(value)),
        Some(serde_json::Value::Number(value)) => value.as_i64().or_else(|| {
            value
                .as_f64()
                .filter(|n| n.is_finite() && *n >= i64::MIN as f64 && *n < -(i64::MIN as f64))
                .map(|n| n as i64)
        }),
        Some(serde_json::Value::String(value)) => {
            BoardSlaRule::cast_threshold(Some(&value)).and_then(|value| value.parse().ok())
        }
        _ => None,
    };
    let room_id = room.id;
    let tag = text(c, "tag");
    let created_by_id = require_current_user(c)?.id;
    let saved = c
        .app()
        .db
        .write(move |tx| {
            BoardTagAssignment::create_from_form(tx, room_id, tag, assignee_id, created_by_id)
        })
        .await;
    let saved = match saved {
        Ok(saved) => saved,
        Err(campfire_db::Error::RecordInvalid(_errors)) => {
            return Ok(c.head(StatusCode::UNPROCESSABLE_ENTITY));
        }
        Err(error) => return Err(db_error(error)),
    };
    c.app().broadcasts.board_automations_changed(room.id);
    let id = saved.assignee_id;
    let name = c
        .app()
        .db
        .read(move |conn| Ok(User::find(conn, id)?.name))
        .await
        .map_err(db_error)?;
    super::audit_room(
        c,
        &room,
        "board.automation.change",
        serde_json::json!({"tag_rule":"created","tag":saved.tag,"assignee":name}),
    )
    .await?;
    redirect(c, &room, Some("Auto-assign rule added."), None)
}
pub async fn destroy_tag_assignment(c: &mut Ctx) -> Result {
    let room = board(c).await?;
    let id = c.param_str("id").and_then(super::cast_integer);
    let room_id = room.id;
    let assignment = c
        .app()
        .db
        .read(move |conn| match id {
            Some(id) => BoardTagAssignment::find(conn, id),
            None => Ok(None),
        })
        .await
        .map_err(db_error)?
        .filter(|a| a.room_id == room_id);
    let Some(assignment) = assignment else {
        return redirect(c, &room, None, Some("Rule not found."));
    };
    let assignee = assignment.assignee_id;
    let name = c
        .app()
        .db
        .read(move |conn| Ok(User::find(conn, assignee)?.name))
        .await
        .map_err(db_error)?;
    let tag = assignment.tag.clone();
    c.app()
        .db
        .write(move |tx| assignment.destroy(tx))
        .await
        .map_err(db_error)?;
    c.app().broadcasts.board_automations_changed(room.id);
    super::audit_room(
        c,
        &room,
        "board.automation.change",
        serde_json::json!({"tag_rule":"removed","tag":tag,"assignee":name}),
    )
    .await?;
    redirect(c, &room, Some("Auto-assign rule removed."), None)
}

fn blank(input: &NewBoardSlaRule) -> bool {
    input
        .nudge_after_minutes
        .as_deref()
        .is_none_or(campfire_richtext::ruby::is_blank)
        && input
            .escalate_after_minutes
            .as_deref()
            .is_none_or(campfire_richtext::ruby::is_blank)
}
fn label(status: &str) -> &str {
    match status {
        "in_progress" => "In progress",
        "blocked" => "Blocked",
        "done" => "Done",
        _ => "Planned",
    }
}
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
