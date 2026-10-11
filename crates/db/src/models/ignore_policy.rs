//! Personal visibility and one-way contact prevention. People remain discoverable.
//!
//! I1: Whether a sequenced publication happens, and how many happen, depends only on
//! unfiltered state. Ignore filtering changes a blocker's payload, never its publication slot.
//! I2: Filter stored per-recipient state at write time where possible. An ignored author's
//! event must not create, replace or bump an activity item the blocker sees.
use crate::sql::{exists, query_all};
use crate::{Connection, Error, Result};
use rusqlite::params;
use std::collections::{BTreeSet, HashMap};

pub fn ignored(conn: &Connection, viewer: i64, author: i64) -> Result<bool> {
    exists(
        conn,
        "SELECT 1 FROM user_blocks WHERE blocker_id=? AND blocked_id=?",
        params![viewer, author],
    )
}

pub fn ignored_authors(conn: &Connection, viewer: i64) -> Result<BTreeSet<i64>> {
    Ok(query_all(
        conn,
        "SELECT blocked_id FROM user_blocks WHERE blocker_id=?",
        [viewer],
        |row| row.get(0),
    )?
    .into_iter()
    .collect())
}

pub fn blockers_of(conn: &Connection, author: i64) -> Result<Vec<i64>> {
    query_all(
        conn,
        "SELECT blocker_id FROM user_blocks WHERE blocked_id=?",
        [author],
        |row| row.get(0),
    )
}

pub fn visible_recipients(conn: &Connection, author: i64, recipients: Vec<i64>) -> Result<Vec<i64>> {
    let blockers = blockers_of(conn, author)?.into_iter().collect::<BTreeSet<_>>();
    Ok(recipients.into_iter().filter(|id| !blockers.contains(id)).collect())
}

pub fn blockers_for_authors(
    conn: &Connection,
    authors: impl IntoIterator<Item = i64>,
) -> Result<HashMap<i64, Vec<i64>>> {
    let authors: BTreeSet<_> = authors.into_iter().collect();
    let mut blockers: HashMap<i64, Vec<i64>> = HashMap::new();
    if authors.is_empty() {
        return Ok(blockers);
    }
    let rows = query_all(
        conn,
        "SELECT blocked_id,blocker_id FROM user_blocks WHERE blocked_id IN (SELECT value FROM json_each(?))",
        [serde_json::json!(authors).to_string()],
        |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
    )?;
    for (author, viewer) in rows {
        blockers.entry(author).or_default().push(viewer);
    }
    Ok(blockers)
}

/// SQL expressions here are owned by callers, never request text.
pub fn visible_sql(viewer: &str, author: &str) -> String {
    format!(
        "NOT EXISTS (SELECT 1 FROM user_blocks ignored_person WHERE ignored_person.blocker_id={viewer} AND ignored_person.blocked_id={author})"
    )
}

pub fn thread_unread_sql() -> String {
    let visible = visible_sql("thread_memberships.user_id", "unread_message.creator_id");
    format!(
        "thread_memberships.unread_at IS NOT NULL AND (NOT EXISTS (SELECT 1 FROM user_blocks WHERE blocker_id=thread_memberships.user_id) OR EXISTS (SELECT 1 FROM messages unread_message WHERE unread_message.thread_id=thread_memberships.thread_id AND NOT unread_message.system_note AND unread_message.creator_id!=thread_memberships.user_id AND (thread_memberships.last_read_message_id IS NULL OR unread_message.id>thread_memberships.last_read_message_id) AND {visible}))"
    )
}

pub fn room_unread_sql() -> String {
    let visible = visible_sql("memberships.user_id", "unread_message.creator_id");
    let visible_post = visible_sql("memberships.user_id", "unread_post.creator_id");
    format!(
        "memberships.unread_at IS NOT NULL AND (NOT EXISTS (SELECT 1 FROM user_blocks WHERE blocker_id=memberships.user_id) OR EXISTS (SELECT 1 FROM messages unread_message LEFT JOIN messages read_message ON read_message.id=memberships.last_read_message_id AND read_message.room_id=memberships.room_id WHERE unread_message.room_id=memberships.room_id AND (unread_message.thread_id IS NULL OR EXISTS (SELECT 1 FROM rooms WHERE rooms.id=memberships.room_id AND rooms.type='Rooms::Board')) AND NOT unread_message.system_note AND unread_message.creator_id!=memberships.user_id AND CASE WHEN memberships.last_read_message_id IS NULL THEN unread_message.created_at>=memberships.unread_at ELSE (unread_message.created_at,unread_message.id)>(COALESCE(read_message.created_at,memberships.unread_at),memberships.last_read_message_id) END AND {visible}) OR EXISTS (SELECT 1 FROM channel_threads unread_post JOIN rooms ON rooms.id=unread_post.room_id WHERE unread_post.room_id=memberships.room_id AND rooms.type='Rooms::Board' AND unread_post.created_at>=memberships.unread_at AND unread_post.creator_id!=memberships.user_id AND {visible_post}))"
    )
}

/// The same unavailable-person error for every refused contact. Existing DMs bypass creation.
pub fn require_contact(conn: &Connection, sender: i64, recipients: &[i64]) -> Result<()> {
    for recipient in recipients {
        if ignored(conn, *recipient, sender)? {
            return Err(Error::RecordNotFound("User"));
        }
    }
    Ok(())
}
