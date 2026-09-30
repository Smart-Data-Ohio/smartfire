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

#[derive(Serialize, Deserialize)]
#[serde(transparent)]
struct TestNotification(campfire_db::models::push_subscription::TestNotificationJob);
impl campfire_db::Job for TestNotification {
    const CLASS: &'static str = "Push::Subscription::TestNotificationJob";
}
impl JobKind for TestNotification {
    const QUEUE: &'static str = super::PUSH_QUEUE;
}

pub(super) fn register(registry: &mut Registry) {
    registry.register(thread_message);
    registry.register(saved_reminder);
    registry.register(test_notification);
}

async fn test_notification(app: App, job: TestNotification, _: Execution) -> JobResult {
    let Some(pool) = app.web_push.clone() else {
        return Ok(Outcome::Done);
    };
    app.db
        .read(move |conn| {
            let subscription =
                match campfire_db::PushSubscription::find(conn, job.0.subscription_id) {
                    Ok(subscription) if subscription.user_id == job.0.user_id => subscription,
                    Ok(_) | Err(campfire_db::Error::RecordNotFound(_)) => return Ok(()),
                    Err(error) => return Err(error),
                };
            // The explicit test sends even in DND/quiet/OOO, as the Rails controller does.
            let payload = campfire_db::PushPayload::new(
                "Smartfire Test".into(),
                job.0.body,
                job.0.path,
                Some("test-notification".into()),
            );
            pool.queue(conn, &payload, vec![subscription])
        })
        .await?;
    Ok(Outcome::Done)
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
