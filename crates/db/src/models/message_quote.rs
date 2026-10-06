//! Quote endpoint scope and source visibility, independent of HTML.
use crate::{Connection, Message, Result, Room};
pub fn source(conn: &Connection, room_id: i64, reference_id: i64) -> Result<Message> {
    let id=conn.query_row("SELECT r.referenced_message_id FROM message_references r JOIN messages m ON m.id=r.message_id WHERE r.id=? AND m.room_id=?",(reference_id,room_id),|r|r.get::<_,i64>(0)).map_err(|e|if matches!(e,rusqlite::Error::QueryReturnedNoRows){crate::Error::RecordNotFound("MessageReference")}else{e.into()})?;
    Message::find(conn, id)
}
pub fn visible(conn: &Connection, source: &Message, viewer: i64) -> Result<bool> {
    Ok(!source.system_note && Room::find_for_user(conn, viewer, source.room_id)?.is_some())
}
