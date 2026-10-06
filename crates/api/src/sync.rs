//! The app's half of the `/api/v1/sync` socket: what a connection may follow, what its frames do
//! ([`Session`], mirroring `PresenceChannel`, `TypingNotificationsChannel` and
//! `WorkspacePresenceChannel`), and how the broadcast points render their JSON twins
//! ([`Renderer`]).

use std::collections::BTreeSet;
use std::sync::{Arc, Weak};

use campfire_api_types as api;
use campfire_app::app::AppState;
use campfire_app::cable::sync::{self as twins, SyncRenderer};
use campfire_app::cable::{CableUser, room_gid, thread_gid};
use campfire_cable::sync::{SyncHandler, SyncSession};
use campfire_db::{Connection, Membership, Message, Room, WorkspacePresenceLease};
use campfire_web::concerns::expire_idle_timed_out_session;
use rusqlite::OptionalExtension as _;

use crate::dto;

/// Renders the twins the broadcast points publish. Holds the app weakly: the app holds it.
pub struct Renderer {
    app: Weak<AppState>,
    runtime: tokio::runtime::Handle,
}

impl Renderer {
    pub fn new(app: &Arc<AppState>, runtime: tokio::runtime::Handle) -> Self {
        Self {
            app: Arc::downgrade(app),
            runtime,
        }
    }
}

impl SyncRenderer for Renderer {
    fn message(&self, conn: &Connection, message: &Message) -> Option<api::MessageDTO> {
        let app = self.app.upgrade()?;
        dto::message(conn, &app, message)
            .inspect_err(|error| tracing::warn!(%error, message_id = message.id, "sync: message not rendered"))
            .ok()
    }

    fn reactions(&self, conn: &Connection, message: &Message) -> Option<api::MessageReactions> {
        let app = self.app.upgrade()?;
        dto::message_reactions(conn, &app, message)
            .inspect_err(|error| tracing::warn!(%error, message_id = message.id, "sync: reactions not rendered"))
            .ok()
    }

    fn sidebar_row(
        &self,
        conn: &Connection,
        room: &Room,
        membership: &Membership,
    ) -> campfire_db::Result<Option<api::SidebarRow>> {
        dto::sidebar_row(conn, room, membership)
    }

    fn defer(&self, job: Box<dyn FnOnce(&Connection) + Send>) {
        let Some(app) = self.app.upgrade() else {
            return;
        };
        self.runtime.spawn(async move {
            let result = app
                .db
                .read(move |conn| {
                    job(conn);
                    Ok(())
                })
                .await;
            if let Err(error) = result {
                tracing::warn!(%error, "sync: deferred twin not read");
            }
        });
    }
}

/// Opens a [`Session`] per connection.
pub struct Handler {
    app: Weak<AppState>,
}

impl Handler {
    pub fn new(app: &Arc<AppState>) -> Self {
        Self {
            app: Arc::downgrade(app),
        }
    }
}

#[async_trait::async_trait]
impl SyncHandler<CableUser> for Handler {
    fn user_id(&self, user: &CableUser) -> i64 {
        user.id
    }

    async fn open(&self, user: Arc<CableUser>) -> Box<dyn SyncSession> {
        let mut session = Session {
            app: self.app.clone(),
            user,
            lease: None,
            present: BTreeSet::new(),
            published: None,
        };
        if let Some(app) = session.app.upgrade() {
            session.establish(&app).await;
            session.publish_presence(&app).await;
        }
        Box::new(session)
    }
}

/// One connection's person, the workspace presence lease it holds, and the rooms it's looking at.
pub struct Session {
    app: Weak<AppState>,
    user: Arc<CableUser>,
    lease: Option<WorkspacePresenceLease>,
    present: BTreeSet<i64>,
    /// The presence last published for the person, so a heartbeat only speaks up on a change.
    published: Option<api::UserPresence>,
}

/// A conversation topic: `room:<id>` or `thread:<id>`.
#[derive(Clone, Copy)]
enum Conversation {
    Room(i64),
    Thread(i64),
}

impl Conversation {
    fn parse(topic: &str) -> Option<Self> {
        let digits = |rest: &str| {
            rest.bytes()
                .all(|b| b.is_ascii_digit())
                .then(|| rest.parse().ok())
                .flatten()
        };
        if let Some(rest) = topic.strip_prefix("room:") {
            digits(rest).map(Self::Room)
        } else {
            topic
                .strip_prefix("thread:")
                .and_then(digits)
                .map(Self::Thread)
        }
    }

    /// The alive room the person is a member of that holds the conversation, if any.
    fn room(self, conn: &Connection, user_id: i64) -> campfire_db::Result<Option<Room>> {
        let room_id = match self {
            Self::Room(room_id) => room_id,
            Self::Thread(thread_id) => {
                let room_id = conn
                    .query_row(r#"SELECT "channel_threads"."room_id" FROM "channel_threads" WHERE "channel_threads"."id" = ? LIMIT 1"#, [thread_id], |row| row.get::<_, i64>(0))
                    .optional()?;
                let Some(room_id) = room_id else {
                    return Ok(None);
                };
                room_id
            }
        };
        Room::find_for_user(conn, user_id, room_id)
    }
}

impl Session {
    /// `WorkspacePresenceChannel#subscribed`: a lease for this connection.
    async fn establish(&mut self, app: &AppState) {
        let (user_id, session_id) = (self.user.id, self.user.session_id);
        match app
            .db
            .write(move |tx| WorkspacePresenceLease::establish(tx, user_id, session_id))
            .await
        {
            Ok(lease) => self.lease = lease,
            Err(error) => tracing::warn!(%error, user_id, "sync: presence lease not established"),
        }
    }

    /// The person's presence, published to everyone when it differs from the last one sent.
    async fn publish_presence(&mut self, app: &AppState) {
        if !self.user.active_human() {
            return;
        }
        let (user_id, now) = (self.user.id, app.db.env().now());
        let presence = match app
            .db
            .read(move |conn| dto::presences(conn, &[user_id], now))
            .await
        {
            Ok(mut presences) if !presences.is_empty() => presences.remove(0),
            Ok(_) => return,
            Err(error) => return tracing::warn!(%error, user_id, "sync: presence not read"),
        };
        if self.published.as_ref() != Some(&presence) {
            twins::presence(&app.cable, presence.clone());
            self.published = Some(presence);
        }
    }

    /// Applies `change` to the person's membership in `room_id`; false when there's none.
    async fn with_membership(
        &self,
        app: &AppState,
        room_id: i64,
        change: impl FnOnce(&mut Membership, &mut campfire_db::Tx<'_>) -> campfire_db::Result<()>
        + Send
        + 'static,
    ) -> bool {
        let user_id = self.user.id;
        let result = app
            .db
            .write(move |tx| {
                match Membership::find_by_room_and_user(tx.conn(), room_id, user_id)? {
                    Some(mut membership) => change(&mut membership, tx).map(|()| true),
                    None => Ok(false),
                }
            })
            .await;
        result.unwrap_or_else(|error| {
            tracing::warn!(%error, room_id, user_id, "sync: membership presence not saved");
            false
        })
    }
}

#[async_trait::async_trait]
impl SyncSession for Session {
    async fn authorize(&mut self, topic: &str) -> bool {
        let (Some(app), Some(conversation)) = (self.app.upgrade(), Conversation::parse(topic))
        else {
            return false;
        };
        let user_id = self.user.id;
        app.db
            .read(move |conn| conversation.room(conn, user_id))
            .await
            .ok()
            .flatten()
            .is_some()
    }

    /// `TypingNotificationsChannel#start`/`#stop`: the classic stream (so the classic pages see
    /// it) and its twin.
    async fn typing(&mut self, topic: &str, on: bool) {
        let (Some(app), Some(conversation)) = (self.app.upgrade(), Conversation::parse(topic))
        else {
            return;
        };
        let user_id = self.user.id;
        let Ok(Some(room)) = app
            .db
            .read(move |conn| conversation.room(conn, user_id))
            .await
        else {
            return;
        };
        let gid = match conversation {
            Conversation::Room(_) => room_gid(&room),
            Conversation::Thread(thread_id) => thread_gid(thread_id),
        };
        let payload = serde_json::json!({
            "action": if on { "start" } else { "stop" },
            "user": { "id": self.user.id, "name": self.user.name },
        });
        app.cable
            .broadcast_to("TypingNotificationsChannel", &[&gid.to_param()], &payload);
        twins::typing(&app.cable, topic, self.user.id, on);
    }

    /// `PresenceChannel#subscribed`: the person is in the room, which they've now read.
    async fn present(&mut self, room_id: i64) {
        let Some(app) = self.app.upgrade() else {
            return;
        };
        if self.present.contains(&room_id) {
            return;
        }
        if self
            .with_membership(&app, room_id, |membership, tx| membership.present(tx))
            .await
        {
            self.present.insert(room_id);
            campfire_app::cable::broadcasts::read_room(&app.cable, self.user.id, room_id);
        }
    }

    /// `PresenceChannel#unsubscribed`.
    async fn absent(&mut self, room_id: i64) {
        let Some(app) = self.app.upgrade() else {
            return;
        };
        if self.present.remove(&room_id) {
            self.with_membership(&app, room_id, |membership, tx| membership.disconnected(tx))
                .await;
        }
    }

    /// `WorkspacePresenceChannel#heartbeat` (and `PresenceChannel#refresh` for the rooms open).
    /// An admin's session that idled out is destroyed here, as the classic channel does; unlike
    /// it, this connection then ends too.
    async fn heartbeat(&mut self, active: bool) -> bool {
        let Some(app) = self.app.upgrade() else {
            return true;
        };
        let (session_id, timeout, lease) = (
            self.user.session_id,
            app.config.admin_session_idle_timeout,
            self.lease.clone(),
        );
        let result = app
            .db
            .write(move |tx| {
                expire_idle_timed_out_session(tx, session_id, timeout)?;
                match campfire_db::Session::find(tx.conn(), session_id) {
                    Ok(_) => {}
                    Err(campfire_db::Error::RecordNotFound(_)) => return Ok(None),
                    Err(error) => return Err(error),
                }
                match lease {
                    Some(mut lease) => {
                        let refreshed = lease.refresh(tx, active)?;
                        Ok(Some((Some(lease), refreshed)))
                    }
                    None => Ok(Some((None, false))),
                }
            })
            .await;
        match result {
            Ok(None) => return false,
            Ok(Some((lease, true))) => self.lease = lease,
            Ok(Some((_, false))) => self.establish(&app).await,
            Err(error) => {
                tracing::warn!(%error, user_id = self.user.id, "sync: heartbeat not saved")
            }
        }
        for room_id in self.present.clone() {
            self.with_membership(&app, room_id, |membership, tx| {
                membership.refresh_connection(tx)
            })
            .await;
        }
        self.publish_presence(&app).await;
        true
    }

    async fn close(&mut self) {
        let Some(app) = self.app.upgrade() else {
            return;
        };
        for room_id in std::mem::take(&mut self.present) {
            self.with_membership(&app, room_id, |membership, tx| membership.disconnected(tx))
                .await;
        }
        if let Some(mut lease) = self.lease.take()
            && let Err(error) = app.db.write(move |tx| lease.delete(tx)).await
        {
            tracing::warn!(%error, user_id = self.user.id, "sync: presence lease not removed");
        }
        self.publish_presence(&app).await;
    }
}

#[cfg(test)]
mod tests {
    use super::Conversation;

    #[test]
    fn only_room_and_thread_topics_with_ids_are_conversations() {
        assert!(matches!(
            Conversation::parse("room:12"),
            Some(Conversation::Room(12))
        ));
        assert!(matches!(
            Conversation::parse("thread:3"),
            Some(Conversation::Thread(3))
        ));
        for topic in [
            "room:",
            "room:-1",
            "room:1x",
            "user",
            "thread:+2",
            "rooms:1",
        ] {
            assert!(Conversation::parse(topic).is_none(), "{topic}");
        }
    }
}
