//! JSON delivery for committed huddle and stage changes.
use crate::app::App;
use crate::cable::huddle_sync;
use campfire_db::{Membership, Room};

pub use crate::controllers::presenters::calls::stage_model;

pub(crate) fn stream_changed(app: &App, room_id: i64) -> anyhow::Result<()> {
    app.db.read_blocking(|conn| {
        if let Some(room) = Room::find_by_id(conn, room_id)?.filter(|room| room.deleted_at.is_none() && room.stage()) {
            huddle_sync::publish_stage(&app.cable, conn, room_id);
            if app.config.huddle.configured() {
                huddle_sync::publish_presence(&app.cable, conn, &room, app.db.env().now());
            }
        }
        Ok(())
    })?;
    Ok(())
}

pub(crate) fn stream_stopped(app: &App, room_id: i64, user_id: i64) -> anyhow::Result<()> {
    let alive = app.db.read_blocking(|conn| Ok(Room::find_by_id(conn, room_id)?.is_some_and(|room| room.deleted_at.is_none() && room.stage())))?;
    if alive { huddle_sync::publish_stream_stopped(&app.cable, room_id, user_id); }
    Ok(())
}

pub(crate) fn stage_roster(app: &App, room_id: i64) -> anyhow::Result<()> {
    app.db.read_blocking(|conn| {
        if Room::find_by_id(conn, room_id)?.is_some_and(|room| room.deleted_at.is_none() && room.stage()) {
            huddle_sync::publish_stage(&app.cable, conn, room_id);
        }
        Ok(())
    })?;
    Ok(())
}

pub(crate) fn role_event(app: &App, room_id: i64, membership_id: i64) -> anyhow::Result<()> {
    app.db.read_blocking(|conn| {
        if let Some(member) = Membership::for_room(conn, room_id)?.into_iter().find(|member| member.id == membership_id) {
            huddle_sync::publish_role(&app.cable, &member);
        }
        Ok(())
    })?;
    Ok(())
}

pub fn presence(app: &App, room_id: i64) -> anyhow::Result<()> {
    if !app.config.huddle.configured() { return Ok(()); }
    app.db.read_blocking(|conn| {
        if let Some(room) = Room::find_by_id(conn, room_id)?.filter(|room| room.deleted_at.is_none()) {
            huddle_sync::publish_presence(&app.cable, conn, &room, app.db.env().now());
        }
        Ok(())
    })?;
    Ok(())
}



pub(crate) fn stage_ended_note(app: &App, message_id: i64) -> anyhow::Result<()> {
    app.db.read_blocking(|conn| {
        if let Some(message) = campfire_db::Message::find_by_id(conn, message_id)?
            && message.system_note
            && Room::find_by_id(conn, message.room_id)?.is_some_and(|room| room.stage() && room.deleted_at.is_none())
        {
            app.broadcasts.sync_message(conn, &message, true);
        }
        Ok(())
    })?;
    Ok(())
}
