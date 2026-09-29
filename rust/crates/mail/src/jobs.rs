//! Typed durable jobs and the API consumed by WS9 security triggers.
use campfire_db::{Event, Job, Tx};
use campfire_jobs::{JobKind, RetryPolicy};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Notification {
    NewSignIn { activity_item_id: i64 },
    Lockout { user_id: i64 },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryJob {
    pub notification: Notification,
}
impl Job for DeliveryJob {
    const CLASS: &'static str = "Smartfire::MailDeliveryJob";
}
impl JobKind for DeliveryJob {
    fn retry_policy() -> RetryPolicy {
        RetryPolicy::application_job().retry_on(|e| {
            campfire_jobs::is_transient(e)
                || e.chain().any(|e| {
                    e.downcast_ref::<lettre::transport::smtp::Error>()
                        .is_some_and(|e| !e.is_permanent())
                })
        })
    }
}
pub fn new_sign_in_alert_later(tx: &mut Tx<'_>, activity_item_id: i64) {
    tx.emit_after_commit(Event::job(&DeliveryJob {
        notification: Notification::NewSignIn { activity_item_id },
    }));
}
pub fn lockout_notice_later(tx: &mut Tx<'_>, user_id: i64) {
    tx.emit_after_commit(Event::job(&DeliveryJob {
        notification: Notification::Lockout { user_id },
    }));
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingJob {
    pub inbound_email_id: i64,
}
impl Job for RoutingJob {
    const CLASS: &'static str = "ActionMailbox::RoutingJob";
}
impl JobKind for RoutingJob {}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncinerationJob {
    pub inbound_email_id: i64,
}
impl Job for IncinerationJob {
    const CLASS: &'static str = "ActionMailbox::IncinerationJob";
}
impl JobKind for IncinerationJob {}
/// An Event::Job asks the app to process its attachment, broadcast_create and fan out webhooks.
/// It is durable and committed with the message, so a crash after commit can't lose the broadcast.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageCreated {
    pub message_id: i64,
}
impl Job for MessageCreated {
    const CLASS: &'static str = "Smartfire::EmailMessageCreatedJob";
}
impl JobKind for MessageCreated {}
