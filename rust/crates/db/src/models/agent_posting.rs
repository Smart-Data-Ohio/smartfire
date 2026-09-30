//! Shared posting preflight from Agents::Posting and Agents::Budgets. Upload staging and
//! rendering stay in the app; idempotency, usage and the budget notice are domain policy.
pub mod client_ids;

use jiff::tz::TimeZone;
use rusqlite::params;
use serde_json::{Value, json};

use crate::slash_commands::time_parser;
use crate::sql::{CachedStatements, query_all, query_one};
use crate::{ActivityItem, Connection, Error, Message, NewMessage, Result, Timestamp, Tx, User};

#[derive(Debug)]
pub enum PostingOutcome {
    Created(Message),
    Replay(Message),
    Budget(Value),
}

#[derive(Debug)]
pub enum PostingCheck {
    Allowed,
    Replay(Box<Message>),
    Budget(Value),
}

#[derive(Clone, Copy)]
pub enum Cap {
    Messages,
    BoardPosts,
    ExternalActions,
}
impl Cap {
    fn name(self) -> &'static str {
        match self {
            Self::Messages => "messages",
            Self::BoardPosts => "board_posts",
            Self::ExternalActions => "external_actions",
        }
    }
    fn noun(self) -> &'static str {
        match self {
            Self::Messages => "message",
            Self::BoardPosts => "board post",
            Self::ExternalActions => "external action",
        }
    }
    fn column(self) -> &'static str {
        match self {
            Self::Messages => "daily_message_cap",
            Self::BoardPosts => "daily_board_post_cap",
            Self::ExternalActions => "daily_external_action_cap",
        }
    }
}

pub struct DailyWindow {
    pub day: String,
    pub start: Timestamp,
    pub end: Timestamp,
    pub retry_after: i64,
}

pub fn daily_window(now: Timestamp, zone: &TimeZone) -> Result<DailyWindow> {
    let date = now.jiff().to_zoned(zone.clone()).date();
    let start = time_parser::local(date, 0, 0, 0, 0, zone)
        .ok_or_else(|| Error::Other("Invalid beginning of day".into()))?;
    let end = time_parser::end_of_day(now, zone)
        .ok_or_else(|| Error::Other("Invalid end of day".into()))?;
    let retry_after = ((end.as_microsecond() - now.as_microsecond()) / 1_000_000).max(1);
    Ok(DailyWindow {
        day: date.to_string(),
        start,
        end,
        retry_after,
    })
}

/// Like Time.zone in an authenticated agent request: the user's recognized zone, else UTC.
fn zone(conn: &Connection, user_id: i64) -> Result<TimeZone> {
    let name: Option<String> =
        conn.query_row_cached("SELECT time_zone FROM users WHERE id=?", [user_id], |r| {
            r.get(0)
        })?;
    Ok(time_parser::zone(name.as_deref().unwrap_or("UTC")))
}

/// None identifies a legacy bot, whose raw-body endpoint ignores client_message_id.
pub fn prepare_for_user(
    tx: &mut Tx<'_>,
    user_id: i64,
    room_id: i64,
    client_message_id: Option<&str>,
) -> Result<Option<PostingCheck>> {
    let agent_id = query_one(
        tx.conn(),
        "SELECT id FROM agents WHERE user_id=? LIMIT 1",
        [user_id],
        |r| r.get::<_, i64>(0),
    )?;
    agent_id
        .map(|agent_id| prepare(tx, agent_id, room_id, client_message_id))
        .transpose()
}

/// Explicit request lookup preserves AR's IN/blank binding policy separately from the
/// string that is assigned to the saved row. Legacy bots ignore the client id entirely.
pub fn prepare_for_user_with_lookup(
    tx: &mut Tx<'_>,
    user_id: i64,
    room_id: i64,
    stored: Option<&str>,
    lookup: &client_ids::Lookup,
) -> Result<Option<PostingCheck>> {
    let Some(agent_id) = query_one(
        tx.conn(),
        "SELECT id FROM agents WHERE user_id=? LIMIT 1",
        [user_id],
        |r| r.get::<_, i64>(0),
    )?
    else {
        return Ok(None);
    };
    match lookup {
        client_ids::Lookup::Attribute => return prepare(tx, agent_id, room_id, stored).map(Some),
        client_ids::Lookup::InvalidParameters => {
            return Err(Error::Other(
                "can't cast ActionController::Parameters".into(),
            ));
        }
        client_ids::Lookup::Values(values) => {
            if !values.is_empty() {
                let mut binds = vec![
                    rusqlite::types::Value::Integer(room_id),
                    rusqlite::types::Value::Integer(user_id),
                ];
                binds.extend(values.iter().cloned().map(rusqlite::types::Value::Text));
                let sql = format!(
                    "SELECT * FROM messages WHERE room_id=? AND creator_id=? AND client_message_id IN ({}) LIMIT 1",
                    crate::sql::placeholders(values.len())
                );
                if let Some(message) = query_one(
                    tx.conn(),
                    &sql,
                    rusqlite::params_from_iter(binds),
                    Message::from_row,
                )? {
                    return Ok(Some(PostingCheck::Replay(Box::new(message))));
                }
            }
        }
    }
    Ok(Some(
        if let Some(payload) = check_budget(tx, agent_id, Cap::Messages)? {
            PostingCheck::Budget(payload)
        } else {
            PostingCheck::Allowed
        },
    ))
}

/// Check an existing client id before budget. The caller saves in the same writer transaction.
pub fn prepare(
    tx: &mut Tx<'_>,
    agent_id: i64,
    room_id: i64,
    client_message_id: Option<&str>,
) -> Result<PostingCheck> {
    let user_id: i64 =
        tx.conn()
            .query_row_cached("SELECT user_id FROM agents WHERE id=?", [agent_id], |r| {
                r.get(0)
            })?;
    if let Some(id) = client_message_id.filter(|id| !campfire_richtext::ruby::is_blank(id))
        && let Some(message) = Message::find_duplicate(tx.conn(), room_id, user_id, id)?
    {
        return Ok(PostingCheck::Replay(Box::new(message)));
    }
    if let Some(payload) = check_budget(tx, agent_id, Cap::Messages)? {
        return Ok(PostingCheck::Budget(payload));
    }
    Ok(PostingCheck::Allowed)
}

/// Plain-message entry point for services that already hold canonicalized attributes.
/// The agent owns creator_id; a submitted attribute cannot impersonate another sender.
pub fn post(tx: &mut Tx<'_>, agent_id: i64, mut attributes: NewMessage) -> Result<PostingOutcome> {
    attributes.creator_id =
        tx.conn()
            .query_row_cached("SELECT user_id FROM agents WHERE id=?", [agent_id], |r| {
                r.get(0)
            })?;
    match prepare(
        tx,
        agent_id,
        attributes.room_id,
        attributes.client_message_id.as_deref(),
    )? {
        PostingCheck::Replay(message) => Ok(PostingOutcome::Replay(*message)),
        PostingCheck::Budget(payload) => Ok(PostingOutcome::Budget(payload)),
        PostingCheck::Allowed => Ok(PostingOutcome::Created(Message::create(tx, attributes)?)),
    }
}

/// Read-only Agents::Budgets.usage. Counts the same persisted rows as the
/// writer's cap check, without creating notices or activity items.
pub fn usage(conn: &Connection, agent_id: i64, now: Timestamp) -> Result<Value> {
    let user_id =
        conn.query_row_cached("SELECT user_id FROM agents WHERE id=?", [agent_id], |r| {
            r.get(0)
        })?;
    let window = daily_window(now, &zone(conn, user_id)?)?;
    Ok(json!({
        "messages": cap_usage(conn, agent_id, user_id, Cap::Messages, &window)?,
        "board_posts": cap_usage(conn, agent_id, user_id, Cap::BoardPosts, &window)?,
        "external_actions": cap_usage(conn, agent_id, user_id, Cap::ExternalActions, &window)?
    }))
}

// FLAGGED WS11 UI visibility seam: Rails page usage uses the viewer's Time.zone.
// Keep the owner counter callable with that window until usage accepts a request zone.
pub fn cap_usage(
    conn: &Connection,
    agent_id: i64,
    user_id: i64,
    cap: Cap,
    window: &DailyWindow,
) -> Result<i64> {
    let (sql, owner) = match cap {
        Cap::Messages => (
            "SELECT COUNT(*) FROM messages WHERE creator_id=? AND created_at BETWEEN ? AND ? AND board_post_opener=0",
            user_id,
        ),
        Cap::BoardPosts => (
            "SELECT COUNT(*) FROM channel_threads WHERE creator_id=? AND created_at BETWEEN ? AND ? AND room_id IN (SELECT id FROM rooms WHERE type='Rooms::Board')",
            user_id,
        ),
        Cap::ExternalActions => (
            "SELECT COUNT(*) FROM agent_approvals WHERE agent_id=? AND created_at BETWEEN ? AND ?",
            agent_id,
        ),
    };
    Ok(conn.query_row_cached(sql, params![owner, window.start, window.end], |r| r.get(0))?)
}

/// Agents::Budgets.check. Counting persisted rows and writing the notice are atomic with
/// the attempted post. Returning the denial (not a DB error) commits the notice on overflow.
pub fn check_budget(tx: &mut Tx<'_>, agent_id: i64, cap: Cap) -> Result<Option<Value>> {
    let (user_id, owner_id, limit): (i64, Option<i64>, Option<i64>) = tx.conn().query_row_cached(
        &format!(
            "SELECT user_id, owner_id, {} FROM agents WHERE id=?",
            cap.column()
        ),
        [agent_id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )?;
    let Some(limit) = limit else { return Ok(None) };
    let window = daily_window(tx.now(), &zone(tx.conn(), user_id)?)?;
    let usage = cap_usage(tx.conn(), agent_id, user_id, cap, &window)?;
    if usage < limit {
        return Ok(None);
    }
    let notice = query_one(
        tx.conn(),
        "INSERT INTO agent_budget_notices (agent_id,cap,day,created_at,updated_at) VALUES (?,?,?,?,?) ON CONFLICT(agent_id,cap,day) DO NOTHING RETURNING id",
        params![agent_id, cap.name(), window.day, tx.now(), tx.now()],
        |r| r.get::<_, i64>(0),
    )?;
    if let Some(notice_id) = notice {
        let recipients = if let Some(owner_id) = owner_id {
            vec![owner_id]
        } else {
            query_all(
                tx.conn(),
                "SELECT id FROM users WHERE status=0 AND role=1",
                [],
                |r| r.get::<_, i64>(0),
            )?
        };
        for user_id in recipients {
            if let Some(user) = User::find_by_id(tx.conn(), user_id)?
                && user.is_active()
                && !user.is_bot()
            {
                ActivityItem::refresh_unread(
                    tx,
                    user_id,
                    "AgentBudgetNotice",
                    notice_id,
                    "agent_budget_exceeded",
                )?;
            }
        }
    }
    Ok(Some(
        json!({"error":format!("Daily {} budget exceeded ({limit}/day)", cap.noun()), "cap":cap.name(), "limit":limit, "retry_after":window.retry_after}),
    ))
}

/// Preserve absent versus unusable Drive input; HTTP adapters own raw JSON casts.
#[derive(Debug, Clone, Default)]
pub enum DriveInput {
    #[default]
    Absent,
    Invalid,
    Ids(Vec<String>),
}
#[derive(Debug)]
pub enum PostResult {
    Posted(Box<Message>),
    Denied(super::agent_service::ServiceResult),
}

/// Agents::Posting. Caller supplies authenticated room membership and post grant.
/// This additive service keeps the existing canonical `post` signature intact.
pub fn post_service(
    tx: &mut Tx<'_>,
    agent_id: i64,
    mut a: NewMessage,
    drive: DriveInput,
) -> Result<PostResult> {
    use super::agent_service::{ServiceResult, invalid};
    let thread = if let Some(id) = a.thread_id {
        let Some(thread) =
            crate::ChannelThread::find_by_id(tx.conn(), id)?.filter(|t| t.room_id == a.room_id)
        else {
            return Ok(PostResult::Denied(ServiceResult::fail(
                "Thread not found",
                404,
            )));
        };
        if thread.locked_at.is_some() {
            return Ok(PostResult::Denied(ServiceResult::fail(
                "This thread is locked",
                422,
            )));
        }
        Some(thread)
    } else {
        if let Some(id) = a.reply_to_message_id {
            let target = Message::find_by_id(tx.conn(), id)?
                .filter(|m| m.room_id == a.room_id && m.thread_id.is_none());
            if target.is_none() {
                return Err(Error::RecordNotFound("Message"));
            }
        }
        None
    };
    match prepare(tx, agent_id, a.room_id, a.client_message_id.as_deref())? {
        PostingCheck::Replay(message) => return Ok(PostResult::Posted(message)),
        PostingCheck::Budget(payload) => {
            return Ok(PostResult::Denied(ServiceResult::budget(payload)));
        }
        PostingCheck::Allowed => {}
    }
    match drive {
        DriveInput::Absent => {}
        DriveInput::Ids(ids) if ids.iter().all(|id| super::message::valid_drive_file_id(id)) => {
            a.drive_file_ids = ids;
        }
        _ => {
            let mut errors = crate::Errors::default();
            errors.add("drive_attachments", "includes an invalid file id");
            return Ok(PostResult::Denied(invalid(errors)));
        }
    }
    if a.markdown_source.is_some() {
        a.body = None;
    }
    a.creator_id =
        tx.conn()
            .query_row("SELECT user_id FROM agents WHERE id=?", [agent_id], |r| {
                r.get(0)
            })?;
    let message = match tx.savepoint(|tx| {
        if let Some(mut thread) = thread {
            thread.post_message(tx, a.creator_id, a)
        } else {
            Message::create(tx, a)
        }
    }) {
        Ok(message) => message,
        Err(Error::RecordInvalid(errors)) => return Ok(PostResult::Denied(invalid(errors))),
        Err(error) => return Err(error),
    };
    broadcast_create(tx, &message)?;
    super::bot_webhook_fanout::deliver(tx, &message)?;
    // Attachment analysis/thumbnail processing is supplied by the storage/app owner.
    Ok(PostResult::Posted(Box::new(message)))
}

pub fn broadcast_stream_start(tx: &mut Tx<'_>, message: &Message) -> Result<()> {
    use crate::broadcasts::{Broadcast, Partial, conversation_messages, dom_id, room_dom_id};
    let room = crate::Room::find(tx.conn(), message.room_id)?;
    let target = message.thread_id.map_or_else(
        || room_dom_id(&room, Some("messages")),
        |id| dom_id("channel_thread", id, Some("messages")),
    );
    tx.emit_after_commit(crate::Event::broadcast(&Broadcast::append(
        conversation_messages(tx.conn(), message)?,
        target,
        Partial::Message {
            message_id: message.id,
        },
    )));
    Ok(())
}

pub fn broadcast_create(tx: &mut Tx<'_>, message: &Message) -> Result<()> {
    broadcast_stream_start(tx, message)?;
    if message.thread_id.is_none() && !message.system_note {
        broadcast_unread_room(tx, message)?;
    }
    Ok(())
}

pub fn broadcast_unread_room(tx: &mut Tx<'_>, message: &Message) -> Result<()> {
    use crate::broadcasts::Broadcast;
    let memberships = crate::Room::find(tx.conn(), message.room_id)?.memberships(tx.conn())?;
    let mentions = if memberships
        .iter()
        .any(|m| m.involvement == Some(crate::Involvement::Muted))
    {
        message
            .mentionees(tx.conn(), tx.rich_text())?
            .into_iter()
            .map(|u| u.id)
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    for m in memberships {
        if m.involvement != Some(crate::Involvement::Muted) || mentions.contains(&m.user_id) {
            tx.emit_after_commit(crate::Event::broadcast(&Broadcast::Cable {
                stream: format!("user_{}_unread_rooms", m.user_id),
                payload: json!({"roomId":message.room_id}),
            }));
        }
    }
    Ok(())
}
