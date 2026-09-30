//! Durable execution for the messaging push jobs persisted by WS8a.
use super::Registry;
use crate::app::App;
use campfire_db::{ChannelThread, SavedItem};
use campfire_jobs::{Execution, JobKind, JobResult, Outcome};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(transparent)]
struct ThreadMessage(campfire_db::models::channel_thread::PushMessageJob);
impl campfire_db::Job for ThreadMessage {
    const CLASS: &'static str = "ChannelThread::PushMessageJob";
}
impl JobKind for ThreadMessage {}

#[derive(Serialize, Deserialize)]
#[serde(transparent)]
struct SavedReminder(campfire_db::models::saved_item::ReminderPushJob);
impl campfire_db::Job for SavedReminder {
    const CLASS: &'static str = "SavedItem::ReminderPushJob";
}
impl JobKind for SavedReminder {}

pub(super) fn register(registry: &mut Registry) {
    registry.register(thread_message);
    registry.register(saved_reminder);
}

async fn thread_message(app: App, job: ThreadMessage, _: Execution) -> JobResult {
    let Some(pool) = app.web_push.clone() else {
        return Ok(Outcome::Done);
    };
    let db = app.db.clone();
    app.db
        .read(move |conn| {
            for push in ChannelThread::push_recipients_with_policy(
                conn,
                &*db.env().rich_text,
                job.0.thread_id,
                job.0.message_id,
                db.env().now(),
            )? {
                pool.queue(conn, &push.payload, push.subscriptions)?;
            }
            Ok(())
        })
        .await?;
    Ok(Outcome::Done)
}

async fn saved_reminder(app: App, job: SavedReminder, _: Execution) -> JobResult {
    let Some(pool) = app.web_push.clone() else {
        return Ok(Outcome::Done);
    };
    let db = app.db.clone();
    app.db
        .read(move |conn| {
            if let Some(push) = SavedItem::reminder_push_with_policy(
                conn,
                &*db.env().rich_text,
                job.0.saved_item_id,
                db.env().now(),
            )? {
                pool.queue(conn, &push.payload, push.subscriptions)?;
            }
            Ok(())
        })
        .await?;
    Ok(Outcome::Done)
}
