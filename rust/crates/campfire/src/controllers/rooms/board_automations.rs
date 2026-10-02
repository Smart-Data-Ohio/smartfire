//! Rooms::Boards::AutomationsController: board membership first, then creator/admin.
mod sla;
use crate::app::AppCtx;
use crate::concerns::{Before, before_actions, require_current_user};
use crate::controllers::presenters::page::{self, db_error};
use campfire_db::{
    BoardSlaRule, BoardTagAssignment, NewBoardSlaRule, NewBoardTagAssignment, Room, User,
};
use campfire_kit::{Ctx, Redirect, Result, StatusCode};
use campfire_views::rooms::board_automations::{Settings, TagRule};
pub use sla::update_sla_rules;
use std::collections::BTreeMap;

async fn board(c: &mut Ctx) -> Result<Room> {
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
pub async fn show(c: &mut Ctx) -> Result {
    let room = board(c).await?;
    render(c, room, None, None, Vec::new(), StatusCode::OK).await
}
async fn render(
    c: &mut Ctx,
    room: Room,
    tag_error: Option<String>,
    sla_error: Option<String>,
    drafts: Vec<NewBoardSlaRule>,
    status: StatusCode,
) -> Result {
    let settings = c
        .app()
        .db
        .read(move |conn| {
            let assignments = BoardTagAssignment::for_room(conn, room.id)?;
            let ids = serde_json::json!(
                assignments
                    .iter()
                    .map(|a| a.assignee_id)
                    .collect::<Vec<_>>()
            )
            .to_string();
            let users: BTreeMap<_, _> = query_all(
                conn,
                "SELECT * FROM users WHERE id IN (SELECT value FROM json_each(?))",
                [ids],
                User::from_row,
            )?
            .into_iter()
            .map(|u| (u.id, u))
            .collect();
            let tags = assignments
                .into_iter()
                .map(|a| {
                    let u = users
                        .get(&a.assignee_id)
                        .ok_or(campfire_db::Error::RecordNotFound("User"))?;
                    Ok(TagRule {
                        id: a.id,
                        tag: a.tag,
                        name: u.name.clone(),
                        agent: u.is_bot(),
                    })
                })
                .collect::<campfire_db::Result<Vec<_>>>()?;
            let mut candidates = query_all(
                conn,
                "SELECT u.* FROM memberships m JOIN users u ON u.id=m.user_id WHERE m.room_id=?",
                [room.id],
                User::from_row,
            )?
            .into_iter()
            .filter(User::is_active)
            .collect::<Vec<_>>();
            candidates.sort_by_key(|u| rails_compat::unicode::downcase(&u.name));
            let mut rules: BTreeMap<_, _> = BoardSlaRule::for_room(conn, room.id)?
                .into_iter()
                .map(|r| {
                    (
                        r.work_status,
                        (
                            Some(r.nudge_after_minutes.to_string()),
                            Some(r.escalate_after_minutes.to_string()),
                        ),
                    )
                })
                .collect();
            // Association find_or_initialize_by retains new unsaved rules in its target, while
            // existing records found by a separate query are reloaded for the error page.
            for draft in drafts {
                if blank(&draft) {
                    continue;
                }
                rules.entry(draft.work_status.unwrap()).or_insert_with(|| {
                    (
                        draft_integer(draft.nudge_after_minutes.as_deref()),
                        draft_integer(draft.escalate_after_minutes.as_deref()),
                    )
                });
            }
            Ok(Settings {
                room_id: room.id,
                room_name: room.name.unwrap_or_default(),
                tags,
                candidates: candidates
                    .into_iter()
                    .map(|u| (u.name, u.id.to_string()))
                    .collect(),
                rules,
                tag_error,
                sla_error,
            })
        })
        .await
        .map_err(db_error)?;
    page::framed_page!(c, status, |ctx| {
        campfire_views::rooms::board_automations::Show {
            ctx,
            settings: &settings,
        }
    })
    .await
}
fn text(c: &Ctx, key: &str) -> String {
    c.params
        .get(key)
        .map(|p| campfire_richtext::ruby::json_value_to_s(&p.to_json()))
        .unwrap_or_default()
}
pub async fn create_tag_assignment(c: &mut Ctx) -> Result {
    let room = board(c).await?;
    let input = NewBoardTagAssignment {
        room_id: room.id,
        tag: text(c, "tag"),
        assignee_id: super::cast_integer(&text(c, "assignee_id")).unwrap_or(0),
        created_by_id: require_current_user(c)?.id,
    };
    let saved = c
        .app()
        .db
        .write(move |tx| BoardTagAssignment::create(tx, input))
        .await;
    let saved = match saved {
        Ok(saved) => saved,
        Err(campfire_db::Error::RecordInvalid(errors)) => {
            return render(
                c,
                room,
                Some(sentence(&errors.full_messages())),
                None,
                Vec::new(),
                StatusCode::UNPROCESSABLE_ENTITY,
            )
            .await;
        }
        Err(error) => return Err(db_error(error)),
    };
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
    let id = c.param_str("id").and_then(super::cast_integer).unwrap_or(0);
    let room_id = room.id;
    let assignment = c
        .app()
        .db
        .read(move |conn| BoardTagAssignment::find(conn, id))
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
        .unwrap_or("")
        .is_empty()
        && input
            .escalate_after_minutes
            .as_deref()
            .unwrap_or("")
            .is_empty()
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

fn query_all<P, F>(
    conn: &campfire_db::Connection,
    sql: &str,
    params: P,
    map: F,
) -> campfire_db::Result<Vec<User>>
where
    P: rusqlite::Params,
    F: FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<User>,
{
    Ok(conn
        .prepare(sql)?
        .query_map(params, map)?
        .collect::<rusqlite::Result<Vec<_>>>()?)
}

fn draft_integer(value: Option<&str>) -> Option<String> {
    let value = value?.trim();
    if value.is_empty() {
        return None;
    }
    let negative = value.starts_with('-');
    let unsigned = value.strip_prefix(['+', '-']).unwrap_or(value);
    let digits: String = unsigned.chars().take_while(char::is_ascii_digit).collect();
    let digits = digits.trim_start_matches('0');
    Some(if digits.is_empty() {
        "0".into()
    } else if negative {
        format!("-{digits}")
    } else {
        digits.into()
    })
}
