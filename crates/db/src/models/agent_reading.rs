//! Bounded, authorized agent reader windows. REST and MCP use the same reads.
use crate::sql::{placeholders, query_all, query_one};
use crate::{ChannelThread, Connection, Message, Result, Room};
use rusqlite::{params, params_from_iter};

pub fn message_by_ids(conn: &Connection, ids: &[i64]) -> Result<Option<Message>> {
    if ids.is_empty() {
        return Ok(None);
    }
    query_one(conn,"SELECT * FROM messages WHERE id IN (SELECT value FROM json_each(?)) ORDER BY id LIMIT 1",
        [serde_json::json!(ids).to_string()],Message::from_row)

}
pub fn thread_by_ids(conn: &Connection, ids: &[i64]) -> Result<Option<ChannelThread>> {
    if ids.is_empty() {
        return Ok(None);
    }
    query_one(conn,"SELECT * FROM channel_threads WHERE id IN (SELECT value FROM json_each(?)) ORDER BY id LIMIT 1",
        [serde_json::json!(ids).to_string()],ChannelThread::from_row)

}
/// Work-create transport's current alive membership lookup, matching user.rooms.
/// Its caller rechecks live capabilities in the same writer transaction.
pub fn member_room_by_ids(conn:&Connection,user:i64,ids:&[i64])->Result<Option<Room>> {
    query_one(conn,"SELECT r.* FROM rooms r JOIN memberships m ON m.room_id=r.id
        WHERE m.user_id=? AND r.deleted_at IS NULL AND r.id IN (SELECT value FROM json_each(?)) ORDER BY r.id LIMIT 1",
        params![user,serde_json::json!(ids).to_string()],Room::from_row)
}

/// Rails resolves cursor arrays inside the original conversation, before applying
/// either window. One JSON bind keeps arbitrary IN lists below SQLite's bind limit.
pub fn conversation_anchor(conn:&Connection,room:i64,thread:Option<i64>,ids:&[i64])->Result<Option<i64>> {
    query_one(conn,"SELECT id FROM messages WHERE id IN (SELECT value FROM json_each(?)) AND ((? IS NOT NULL AND thread_id=?) OR (? IS NULL AND room_id=? AND thread_id IS NULL)) ORDER BY id LIMIT 1",
        params![serde_json::json!(ids).to_string(),thread,thread,thread,room],|row|row.get(0))
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
    let names = if tag.is_empty() { Vec::new() } else { crate::ThreadTag::matching_names(conn, room, tag)? };
    query_all(
        conn,
        &format!("SELECT * FROM channel_threads WHERE room_id=? AND {thread_access} AND (?='all' OR (?='open' AND work_status IS NOT NULL AND work_status!='done') OR (? NOT IN ('all','open') AND work_status=?)) AND (?='' OR (?='me' AND work_owner_id=?) OR (?='agents' AND work_owner_id IN (SELECT user_id FROM agents)) OR (? NOT IN ('','me','agents') AND work_owner_id=?)) AND (?='' OR EXISTS(SELECT 1 FROM thread_tags WHERE channel_thread_id=channel_threads.id AND name IN (SELECT value FROM json_each(?)))) ORDER BY last_activity_at DESC,id DESC LIMIT 100", thread_access = super::channel_thread::readable_by_sql("channel_threads.id", &user.to_string())),
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
            serde_json::json!(names).to_string()
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
            "SELECT * FROM channel_threads WHERE work_status IS NOT NULL AND work_owner_id=? AND room_id IN ({}) AND {} ORDER BY updated_at DESC,id DESC LIMIT 100",
            placeholders(rooms.len()), super::channel_thread::readable_by_sql("channel_threads.id", &user.to_string())
        ),
        params_from_iter(values),
        ChannelThread::from_row,
    )
}

/// Recheck the selected rows in the snapshot that renders their response.
pub fn visible_work_by_ids(
    conn: &Connection,
    agent: &crate::Agent,
    ids: &[i64],
    owned: bool,
) -> Result<Vec<ChannelThread>> {
    let thread = super::channel_thread::readable_by_sql("current_thread.id", "?3");
    let cap =
        super::agent_access::capability_sql("?2", "current_thread.room_id", "'read_messages'");
    query_all(
        conn,
        &format!(
            "SELECT current_thread.* FROM json_each(?1) requested JOIN channel_threads current_thread ON current_thread.id=requested.value WHERE EXISTS (SELECT 1 FROM memberships WHERE room_id=current_thread.room_id AND user_id=?3) AND ({thread}) AND ({cap}) AND (?4=0 OR (current_thread.work_status IS NOT NULL AND current_thread.work_owner_id=?3)) ORDER BY CAST(requested.key AS INTEGER)"
        ),
        params![
            serde_json::json!(ids).to_string(),
            agent.id,
            agent.user_id,
            owned
        ],
        ChannelThread::from_row,
    )
}

pub fn work_response(
    conn: &Connection,
    agent_id: i64,
    thread_id: i64,
    creating: bool,
) -> Result<ChannelThread> {
    let readable =
        super::channel_thread::readable_by_sql("response_thread.id", "response_agent.user_id");
    let primary = super::agent_access::capability_sql(
        "response_agent.id",
        "response_thread.room_id",
        "CASE WHEN ?3 THEN 'post_messages' ELSE 'read_messages' END",
    );
    let manage = super::agent_access::capability_sql(
        "response_agent.id",
        "response_thread.room_id",
        "'manage_threads'",
    );
    query_one(conn,&format!("SELECT response_thread.* FROM channel_threads response_thread JOIN agents response_agent ON response_agent.id=?1 WHERE response_thread.id=?2 AND (response_agent.kind!='personal' OR (({readable}) AND ({primary}) AND ({manage})))"),params![agent_id,thread_id,creating],ChannelThread::from_row)?.ok_or(crate::Error::RecordNotFound("Work"))
}
