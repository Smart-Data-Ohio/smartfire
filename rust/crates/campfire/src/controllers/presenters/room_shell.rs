//! The root-page and unread facts consumed by the shell and WS8b-m's list adapter.
use campfire_db::CachedStatements;
use campfire_db::{Connection, Membership, Message, Result, Timeline};

pub fn find_messages(
    conn: &Connection,
    room_id: i64,
    message_id: Option<i64>,
) -> Result<Vec<Message>> {
    match message_id
        .map(|id| Message::find_by_id(conn, id))
        .transpose()?
        .flatten()
        .filter(|m| m.room_id == room_id && m.thread_id.is_none())
    {
        Some(message) => Message::page_around(conn, Timeline::Room(room_id), &message),
        None => Message::last_page(conn, Timeline::Room(room_id)),
    }
}
#[derive(Default, Debug, PartialEq)]
pub struct UnreadDivider {
    pub message_id: Option<i64>,
    pub count: i64,
    pub scroll: Option<bool>,
    pub jump_url: Option<String>,
}
/// Rails Membership#first_unread_message and RoomsController#set_unread_divider.
/// These read-only queries are a presenter seam; no membership is written or marked read.
pub fn unread_divider(
    conn: &Connection,
    membership: &Membership,
    messages: &[Message],
) -> Result<UnreadDivider> {
    if !membership.unread() {
        return Ok(UnreadDivider::default());
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
        return Ok(UnreadDivider::default());
    };
    let count=conn.query_row_cached("SELECT COUNT(*) FROM messages WHERE room_id=? AND thread_id IS NULL AND (created_at,id) >= (?,?)",rusqlite::params![membership.room_id,created_at,id],|r|r.get::<_,i64>(0))?;
    Ok(if messages.iter().any(|m| m.id == id) {
        UnreadDivider {
            message_id: Some(id),
            count,
            scroll: (count > 5).then_some(true),
            jump_url: None,
        }
    } else {
        UnreadDivider {
            message_id: None,
            count,
            scroll: None,
            jump_url: Some(format!("/rooms/{}?message_id={}", membership.room_id, id)),
        }
    })
}
