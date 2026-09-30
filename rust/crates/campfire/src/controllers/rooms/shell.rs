//! Read-only request adapter. Personal notice sets never enter the fragment cache.
use campfire_db::{CachedStatements, Connection, Membership, Message, Room, Timestamp};
use campfire_views::rooms::shell::{Notice, State};
use rusqlite::OptionalExtension;

pub(super) fn load(
    conn: &Connection,
    room: &Room,
    user_id: i64,
    page: &[Message],
    now: Timestamp,
) -> campfire_db::Result<State> {
    let mut state = State::default();
    if let Some(membership) = Membership::find_by_room_and_user(conn, room.id, user_id)?
        && let Some(unread_at) = membership.unread_at
    {
        let reference = membership
            .last_read_message_id
            .map(|id| Message::find_by_id(conn, id))
            .transpose()?
            .flatten()
            .filter(|m| m.room_id == room.id && m.thread_id.is_none());
        let boundary = if let Some(reference) = reference {
            first(conn, room.id, ">", reference.created_at, Some(reference.id))?
        } else if let Some(id) = membership.last_read_message_id {
            first(conn, room.id, ">", unread_at, Some(id))?
        } else {
            first(conn, room.id, ">=", unread_at, None)?
        };
        if let Some(boundary) = boundary {
            state.unread_count=conn.query_row_cached("SELECT COUNT(*) FROM messages WHERE room_id=? AND thread_id IS NULL AND (created_at,id)>=(?,?)",rusqlite::params![room.id,boundary.created_at,boundary.id],|r|r.get(0))?;
            if let Some(index) = page.iter().position(|m| m.id == boundary.id) {
                state.unread_index = Some(index);
                state.unread_message_id = Some(boundary.id);
                state.scroll_to_divider = state.unread_count > 5;
            } else {
                state.jump_url = Some(format!("/rooms/{}?message_id={}", room.id, boundary.id));
            }
        }
    }
    if room.direct() {
        let mut stmt=conn.prepare_cached("SELECT users.id,users.name,users.ooo_until,users.ooo_note,users.presence_setting,users.time_zone,users.ooo_calendar_enabled,calendar_meeting_caches.ooo_intervals FROM users JOIN memberships ON memberships.user_id=users.id LEFT JOIN calendar_meeting_caches ON calendar_meeting_caches.user_id=users.id WHERE memberships.room_id=? AND users.id!=? AND users.status=0 AND users.role!=2 ORDER BY users.name COLLATE NOCASE")?;
        let rows = stmt.query_map([room.id, user_id], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, Option<Timestamp>>(2)?,
                r.get::<_, Option<String>>(3)?,
                r.get::<_, String>(4)?,
                r.get::<_, Option<String>>(5)?,
                r.get::<_, bool>(6)?,
                r.get::<_, Option<String>>(7)?,
            ))
        })?;
        for row in rows {
            state.notices.push(notice_from_fields(row?, now));
        }
    }
    Ok(state)
}
fn first(
    conn: &Connection,
    room_id: i64,
    operator: &str,
    at: Timestamp,
    id: Option<i64>,
) -> campfire_db::Result<Option<Message>> {
    let sql = if id.is_some() {
        format!(
            "SELECT messages.id FROM messages WHERE room_id=? AND thread_id IS NULL AND (created_at,id){operator}(?,?) ORDER BY created_at,id LIMIT 1"
        )
    } else {
        format!(
            "SELECT messages.id FROM messages WHERE room_id=? AND thread_id IS NULL AND created_at{operator}? ORDER BY created_at,id LIMIT 1"
        )
    };
    let values = if let Some(id) = id {
        vec![
            rusqlite::types::Value::from(room_id),
            at.to_db().into(),
            id.into(),
        ]
    } else {
        vec![room_id.into(), at.to_db().into()]
    };
    let id: Option<i64> = conn
        .query_row(&sql, rusqlite::params_from_iter(values), |r| r.get(0))
        .optional()?;
    id.map(|id| Message::find(conn, id)).transpose()
}

type NoticeFields = (
    i64,
    String,
    Option<Timestamp>,
    Option<String>,
    String,
    Option<String>,
    bool,
    Option<String>,
);
fn notice_from_fields(row: NoticeFields, now: Timestamp) -> Notice {
    let (id, name, manual, note, presence, zone, calendar, intervals) = row;
    let zone = campfire_views::time::Zone::for_user(zone.as_deref());
    let manual = manual.filter(|at| *at > now);
    let calendar = if calendar {
        super::super::presenters::runtime_chrome::epochs(
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
pub(crate) fn notice(
    conn: &Connection,
    user_id: i64,
    now: Timestamp,
) -> campfire_db::Result<Notice> {
    let row=conn.query_row_cached("SELECT users.id,users.name,users.ooo_until,users.ooo_note,users.presence_setting,users.time_zone,users.ooo_calendar_enabled,calendar_meeting_caches.ooo_intervals FROM users LEFT JOIN calendar_meeting_caches ON calendar_meeting_caches.user_id=users.id WHERE users.id=?",[user_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?,r.get(7)?)))?;
    Ok(notice_from_fields(row, now))
}
