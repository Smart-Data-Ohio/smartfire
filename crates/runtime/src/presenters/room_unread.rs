//! Unread timeline facts shared by the SPA API.
use campfire_db::{CachedStatements, Connection, Membership, Message, Result};

/// `Membership#first_unread_message` and the count of root messages from it to the newest,
/// inclusive; `None` when the room is read (or nothing follows the read position).
pub fn first_unread(conn: &Connection, membership: &Membership) -> Result<Option<(i64, i64)>> {
    if !membership.unread() {
        return Ok(None);
    }
    let mut query = String::from(
        "SELECT id,created_at FROM messages WHERE room_id=? AND thread_id IS NULL AND ",
    );
    let boundary = if let Some(id) = membership.last_read_message_id {
        if let Some(reference) = Message::find_by_id(conn, id)?
            .filter(|m| m.room_id == membership.room_id && m.thread_id.is_none())
        {
            query.push_str("(created_at,id) > (?,?) ORDER BY created_at,id LIMIT 1");
            conn.prepare_cached(&query)?
                .query_map(
                    rusqlite::params![membership.room_id, reference.created_at, id],
                    |r| Ok((r.get::<_, i64>(0)?, r.get::<_, campfire_db::Timestamp>(1)?)),
                )?
                .next()
                .transpose()?
        } else if let Some(stamp) = membership.unread_at {
            query.push_str("(created_at,id) > (?,?) ORDER BY created_at,id LIMIT 1");
            conn.prepare_cached(&query)?
                .query_map(rusqlite::params![membership.room_id, stamp, id], |r| {
                    Ok((r.get::<_, i64>(0)?, r.get::<_, campfire_db::Timestamp>(1)?))
                })?
                .next()
                .transpose()?
        } else {
            None
        }
    } else if let Some(stamp) = membership.unread_at {
        query.push_str("created_at >= ? ORDER BY created_at,id LIMIT 1");
        conn.prepare_cached(&query)?
            .query_map(rusqlite::params![membership.room_id, stamp], |r| {
                Ok((r.get::<_, i64>(0)?, r.get::<_, campfire_db::Timestamp>(1)?))
            })?
            .next()
            .transpose()?
    } else {
        None
    };
    let Some((id, created_at)) = boundary else {
        return Ok(None);
    };
    let count=conn.query_row_cached("SELECT COUNT(*) FROM messages WHERE room_id=? AND thread_id IS NULL AND (created_at,id) >= (?,?)",rusqlite::params![membership.room_id,created_at,id],|r|r.get::<_,i64>(0))?;
    Ok(Some((id, count)))
}
