//! Message sync publication after committed mutations.
use campfire_db::{Message, Room};
use campfire_kit::{Ctx, Result};
use crate::app::{App, AppCtx};
use crate::controllers::presenters::page::db_error;

pub async fn broadcast_edit(c: &Ctx, room: &Room, message: &Message, _drive_given: bool) -> Result<()> {
    c.app().broadcasts.message_replace(room, message);
    Ok(())
}

pub async fn broadcast_thread_edit(c: &Ctx, room: &Room, message: &Message, drive_given: bool) -> Result<()> {
    broadcast_edit(c, room, message, drive_given).await
}

pub async fn broadcast_tombstones(c: &Ctx, ids: Vec<i64>) -> Result<()> {
    let app = c.app().clone();
    c.app().db.read(move |conn| {
        for id in ids {
            if let Some(message) = Message::find_by_id(conn, id)? {
                app.broadcasts.sync_message(conn, &message, false);
            }
        }
        Ok(())
    }).await.map_err(db_error)
}

pub async fn broadcast_thread_refresh(app: &App, room_id: i64, thread_id: i64) -> Result<()> {
    let runtime = app.clone();
    app.db.read(move |conn| {
        for membership in campfire_db::Membership::for_room(conn, room_id)? {
            runtime.broadcasts.thread_refresh(membership.user_id, room_id, thread_id);
        }
        Ok(())
    }).await.map_err(db_error)
}
