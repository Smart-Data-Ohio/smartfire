//! Notifier's `message.broadcast_create`, rendered without a current viewer or session.
use crate::app::App;
use crate::controllers::presenters::{
    Presenter,
    page::{self, Rendered},
};
use crate::integrations::github::notifier::MessageCreated;
use campfire_db::{Account, Message, Room};

pub fn publish(app: &App, event: &MessageCreated) -> anyhow::Result<()> {
    let event = event.clone();
    let app_for_read = app.clone();
    app.db.read_blocking(move |conn| {
        let app = &app_for_read;
        let message = Message::find(conn, event.message_id)?;
        let room = Room::find(conn, event.room_id)?;
        let view = Presenter::new(conn, app, None).message(&message)?;
        let account = Account::first(conn)?;
        let html = page::render_detached(app, account.as_ref(), |ctx| {
            campfire_views::messages::message(ctx, &view)
        });
        let partials = Rendered {
            message: Some(html),
            ..Default::default()
        };
        app.broadcasts
            .message_create(conn, &room, &message, &partials, &*app.db.env().rich_text)
    })?;
    Ok(())
}
