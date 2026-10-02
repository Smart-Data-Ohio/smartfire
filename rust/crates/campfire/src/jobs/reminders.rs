//! Test transport boundary retained from WS8bm2; production delivery is WS17 notifications.
use crate::app::App;
use campfire_db::{Connection, SavedItem, saved_item::ReminderPush};
use campfire_jobs::{JobResult, Outcome};
/// A synchronous enqueue boundary, like Rails' web_push_pool.queue; never sends on a reader.
/// Keep the tag and subscriptions intact for WS17; errors leave the durable job unfinished.
pub(crate) async fn deliver_with(
    app: App,
    id: i64,
    send: impl FnOnce(&Connection, ReminderPush) -> campfire_db::Result<Outcome> + Send + 'static,
) -> JobResult {
    let db = app.db.clone();
    let now = app.db.env().now();
    app.db
        .read(move |conn| {
            let item = SavedItem::find(conn, id)?;
            if !item.reminder_room_member(conn)? { return Ok(Outcome::Done); }
            if let Some(push) = SavedItem::reminder_push_with_policy(
                conn, &*db.env().rich_text, id, now,
            )? {
                return send(conn, push);
            }
            Ok(Outcome::Done)
        })
        .await
        .map_err(super::discard_missing)
}
