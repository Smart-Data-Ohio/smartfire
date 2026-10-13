//! Quote endpoint scope and source visibility, independent of HTML.
use crate::{Connection, Message, Result, Room};
pub fn source(conn: &Connection, room_id: i64, reference_id: i64) -> Result<Message> {
    let id=conn.query_row("SELECT r.referenced_message_id FROM message_references r JOIN messages m ON m.id=r.message_id WHERE r.id=? AND m.room_id=?",(reference_id,room_id),|r|r.get::<_,i64>(0)).map_err(|e|if matches!(e,rusqlite::Error::QueryReturnedNoRows){crate::Error::RecordNotFound("MessageReference")}else{e.into()})?;
    Message::find(conn, id)
}
pub fn visible(conn: &Connection, source: &Message, viewer: i64) -> Result<bool> {
    if source.system_note || Room::find_for_user(conn, viewer, source.room_id)?.is_none() { return Ok(false); }
    match source.thread_id {
        Some(id) => crate::ChannelThread::find(conn, id)?.readable_by(conn, viewer),
        None => Ok(true),
    }
}

/// A write grant may name a source without granting disclosure of its content.
pub fn content_visible(conn:&Connection,source:&Message,viewer:i64) -> Result<bool> {
    if super::agent_access::is_personal_user(conn,viewer)? && super::agent_access::capability_for_user(conn,viewer,"read_messages",source.room_id)? == Some(false) {
        return Ok(false);
    }
    visible(conn,source,viewer)
}

/// Both the holder and its quoted source must be readable by this viewer.
pub fn source_for_viewer(conn: &Connection, room_id: i64, reference_id: i64, viewer: i64) -> Result<Message> {
    let holder = crate::sql::query_one(conn, "SELECT message_id FROM message_references WHERE id=?", [reference_id], |row| row.get::<_, i64>(0))?
        .ok_or(crate::Error::RecordNotFound("MessageReference"))?;
    let holder = Message::find(conn, holder)?;
    if holder.room_id != room_id || !visible(conn, &holder, viewer)? {
        return Err(crate::Error::RecordNotFound("MessageReference"));
    }
    let source = source(conn, room_id, reference_id)?;
    if let Some(thread_id) = source.thread_id {
        let thread = crate::ChannelThread::find(conn, thread_id)?;
        if thread.visibility == super::channel_thread::ThreadVisibility::Private
            && !thread.readable_by(conn, viewer)? {
            return Err(crate::Error::RecordNotFound("MessageReference"));
        }
    }
    Ok(source)
}
