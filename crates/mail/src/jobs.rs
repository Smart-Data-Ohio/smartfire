//! Typed durable jobs and the API consumed by WS9 security triggers.
use campfire_db::{Event, Job, Tx, User};
use campfire_jobs::{JobKind, RetryPolicy};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Notification {
    OAuthRefreshReuse {
        user_id: i64,
    },
    NewSignIn {
        activity_item_id: i64,
    },
    Lockout {
        user_id: i64,
    },
    /// "Forgot password?": the address as it was submitted. Delivery looks it up and mints the
    /// link, so the jobs table never holds one, and sends nothing to an address that can't reset.
    PasswordReset {
        email_address: String,
    },
    EmailAddressConfirmation(EmailAddressConfirmation),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailAddressConfirmation {
    pub user_id: i64,
    pub email_address: String,
    // Jobs queued before fingerprints were recorded deserialize and skip delivery.
    #[serde(default)]
    password_fingerprint: Option<String>,
    #[serde(default)]
    pub after_audit_id: Option<i64>,
}

impl EmailAddressConfirmation {
    pub fn new(user: &User, email_address: String, after_audit_id: i64) -> Self {
        Self {
            user_id: user.id,
            email_address,
            password_fingerprint: Some(password_fingerprint(user)),
            after_audit_id: Some(after_audit_id),
        }
    }

    pub fn matches_password(&self, user: &User) -> bool {
        self.password_fingerprint
            .as_ref()
            .is_some_and(|fingerprint| *fingerprint == password_fingerprint(user))
    }
}

fn password_fingerprint(user: &User) -> String {
    let mut hash = Sha256::new();
    if let Some(digest) = &user.password_digest {
        hash.update([1]);
        hash.update(digest.as_bytes());
    }
    format!("{:x}", hash.finalize())
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
pub fn password_reset_later(tx: &mut Tx<'_>, email_address: String) {
    tx.emit_after_commit(Event::job(&DeliveryJob {
        notification: Notification::PasswordReset { email_address },
    }));
}
pub fn email_address_confirmation_later(
    tx: &mut Tx<'_>,
    user: &User,
    email_address: String,
    after_audit_id: i64,
) {
    tx.emit_after_commit(Event::job(&DeliveryJob {
        notification: Notification::EmailAddressConfirmation(EmailAddressConfirmation::new(
            user,
            email_address,
            after_audit_id,
        )),
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
