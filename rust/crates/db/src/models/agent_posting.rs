//! Shared posting preflight from Agents::Posting and Agents::Budgets. Upload staging and
//! rendering stay in the app; idempotency, usage and the budget notice are domain policy.
use jiff::tz::TimeZone;
use rusqlite::params;
use serde_json::{Value, json};

use crate::slash_commands::time_parser;
use crate::sql::{CachedStatements, query_all, query_one};
use crate::{ActivityItem, Error, Message, NewMessage, Result, Timestamp, Tx, User};

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
fn zone(tx: &Tx<'_>, user_id: i64) -> Result<TimeZone> {
    let name: Option<String> =
        tx.conn()
            .query_row_cached("SELECT time_zone FROM users WHERE id=?", [user_id], |r| {
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
    let window = daily_window(tx.now(), &zone(tx, user_id)?)?;
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
    let usage: i64 =
        tx.conn()
            .query_row_cached(sql, params![owner, window.start, window.end], |r| r.get(0))?;
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
