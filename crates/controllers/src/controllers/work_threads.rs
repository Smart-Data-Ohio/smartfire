//! Human workspace work and handoff endpoints; use the shared WS12 model writers.
use crate::app::AppCtx;
use crate::concerns::{Before, before_actions, head, require_current_user};
use crate::controllers::{
    messages,
    presenters::page::db_error,
};
use campfire_db::{Agent, ChannelThread, HandoffPackage, Room, User, WorkHandoff};
use campfire_kit::{Ctx, Param, Result, StatusCode, format, halt};
use rusqlite::OptionalExtension;
use serde_json::{Value, json};
pub mod links;
pub use links::{create as create_link, destroy as destroy_link};

// ActiveRecord integer predicates, including ArrayHandler's single-value recursion.
// Multi-value arrays cast each member, without flattening nested collections.
fn finder_ids(value: Option<&Param>) -> Vec<i64> {
    fn integer(value: &Param) -> Option<i64> {
        match value {
            Param::Bool(value) => Some(i64::from(*value)),
            Param::Number(value) => value.as_i64().or_else(|| {
                value
                    .as_f64()
                    .filter(|v| *v >= i64::MIN as f64 && *v < -(i64::MIN as f64))
                    .map(|v| v.trunc() as i64)
            }),
            Param::Str(value) => {
                let value = value.trim_start_matches([' ', '\t', '\n', '\u{b}', '\u{c}', '\r']);
                let (negative, value) = if let Some(value) = value.strip_prefix('-') {
                    (true, value)
                } else {
                    (false, value.strip_prefix('+').unwrap_or(value))
                };
                if !value.as_bytes().first().is_some_and(u8::is_ascii_digit) {
                    return None;
                }
                let value = value
                    .strip_prefix("0d")
                    .or_else(|| value.strip_prefix("0D"))
                    .unwrap_or(value);
                let mut number = 0_i128;
                let mut previous_digit = false;
                for c in value.chars() {
                    match c {
                        '0'..='9' => {
                            number = number
                                .checked_mul(10)?
                                .checked_add(i128::from(c as u8 - b'0'))?;
                            previous_digit = true;
                        }
                        '_' if previous_digit => previous_digit = false,
                        _ => break,
                    }
                }
                i64::try_from(if negative { -number } else { number }).ok()
            }
            _ => None,
        }
    }
    match value {
        Some(Param::Array(values)) => {
            let values = values
                .iter()
                .filter(|value| !value.is_null())
                .collect::<Vec<_>>();
            if values.len() == 1 {
                finder_ids(Some(values[0]))
            } else {
                values.into_iter().filter_map(integer).collect()
            }
        }
        Some(value) => integer(value).into_iter().collect(),
        None => Vec::new(),
    }
}
fn find_id(
    conn: &rusqlite::Connection,
    table: &'static str,
    ids: &[i64],
) -> campfire_db::Result<Option<i64>> {
    // These table names are controller constants, never request values.
    Ok(conn.query_row(
        &format!(
            "SELECT id FROM {table} WHERE id IN (SELECT value FROM json_each(?)) ORDER BY id LIMIT 1"
        ),
        [json!(ids).to_string()],
        |row| row.get(0),
    ).optional()?)
}

pub async fn index(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let state = scalar(c, "state");
    let state = if ["all", "done", "agents", "boards"].contains(&state.as_str()) {
        state
    } else {
        "open".into()
    };
    let viewer = require_current_user(c)?.clone();
    let filter = state.clone();
    let threads = c
        .app()
        .db
        .read(move |conn| ChannelThread::visible_work_threads(conn, &viewer, &filter))
        .await
        .map_err(db_error)?;
    c.no_store(); c.set_header("pragma", "no-cache");
    let viewer = require_current_user(c)?.clone();
    if *c.respond_to(&[&format::HTML, &format::JSON])? == format::JSON {
        let base = c.url_for("");
        let payload = messages::present(c, move |p| {
            Ok(json!({"threads": messages::payload::work_threads(p, &threads, &viewer, &base)?}))
        }).await?;
        return super::channel_threads::render_json(c, StatusCode::OK, &payload);
    }
    campfire_runtime::navigation::redirect(c).await
}


/// The controller's ordered before-actions: thread, human room access, tracking, sender rights.
pub(super) async fn scope(c: &mut Ctx, manager: bool) -> Result<(Room, ChannelThread)> {
    let ids = finder_ids(c.params.get("thread_id"));
    let viewer = require_current_user(c)?.clone();
    let facts = c
        .app()
        .db
        .read(move |conn| {
            let Some(id) = find_id(conn, "channel_threads", &ids)? else {
                return Ok(None);
            };
            let Some(thread) = ChannelThread::find_by_id(conn, id)? else {
                return Ok(None);
            };
            if !thread.work_viewable_by(conn, &viewer)? {
                return Ok(None);
            }
            let tracked = thread.work();
            let manageable = thread.work_manageable_by(conn, &viewer)?;
            Ok(Some((thread.room(conn)?, thread, tracked, manageable)))
        })
        .await
        .map_err(db_error)?;
    let Some((room, thread, tracked, manageable)) = facts else {
        return halt(head(StatusCode::NOT_FOUND));
    };
    if !tracked {
        return halt(head(StatusCode::UNPROCESSABLE_ENTITY));
    }
    if manager && !manageable {
        return halt(head(StatusCode::FORBIDDEN));
    }
    Ok((room, thread))
}
pub async fn create_handoff(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let (_, mut thread) = scope(c, true).await?;
    let original = thread.clone();
    let sender = require_current_user(c)?.clone();
    let receiver_ids = finder_ids(c.params.get("receiver_agent_id"));
    let context = super::two_factor::audit_context(c)?;
    let package = HandoffPackage {
        summary: scalar(c, "summary"),
        links: list(c, "links"),
        open_questions: list(c, "open_questions"),
    };
    let result = c
        .app()
        .db
        .write(move |tx| {
            let receiver = find_id(tx.conn(), "agents", &receiver_ids)?
                .map(|id| Agent::find(tx.conn(), id))
                .transpose()?
                .flatten();
            if let Some(error) = WorkHandoff::receiver_error(tx.conn(), &thread, receiver.as_ref())?
            {
                return Ok(Err(error.to_string()));
            }
            let receiver = receiver.expect("receiver policy passed");
            match thread.hand_off(tx, &sender, &receiver, package, &context) {
                Ok(handoff) => Ok(Ok((
                    thread,
                    handoff,
                    User::find(tx.conn(), receiver.user_id)?.name,
                ))),
                Err(campfire_db::Error::RecordInvalid(errors)) => Ok(Err(
                    campfire_presentation::helpers::to_sentence(&errors.full_messages(), " and "),
                )),
                Err(error) => Err(error),
            }
        })
        .await;
    let (thread, handoff, receiver) = match result {
        Ok(Ok(result)) => result,
        Ok(Err(error)) => return handoff_invalid(c, original, error).await,
        Err(campfire_db::Error::Other(error))
            if error == campfire_db::models::channel_thread::WORK_UPDATE_FORBIDDEN =>
        {
            return Ok(c.head(StatusCode::FORBIDDEN));
        }
        Err(error) => return Err(db_error(error)),
    };
    if *c.respond_to(&[&format::HTML, &format::JSON])? == format::JSON {
        let payload = messages::present(c, move |p| {
            let fresh = ChannelThread::find(p.conn, thread.id)?;
            let mut payload = campfire_db::models::agent_payloads::work_payload(
                p.conn,
                &fresh,
                None,
                &Default::default(),
            )?;
            payload["handoff"] = handoff.payload(p.conn)?;
            Ok(payload)
        })
        .await?;
        return super::channel_threads::render_json(c, StatusCode::CREATED, &payload);
    }
    c.flash()
        .set_notice(format!("Work handed off to {receiver}."));
    c.redirect_to(&format!("/rooms/{}/threads/{}", thread.room_id, thread.id))
}
async fn handoff_invalid(c: &mut Ctx, _thread: ChannelThread, error: String) -> Result {
    if *c.respond_to(&[&format::HTML, &format::JSON])? == format::JSON {
        super::channel_threads::render_json(
            c,
            StatusCode::UNPROCESSABLE_ENTITY,
            &json!({"error":error}),
        )
    } else {
        Ok(c.head(StatusCode::UNPROCESSABLE_ENTITY))
    }
}
fn scalar(c: &Ctx, key: &str) -> String {
    c.params
        .get(key)
        .map(|value| campfire_richtext::ruby::json_value_to_s(&value.to_json()))
        .unwrap_or_default()
}
fn list(c: &Ctx, key: &str) -> Value {
    match c.params.get(key) {
        None => Value::Null,
        Some(value) if value.is_blank() => Value::Null,
        Some(Param::Array(items)) => json!(items.iter().map(Param::to_json).collect::<Vec<_>>()),
        Some(value) => json!(
            campfire_richtext::ruby::json_value_to_s(&value.to_json())
                .split(['\r', '\n'])
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
        ),
    }
}
