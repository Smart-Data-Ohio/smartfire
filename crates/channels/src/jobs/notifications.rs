//! Durable execution for the messaging push jobs persisted by WS8a.
use super::Registry;
use crate::app::App;
use campfire_db::{ChannelThread, SavedItem};
use campfire_db::models::activity_item::message_recorder::MentionPushJob;
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
    const QUEUE: &'static str = crate::queue::PUSH_QUEUE;
}

pub(super) fn register(registry: &mut Registry) {
    registry.register(thread_message);
    registry.register(message_mentions);
    registry.register(saved_reminder);
    registry.register(test_notification);
    registry.register(event_reminder);
    registry.register(board_nudge);
    registry.register(huddle_invitation_delivery);
    registry.register(huddle_join_delivery);
    registry.register(huddle_push_request);
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

async fn thread_message(app: App, job: ThreadMessage, execution: Execution) -> JobResult {
    let Some(pool) = app.web_push.clone() else {
        return Ok(Outcome::Done);
    };
    let db = app.db.clone();
    app.db
        .write(move |tx| {
            let excluded = MentionPushJob::original_push_exclusions(tx.conn(), execution.id)?;
            for push in ChannelThread::push_recipients_with_policy(
                tx.conn(),
                &*db.env().rich_text,
                job.0.thread_id,
                job.0.message_id,
                tx.now(),
            )? {
                if !excluded.contains(&push.user_id) {
                    pool.queue(tx.conn(), &push.payload, push.subscriptions)?;
                }
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

macro_rules! source_job {
    ($name:ident,$source:ty) => {
        #[derive(Serialize, Deserialize)]
        #[serde(transparent)]
        struct $name($source);
        impl campfire_db::Job for $name {
            const CLASS: &'static str = <$source as campfire_db::Job>::CLASS;
        }
        impl JobKind for $name {}
    };
}
source_job!(
    MessageMentions,
    campfire_db::models::activity_item::message_recorder::MentionPushJob
);
source_job!(
    EventReminder,
    campfire_db::models::notification_push::EventReminderJob
);
source_job!(
    BoardNudge,
    campfire_db::models::notification_push::BoardNudgeJob
);
source_job!(
    HuddleInvitationDelivery,
    campfire_db::models::notification_push::HuddleInvitationDeliveryJob
);
source_job!(
    HuddleJoinDelivery,
    campfire_db::models::notification_push::HuddleJoinDeliveryJob
);
source_job!(
    HuddlePush,
    campfire_db::models::notification_push::HuddlePushRequest
);

async fn message_mentions(app: App, job: MessageMentions, _: Execution) -> JobResult {
    let Some(pool) = app.web_push.clone() else {
        return Ok(Outcome::Done);
    };
    let db = app.db.clone();
    app.db
        .read(move |conn| {
            for push in job
                .0
                .deliveries(conn, &*db.env().rich_text, db.env().now())?
            {
                pool.queue(conn, &push.payload, push.subscriptions)?;
            }
            Ok(())
        })
        .await?;
    Ok(Outcome::Done)
}

async fn huddle_push_request(app: App, job: HuddlePush, _: Execution) -> JobResult {
    app.db
        .write(move |tx| {
            campfire_db::models::notification_push::enqueue_huddle_request(tx, job.0)?;
            Ok(())
        })
        .await
        .map_err(super::discard_missing)?;
    Ok(Outcome::Done)
}

async fn event_reminder(app: App, job: EventReminder, _: Execution) -> JobResult {
    let Some(pool) = app.web_push.clone() else {
        return Ok(Outcome::Done);
    };
    let now = app.db.env().now();
    app.db
        .read(move |conn| {
            if let Some(push) = campfire_db::models::notification_push::event_reminder_push(
                conn,
                job.0.event_id,
                now,
            )? {
                pool.queue(conn, &push.payload, push.subscriptions)?;
            }
            Ok(())
        })
        .await
        .map_err(super::discard_missing)?;
    Ok(Outcome::Done)
}
async fn board_nudge(app: App, job: BoardNudge, _: Execution) -> JobResult {
    let Some(pool) = app.web_push.clone() else {
        return Ok(Outcome::Done);
    };
    let now = app.db.env().now();
    app.db
        .read(move |conn| {
            if let Some(push) =
                campfire_db::models::notification_push::board_nudge_push(conn, job.0.nudge_id, now)?
            {
                pool.queue(conn, &push.payload, push.subscriptions)?;
            }
            Ok(())
        })
        .await
        .map_err(super::discard_missing)?;
    Ok(Outcome::Done)
}
async fn huddle_delivery(app: App, payload: campfire_db::PushPayload, ids: Vec<i64>) -> JobResult {
    let Some(pool) = app.web_push.clone() else {
        return Ok(Outcome::Done);
    };
    // The source adapter already checked policy and, for joins, committed its throttle with
    // this job. Preserve that decision/payload; Pool builds the current unread badge at run.
    app.db
        .read(move |conn| {
            pool.queue(
                conn,
                &payload,
                campfire_db::PushSubscription::for_ids(conn, &ids)?,
            )
        })
        .await?;
    Ok(Outcome::Done)
}
async fn huddle_invitation_delivery(
    app: App,
    job: HuddleInvitationDelivery,
    _: Execution,
) -> JobResult {
    huddle_delivery(app, job.0.payload, job.0.subscription_ids).await
}
async fn huddle_join_delivery(app: App, job: HuddleJoinDelivery, _: Execution) -> JobResult {
    huddle_delivery(app, job.0.payload, job.0.subscription_ids).await
}
