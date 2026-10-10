//! Notifier's `message.broadcast_create`, published as JSON without a current viewer or session.
use crate::app::App;
use crate::integrations::github::notifier::MessageCreated;
use campfire_db::{Message, Room};

pub fn publish(app: &App, event: &MessageCreated) -> anyhow::Result<()> {
    let event = event.clone();
    let app_for_read = app.clone();
    app.db.read_blocking(move |conn| {
        let app = &app_for_read;
        let message = Message::find(conn, event.message_id)?;
        let room = Room::find(conn, event.room_id)?;
        app.broadcasts
            .message_create(conn, &room, &message, &*app.db.env().rich_text)
    })?;
    Ok(())
}
