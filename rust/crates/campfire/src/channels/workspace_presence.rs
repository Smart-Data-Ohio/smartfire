//! `WorkspacePresenceChannel` (reference/app/channels/workspace_presence_channel.rb): no stream; a
//! `WorkspacePresenceLease` per subscription, established on subscribe, extended by the client's
//! `heartbeat` (every 25 s) and deleted on unsubscribe. The presence reads and the periodic prune
//! belong to the presence workstream.
//!
//! A heartbeat first destroys the connection's session if it's an administrator's that has sat
//! idle past the timeout (a fresh read, not the connect-time state); the lease refresh then fails,
//! re-establishing fails, and the subscription is rejected. As in Rails, a rejection inside an
//! action sends no frame: the subscription just stops performing actions.
use campfire_cable::{Channel, ChannelResult, Params, Subscription};
use campfire_db::{Database, Session, User, WorkspacePresenceLease};
use jiff::SignedDuration;

use super::CableUser;
use crate::concerns::session_expired;

pub struct WorkspacePresenceChannel {
    db: Database,
    admin_session_idle_timeout: SignedDuration,
    /// `@lease`
    lease: Option<WorkspacePresenceLease>,
}

impl WorkspacePresenceChannel {
    pub fn new(db: Database, admin_session_idle_timeout: SignedDuration) -> Self {
        Self { db, admin_session_idle_timeout, lease: None }
    }

    /// `@lease = WorkspacePresenceLease.establish(user: current_user, session: connection.current_session)`,
    /// and `reject unless @lease`.
    async fn establish(&mut self, sub: &mut Subscription<CableUser>) -> ChannelResult {
        let (user_id, session_id) = (sub.current_user().id, sub.current_user().session_id);
        self.lease = self.db.write(move |tx| WorkspacePresenceLease::establish(tx, user_id, session_id)).await?;
        if self.lease.is_none() {
            sub.reject();
        }
        Ok(())
    }

    /// `heartbeat(data)`
    async fn heartbeat(&mut self, data: &Params, sub: &mut Subscription<CableUser>) -> ChannelResult {
        let session_id = sub.current_user().session_id;
        let timeout = self.admin_session_idle_timeout;
        let active = data.get("active") == Some(&serde_json::Value::Bool(true));
        let lease = self.lease.clone();
        let (lease, refreshed) = self
            .db
            .write(move |tx| {
                expire_idle_timed_out_session(tx, session_id, timeout)?;
                // `@lease&.refresh(active: data["active"] == true)`
                match lease {
                    Some(mut lease) => {
                        let refreshed = lease.refresh(tx, active);
                        Ok((Some(lease), refreshed))
                    }
                    None => Ok((None, Ok(false))),
                }
            })
            .await?;
        self.lease = lease;
        if refreshed? {
            return Ok(());
        }
        self.establish(sub).await
    }
}

/// `expire_idle_timed_out_session!`: `Session.find_by(id:)`, destroyed if `expired?`. A session
/// that's already gone is skipped.
fn expire_idle_timed_out_session(tx: &mut campfire_db::Tx<'_>, session_id: i64, timeout: SignedDuration) -> campfire_db::Result<()> {
    let fresh = match Session::find(tx.conn(), session_id) {
        Ok(session) => session,
        Err(campfire_db::Error::RecordNotFound(_)) => return Ok(()),
        Err(error) => return Err(error),
    };
    let user = User::find(tx.conn(), fresh.user_id)?;
    if session_expired(&fresh, &user, timeout, tx.now()) {
        fresh.destroy(tx)?;
    }
    Ok(())
}

#[async_trait::async_trait]
impl Channel<CableUser> for WorkspacePresenceChannel {
    async fn subscribed(&mut self, sub: &mut Subscription<CableUser>) -> ChannelResult {
        self.establish(sub).await
    }

    /// `@lease&.delete`
    async fn unsubscribed(&mut self, _sub: &mut Subscription<CableUser>) -> ChannelResult {
        if let Some(mut lease) = self.lease.clone() {
            let lease = self
                .db
                .write(move |tx| {
                    lease.delete(tx)?;
                    Ok(lease)
                })
                .await?;
            self.lease = Some(lease);
        }
        Ok(())
    }

    /// Its public methods: `subscribed`, `unsubscribed` and `heartbeat`.
    async fn perform(&mut self, action: &str, data: &Params, sub: &mut Subscription<CableUser>) -> ChannelResult<bool> {
        if sub.rejected() {
            return Ok(false);
        }
        match action {
            "subscribed" => self.subscribed(sub).await?,
            "unsubscribed" => self.unsubscribed(sub).await?,
            "heartbeat" => self.heartbeat(data, sub).await?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}
