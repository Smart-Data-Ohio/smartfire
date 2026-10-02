//! Bounded, authorized agent reader windows. REST and MCP use the same reads.
use crate::sql::{placeholders, query_all, query_one};
use crate::{ChannelThread, Connection, Message, Result};
use rusqlite::{params, params_from_iter};

pub fn message_by_ids(conn: &Connection, ids: &[i64]) -> Result<Option<Message>> {
    if ids.is_empty() {
        return Ok(None);
    }
    query_one(
        conn,
        &format!(
            "SELECT * FROM messages WHERE id IN ({}) LIMIT 1",
            placeholders(ids.len())
        ),
        params_from_iter(ids),
        Message::from_row,
    )
}
pub fn thread_by_ids(conn: &Connection, ids: &[i64]) -> Result<Option<ChannelThread>> {
    if ids.is_empty() {
        return Ok(None);
    }
    query_one(
        conn,
        &format!(
            "SELECT * FROM channel_threads WHERE id IN ({}) LIMIT 1",
            placeholders(ids.len())
        ),
        params_from_iter(ids),
        ChannelThread::from_row,
    )
}
pub fn message_window(
    conn: &Connection,
    room: i64,
    thread: Option<i64>,
    before: Option<i64>,
    after: Option<i64>,
    inclusive: bool,
    limit: i64,
) -> Result<Vec<Message>> {
    let comparison = if inclusive { "<=" } else { "<" };
    let mut records = query_all(
        conn,
        &format!(
            "SELECT * FROM messages WHERE room_id=? AND ((? IS NULL AND thread_id IS NULL) OR thread_id=?) AND (? IS NULL OR id{comparison}?) AND (? IS NULL OR id>?) ORDER BY id DESC LIMIT ?"
        ),
        params![room, thread, thread, before, before, after, after, limit],
        Message::from_row,
    )?;
    records.reverse();
    Ok(records)
}
pub fn board_posts(
    conn: &Connection,
    room: i64,
    user: i64,
    status: &str,
    owner: &str,
    tag: &str,
) -> Result<Vec<ChannelThread>> {
    let numeric_owner = super::agent_delivery::ruby_i64(&serde_json::json!(owner));
    query_all(
        conn,
        "SELECT * FROM channel_threads WHERE room_id=? AND (?='all' OR (?='open' AND work_status IS NOT NULL AND work_status!='done') OR (? NOT IN ('all','open') AND work_status=?)) AND (?='' OR (?='me' AND work_owner_id=?) OR (?='agents' AND work_owner_id IN (SELECT user_id FROM agents)) OR (? NOT IN ('','me','agents') AND work_owner_id=?)) AND (?='' OR EXISTS(SELECT 1 FROM thread_tags WHERE channel_thread_id=channel_threads.id AND name=?)) ORDER BY last_activity_at DESC,id DESC LIMIT 100",
        params![
            room,
            status,
            status,
            status,
            status,
            owner,
            owner,
            user,
            owner,
            owner,
            numeric_owner,
            tag,
            tag
        ],
        ChannelThread::from_row,
    )
}
pub fn owned_work(conn: &Connection, user: i64, rooms: &[i64]) -> Result<Vec<ChannelThread>> {
    if rooms.is_empty() {
        return Ok(vec![]);
    }
    let mut values = vec![user];
    values.extend_from_slice(rooms);
    query_all(
        conn,
        &format!(
            "SELECT * FROM channel_threads WHERE work_status IS NOT NULL AND work_owner_id=? AND room_id IN ({}) ORDER BY updated_at DESC,id DESC LIMIT 100",
            placeholders(rooms.len())
        ),
        params_from_iter(values),
        ChannelThread::from_row,
    )
}
