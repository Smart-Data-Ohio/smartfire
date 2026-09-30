//! SavedItem::ReminderPushJob. Policy/payload are ready; WS17 supplies tagged push transport.
use super::Registry;
use crate::app::App;
use campfire_db::{Connection, Job, SavedItem, saved_item::ReminderPush};
use campfire_jobs::{Execution, JobKind, JobResult, Outcome};
use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize)]
#[serde(transparent)]
struct Reminder(campfire_db::saved_item::ReminderPushJob);
impl Job for Reminder {
    const CLASS: &'static str = "SavedItem::ReminderPushJob";
}
impl JobKind for Reminder {}
pub(super) fn register(registry: &mut Registry) {
    registry.register(perform);
}
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
            if !item.reminder_room_member(conn)? {
                return Ok(Outcome::Done);
            }
            let allowed = campfire_db::reminder_policy::allows(conn, item.user_id, now)?;
            if let Some(push) =
                SavedItem::reminder_push(conn, &*db.env().rich_text, id, &|_| allowed)?
            {
                return send(conn, push);
            }
            Ok(Outcome::Done)
        })
        .await
        .map_err(super::discard_missing)
}
async fn perform(app: App, job: Reminder, _: Execution) -> JobResult {
    deliver_with(app, job.0.saved_item_id, |_, push| {
        if push.subscriptions.is_empty() {
            return Ok(Outcome::Done);
        }
        // WS17 must enqueue the tag and every subscription. Keep work durable until wired.
        tracing::warn!(
            saved_tag = push.tag,
            "WS17 tagged reminder push transport is not installed"
        );
        Ok(Outcome::Again(std::time::Duration::from_secs(60)))
    })
    .await
}
