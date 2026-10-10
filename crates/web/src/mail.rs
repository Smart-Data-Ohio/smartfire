//! Classic publication adapter for the shared inbound-mail runtime.
use crate::controllers::presenters::{
    Presenter, Rendering,
    page::{self, Rendered},
};
use campfire_app::app::App;
use campfire_db::{Account, Message, Room};
pub use campfire_runtime::mail::*;
pub fn register(registry: &mut campfire_app::queue::Registry) {
    campfire_runtime::mail::register(registry, publish);
}
pub async fn message_created(
    app: App,
    job: campfire_mail::jobs::MessageCreated,
    execution: campfire_jobs::Execution,
) -> campfire_jobs::JobResult {
    campfire_runtime::mail::message_created(app, job, execution, publish).await
}
fn publish(
    conn: &campfire_db::Connection,
    app: &App,
    message: &Message,
) -> campfire_db::Result<campfire_runtime::presenters::RenderRefreshes> {
    let room = Room::find(conn, message.room_id)?;
    let presenter = Presenter::new(conn, app, None);
    let view = presenter.message(message)?;
    let account = Account::first(conn)?;
    let html = page::render_detached(app, account.as_ref(), |ctx| {
        campfire_views::messages::message(ctx, &view)
    });
    app.broadcasts.message_create(
        conn,
        &room,
        message,
        &Rendered {
            message: Some(html),
            ..Rendered::default()
        },
        &*app.db.env().rich_text,
    )?;
    Ok(presenter.take_render_refreshes())
}
