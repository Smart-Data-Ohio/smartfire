//! The two `ChannelThread` reads the channels need, until the messaging workstream's model lands
//! (then these should call it). Read-only: the channels never write threads.
use campfire_db::{Connection, Result};
use rails_compat::global_id::GlobalId;
use rusqlite::OptionalExtension as _;

/// A thread's GlobalID (`gid://campfire/ChannelThread/1`).
pub fn thread_gid(thread_id: i64) -> GlobalId {
    GlobalId::new("ChannelThread", thread_id)
}

/// `ChannelThread.find(id).room_id`, or `None` for `RecordNotFound`.
pub fn room_id_of(conn: &Connection, thread_id: i64) -> Result<Option<i64>> {
    Ok(conn
        .query_row(r#"SELECT "channel_threads"."room_id" FROM "channel_threads" WHERE "channel_threads"."id" = ? LIMIT 1"#, [thread_id], |row| row.get(0))
        .optional()?)
}

/// `room.channel_threads.find_by(id:)` / `.exists?(id:)`: the thread, if it's in this room.
pub fn in_room(conn: &Connection, room_id: i64, thread_id: i64) -> Result<bool> {
    Ok(room_id_of(conn, thread_id)? == Some(room_id))
}
