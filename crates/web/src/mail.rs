//! Classic publication adapter for the shared inbound-mail runtime.
use campfire_app::app::App;
use campfire_db::{Message, Room};
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
    let refreshes = campfire_runtime::presenters::broadcast_refreshes(conn, app, message)?;
    app.broadcasts.message_create(
        conn,
        &room,
        message,
        &*app.db.env().rich_text,
    )?;
    Ok(refreshes)
}
