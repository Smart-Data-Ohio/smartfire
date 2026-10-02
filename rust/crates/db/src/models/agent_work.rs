//! `app/services/agents/{board_posts,work_threads,work_handoffs}.rb`.
//! Typed payloads keep REST/MCP rendering in the app. A Denied outcome is a successful
//! database operation: callers commit it so Rails' budget notices survive the denial.
//! Board callers own room membership, read/post/manage grants, room type and throttling.
//! Work services own membership/read access, then manage_threads before any field validation.
use super::{agent_posting, agent_service::ServiceResult, audit_log};
use crate::models::channel_thread::{AgentWorkChanges, normalize_owner_id, tag_names_from_value};
use crate::sql::{query_all, query_one};
use crate::{
    Agent, ChannelThread, Error, HandoffPackage, Membership, NewChannelThread, Result, Room, Tx,
    User, WorkHandoff,
};
use campfire_richtext::ruby::{is_blank, strip};
use rusqlite::{Connection, params};
use serde_json::Value;

pub const LIST_MAX_LIMIT: usize = 100;
pub const LIST_STATUSES: [&str; 6] = ["planned", "in_progress", "blocked", "done", "open", "all"];

#[derive(Debug)]
pub enum Outcome<T> {
    Success { payload: T, status: u16 },
    Denied(ServiceResult),
}
impl<T> Outcome<T> {
    fn ok(payload: T, status: u16) -> Self {
        Self::Success { payload, status }
    }
    fn fail(message: &str, status: u16) -> Self {
        Self::Denied(ServiceResult::fail(message, status))
    }
}
#[derive(Debug, Clone, Default)]
pub struct BoardPostInput {
    pub title: Option<String>,
    pub body: Option<String>,
    pub tags: Option<Value>,
    pub work_status: Option<String>,
    pub run_url: Option<String>,
    pub owner_id: Option<Value>,
}
#[derive(Debug)]
pub struct HandedOffWork {
    pub thread: ChannelThread,
    pub handoff: WorkHandoff,
}

pub fn list_board_posts(
    conn: &Connection,
    agent: &Agent,
    room: &Room,
    status: Option<&str>,
    owner: Option<&str>,
    tag: Option<&str>,
) -> Result<Outcome<Vec<ChannelThread>>> {
    let status = strip(status.unwrap_or(""));
    let status = if is_blank(status) { "open" } else { status };
    if !LIST_STATUSES.contains(&status) {
        return Ok(Outcome::fail(
            "Status must be one of planned, in_progress, blocked, done, open, all",
            422,
        ));
    }
    let owner = strip(owner.unwrap_or(""));
    let numeric_owner = !owner.is_empty() && owner.bytes().all(|b| b.is_ascii_digit());
    if !is_blank(owner) && !["me", "agents"].contains(&owner) && !numeric_owner {
        return Ok(Outcome::fail("Owner must be a user id, me, or agents", 422));
    }
    use rusqlite::types::Value as Bind;
    let mut sql = "SELECT * FROM channel_threads WHERE room_id=?".to_owned();
    let mut binds = vec![Bind::Integer(room.id)];
    match status {
        "all" => (),
        "open" => sql.push_str(" AND work_status IS NOT NULL AND work_status != 'done'"),
        _ => {
            sql.push_str(" AND work_status=?");
            binds.push(Bind::Text(status.into()));
        }
    }
    if owner == "agents" {
        sql.push_str(" AND work_owner_id IN (SELECT user_id FROM agents)");
    } else if owner == "me" || numeric_owner {
        sql.push_str(" AND work_owner_id=?");
        binds.push(if owner == "me" {
            Bind::Integer(agent.user_id)
        } else {
            owner
                .parse::<i64>()
                .map(Bind::Integer)
                .unwrap_or(Bind::Null)
        });
    }
    let tag = rails_compat::unicode::downcase(strip(tag.unwrap_or("")));
    if !is_blank(&tag) {
        sql.push_str(" AND id IN (SELECT channel_thread_id FROM thread_tags WHERE name=?)");
        binds.push(Bind::Text(tag));
    }
    sql.push_str(" ORDER BY last_activity_at DESC,id DESC LIMIT 100");
    Ok(Outcome::ok(
        query_all(
            conn,
            &sql,
            rusqlite::params_from_iter(binds),
            ChannelThread::from_row,
        )?,
        200,
    ))
}

pub fn create_board_post(
    tx: &mut Tx<'_>,
    agent: &Agent,
    room: &Room,
    input: BoardPostInput,
) -> Result<Outcome<ChannelThread>> {
    if let Some(payload) =
        agent_posting::check_budget(tx, agent.id, agent_posting::Cap::BoardPosts)?
    {
        return Ok(Outcome::Denied(ServiceResult::budget(payload)));
    }
    let operation = tx.savepoint(|tx| {
        let owner = input
            .owner_id
            .as_ref()
            .map(normalize_owner_id)
            .transpose()?
            .flatten()
            .unwrap_or(agent.user_id);
        ChannelThread::create_board_post(
            tx,
            NewChannelThread {
                room_id: room.id,
                creator_id: agent.user_id,
                name: input.title,
                work_status: Some(
                    input
                        .work_status
                        .filter(|s| !is_blank(s))
                        .unwrap_or_else(|| "in_progress".into()),
                ),
                work_owner_id: Some(owner),
                tag_names: input.tags.as_ref().map(tag_names_from_value),
                run_url: input.run_url,
                ..Default::default()
            },
            input.body,
        )
    });
    match operation {
        Ok(thread) => Ok(Outcome::ok(thread, 201)),
        Err(Error::RecordNotFound(_)) => {
            Ok(Outcome::fail("Agent is not a member of the room", 404))
        }
        Err(Error::RecordInvalid(errors)) => Ok(invalid(errors)),
        Err(error) => Err(error),
    }
}

pub fn list_work(conn: &Connection, agent: &Agent) -> Result<Outcome<Vec<ChannelThread>>> {
    let threads = query_all(
        conn,
        "SELECT * FROM channel_threads WHERE work_status IS NOT NULL AND work_owner_id=? AND room_id IN (SELECT room_id FROM memberships WHERE user_id=?) ORDER BY updated_at DESC,id DESC",
        params![agent.user_id, agent.user_id],
        ChannelThread::from_row,
    )?;
    let mut visible = Vec::new();
    for thread in threads {
        if agent.can(conn, "read_messages", Some(thread.room_id))? {
            visible.push(thread);
        }
        if visible.len() == LIST_MAX_LIMIT {
            break;
        }
    }
    Ok(Outcome::ok(visible, 200))
}
fn find_owned(conn: &Connection, agent: &Agent, id: i64) -> Result<Option<ChannelThread>> {
    let thread = query_one(
        conn,
        "SELECT * FROM channel_threads WHERE id=? AND work_status IS NOT NULL AND work_owner_id=?",
        params![id, agent.user_id],
        ChannelThread::from_row,
    )?;
    let Some(thread) = thread else {
        return Ok(None);
    };
    if Membership::find_by_room_and_user(conn, thread.room_id, agent.user_id)?.is_none()
        || !agent.can(conn, "read_messages", Some(thread.room_id))?
    {
        return Ok(None);
    }
    Ok(Some(thread))
}
pub fn show_work(conn: &Connection, agent: &Agent, id: i64) -> Result<Outcome<ChannelThread>> {
    Ok(match find_owned(conn, agent, id)? {
        Some(thread) => Outcome::ok(thread, 200),
        None => Outcome::fail("Work not found", 404),
    })
}
fn writable(conn: &Connection, agent: &Agent, id: i64) -> Result<Outcome<ChannelThread>> {
    let Some(thread) = find_owned(conn, agent, id)? else {
        return Ok(Outcome::fail("Work not found", 404));
    };
    if !agent.can(conn, "manage_threads", Some(thread.room_id))? {
        return Ok(Outcome::fail(
            "Forbidden: agent lacks manage_threads capability",
            403,
        ));
    }
    Ok(Outcome::ok(thread, 200))
}
pub fn update_work(
    tx: &mut Tx<'_>,
    agent: &Agent,
    id: i64,
    changes: AgentWorkChanges,
) -> Result<Outcome<ChannelThread>> {
    let mut thread = match writable(tx.conn(), agent, id)? {
        Outcome::Success { payload, .. } => payload,
        denial => return Ok(denial),
    };
    match thread.update_work_by_agent(tx, agent, changes) {
        Ok(()) => Ok(Outcome::ok(ChannelThread::find(tx.conn(), id)?, 200)),
        Err(Error::RecordInvalid(errors)) => Ok(invalid(errors)),
        Err(error) => Err(error),
    }
}
/// None omits the markdown key; Some(Null) is an explicit blank that clears the result.
pub fn set_result(
    tx: &mut Tx<'_>,
    agent: &Agent,
    id: i64,
    markdown: Option<&Value>,
) -> Result<Outcome<ChannelThread>> {
    let mut thread = match writable(tx.conn(), agent, id)? {
        Outcome::Success { payload, .. } => payload,
        denial => return Ok(denial),
    };
    let Some(markdown) = markdown else {
        return Ok(Outcome::fail("Markdown can't be blank", 422));
    };
    match thread.update_result_by_agent(tx, agent, markdown) {
        Ok(()) => Ok(Outcome::ok(ChannelThread::find(tx.conn(), id)?, 200)),
        Err(Error::RecordInvalid(errors)) => Ok(invalid(errors)),
        Err(error) => Err(error),
    }
}
pub fn handoff_work(
    tx: &mut Tx<'_>,
    agent: &Agent,
    id: i64,
    receiver_agent_id: i64,
    package: HandoffPackage,
    context: &audit_log::Context,
) -> Result<Outcome<HandedOffWork>> {
    let mut thread = match writable(tx.conn(), agent, id)? {
        Outcome::Success { payload, .. } => payload,
        Outcome::Denied(denial) => return Ok(Outcome::Denied(denial)),
    };
    let receiver = Agent::find(tx.conn(), receiver_agent_id)?;
    if let Some(error) = WorkHandoff::receiver_error(tx.conn(), &thread, receiver.as_ref())? {
        return Ok(Outcome::fail(error, 422));
    }
    let receiver = receiver.expect("receiver policy requires an Agent");
    let sender = User::find(tx.conn(), agent.user_id)?;
    // The service uses `links || []` / `open_questions || []` before the model's
    // Array(value) normalization. Ruby treats an explicit false as absent here.
    let mut package = package;
    if package.links == Value::Bool(false) {
        package.links = Value::Null;
    }
    if package.open_questions == Value::Bool(false) {
        package.open_questions = Value::Null;
    }
    match thread.hand_off(tx, &sender, &receiver, package, context) {
        Ok(handoff) => Ok(Outcome::ok(
            HandedOffWork {
                thread: ChannelThread::find(tx.conn(), id)?,
                handoff,
            },
            201,
        )),
        Err(Error::RecordNotFound(_)) => Ok(Outcome::fail("Work not found", 404)),
        Err(Error::RecordInvalid(errors)) => Ok(invalid(errors)),
        Err(error) => Err(error),
    }
}
fn invalid<T>(errors: crate::Errors) -> Outcome<T> {
    Outcome::Denied(ServiceResult::fail(
        crate::slash_commands::sentence(errors.full_messages()),
        422,
    ))
}
