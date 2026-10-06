//! The root-page and unread facts consumed by the shell and WS8b-m's list adapter.
use campfire_db::CachedStatements;
use campfire_db::{Connection, Membership, Message, Result, Timeline, Timestamp};
use campfire_views::rooms::shell::Notice;

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
    let Some((id, count)) = first_unread(conn, membership)? else {
        return Ok(UnreadDivider::default());
    };
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

pub type NoticeFields = (
    i64,
    String,
    Option<Timestamp>,
    Option<String>,
    String,
    Option<String>,
    bool,
    Option<String>,
);
pub fn notice_from_fields(row: NoticeFields, now: Timestamp) -> Notice {
    let (id, name, manual, note, presence, zone, calendar, intervals) = row;
    let zone = campfire_views::time::Zone::for_user(zone.as_deref());
    let manual = manual.filter(|at| *at > now);
    let calendar = if calendar {
        super::runtime_chrome::epochs(
            intervals.as_deref().unwrap_or("[]"),
            &zone,
        )
        .into_iter()
        .filter(|(start, end)| *start <= now.as_second() && now.as_second() < *end)
        .map(|(_, end)| Timestamp::from_second(end))
        .max()
    } else {
        None
    };
    let until = manual
        .into_iter()
        .chain(calendar)
        .max()
        .filter(|_| presence != "invisible");
    Notice {
        id,
        name,
        until_date: until.map(|at| zone.format(at.jiff(), "%B %d, %Y")),
        note: if until.is_some() && manual.is_some() {
            note.filter(|s| !campfire_richtext::ruby::is_blank(s))
        } else {
            None
        },
    }
}
pub fn notice(
    conn: &Connection,
    user_id: i64,
    now: Timestamp,
) -> campfire_db::Result<Notice> {
    let row=conn.query_row_cached("SELECT users.id,users.name,users.ooo_until,users.ooo_note,users.presence_setting,users.time_zone,users.ooo_calendar_enabled,calendar_meeting_caches.ooo_intervals FROM users LEFT JOIN calendar_meeting_caches ON calendar_meeting_caches.user_id=users.id WHERE users.id=?",[user_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?,r.get(7)?)))?;
    Ok(notice_from_fields(row, now))
}
