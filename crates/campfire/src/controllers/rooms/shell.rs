//! Read-only request adapter. Personal notice sets never enter the fragment cache.
#[cfg(test)]
use campfire_db::{CachedStatements, Connection, Timestamp};
#[cfg(test)]
use campfire_db::{Membership, Message, Room};
#[cfg(test)]
use crate::controllers::presenters::room_shell::notice_from_fields;
#[cfg(test)]
use campfire_views::rooms::shell::State;
#[cfg(test)]
use rusqlite::OptionalExtension;

#[cfg(test)]
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
#[cfg(test)]
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

