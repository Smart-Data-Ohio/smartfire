//! The app's half of the `/api/v1/sync` socket: what a connection may follow, what its frames do
//! ([`Session`], mirroring `PresenceChannel`, `TypingNotificationsChannel` and
//! `WorkspacePresenceChannel`), and how the broadcast points render their JSON twins
//! ([`Renderer`]).

use std::collections::BTreeSet;
#[cfg(feature = "test-support")]
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Weak};

use campfire_api_types as api;
use campfire_app::app::AppState;
use campfire_app::cable::sync::{self as twins, SyncRenderer};
use campfire_app::cable::{CableUser, room_gid, thread_gid};
use campfire_cable::sync::{SyncHandler, SyncSession};
use campfire_db::{Connection, Membership, Message, Room, WorkspacePresenceLease};
use campfire_runtime::concerns::expire_idle_timed_out_session;

use crate::dto;

/// Renders the twins the broadcast points publish. Holds the app weakly: the app holds it.
pub struct Renderer {
    app: Weak<AppState>,
    runtime: tokio::runtime::Handle,
    #[cfg(feature = "test-support")]
    deferred: Arc<DeferredJobs>,
}

#[cfg(feature = "test-support")]
#[derive(Default)]
struct DeferredJobs {
    outstanding: AtomicUsize,
    finished: tokio::sync::Notify,
}

#[cfg(feature = "test-support")]
impl DeferredJobs {
    fn start(self: &Arc<Self>) -> DeferredJob {
        self.outstanding.fetch_add(1, Ordering::AcqRel);
        DeferredJob(self.clone())
    }

    async fn settle(&self) {
        loop {
            let notified = self.finished.notified();
            tokio::pin!(notified);
            // Register before checking: the last job can finish between the check and await.
            notified.as_mut().enable();
            if self.outstanding.load(Ordering::Acquire) == 0 {
                return;
            }
            notified.await;
        }
    }
}

#[cfg(feature = "test-support")]
struct DeferredJob(Arc<DeferredJobs>);

#[cfg(feature = "test-support")]
impl Drop for DeferredJob {
    fn drop(&mut self) {
        if self.0.outstanding.fetch_sub(1, Ordering::AcqRel) == 1 {
            self.0.finished.notify_waiters();
        }
    }
}

impl Renderer {
    pub fn new(app: &Arc<AppState>, runtime: tokio::runtime::Handle) -> Self {
        Self {
            app: Arc::downgrade(app),
            runtime,
            #[cfg(feature = "test-support")]
            deferred: Arc::default(),
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

    fn messages(&self, conn: &Connection, messages: &[Message]) -> Option<Vec<api::MessageDTO>> {
        let app = self.app.upgrade()?;
        dto::digest_messages(conn, &app, messages)
            .inspect_err(|error| tracing::warn!(%error, "sync: digest batch not rendered"))
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
        conn: &campfire_db::Snapshot<'_>,
        room: &Room,
        membership: &Membership,
    ) -> campfire_db::Result<Option<api::SidebarRow>> {
        let Some(app) = self.app.upgrade() else {
            return Ok(None);
        };
        let row = dto::sidebar_row(conn, room, membership, app.db.env().now());
        #[cfg(feature = "test-support")]
        {
            crate::test_hooks::after_sidebar_snapshot(app.db.path(), room.id);
            if crate::test_hooks::read_after_sidebar_snapshot(app.db.path(), room.id) {
                campfire_app::cable::sync::room_read(&app.cable, membership.user_id, room.id);
            }
        }
        row
    }

    fn workspace_layout(
        &self,
        conn: &Connection,
        user_id: i64,
    ) -> campfire_db::Result<Option<api::WorkspaceLayout>> {
        workspace_layout(conn, user_id).map(Some)
    }

    fn thread(
        &self,
        conn: &Connection,
        thread: &campfire_db::ChannelThread,
    ) -> Option<api::Thread> {
        let app = self.app.upgrade()?;
        let now = app.db.env().now();
        let dto = Room::find(conn, thread.room_id)
            .and_then(|room| {
                let work =
                    crate::work::facts(conn, &app.secrets, std::slice::from_ref(thread), now)?
                        .remove(&thread.id);
                Ok(dto::thread(thread, &room, now, work))
            })
            .inspect_err(
                |error| tracing::warn!(%error, thread_id = thread.id, "sync: thread not rendered"),
            )
            .ok();
        #[cfg(feature = "test-support")]
        crate::test_hooks::after_thread_snapshot(app.db.path(), thread.id);
        dto
    }

    fn thread_indicator(
        &self,
        conn: &Connection,
        parent: &Message,
    ) -> campfire_db::Result<Option<api::ThreadIndicator>> {
        dto::thread_indicator(conn, parent)
    }

    fn activity_item(
        &self,
        conn: &Connection,
        user_id: i64,
        item_id: i64,
    ) -> campfire_db::Result<Option<api::ActivityItemChanged>> {
        let Some(app) = self.app.upgrade() else {
            return Ok(None);
        };
        let Some(viewer) = campfire_db::User::find_by_id(conn, user_id)? else {
            return Ok(None);
        };
        if let Some(changed) = crate::activity::changed(conn, &app, &viewer, item_id)? {
            return Ok(Some(changed));
        }
        let unread = campfire_db::ActivityItem::unread_snapshot(conn, user_id, app.db.env().now())?;
        Ok(Some(api::ActivityItemChanged {
            item: None,
            unread_count: unread.count,
            unread_revision: unread.revision,
            evaluated_at: unread.evaluated_at.to_evaluation_time(),
        }))
    }

    fn activity_unread_count(
        &self,
        conn: &Connection,
        user_id: i64,
    ) -> campfire_db::Result<Option<campfire_db::models::activity_item::ActivityUnread>> {
        let Some(app) = self.app.upgrade() else {
            return Ok(None);
        };
        campfire_db::ActivityItem::unread_snapshot(conn, user_id, app.db.env().now()).map(Some)
    }

    fn scheduled_message(
        &self,
        conn: &Connection,
        id: i64,
    ) -> campfire_db::Result<Option<api::ScheduledMessage>> {
        let Some(app) = self.app.upgrade() else {
            return Ok(None);
        };
        let Some(row) = campfire_db::ScheduledMessage::find_by_id(conn, id)? else {
            return Ok(None);
        };
        Ok(crate::composer::scheduled_rows(conn, &[row], app.db.env().now(), &app)?.pop())
    }

    fn agent_status(
        &self,
        conn: &Connection,
        agent_id: i64,
    ) -> campfire_db::Result<Option<api::AgentStatusChanged>> {
        let Some(app) = self.app.upgrade() else {
            return Ok(None);
        };
        let Some(agent) = campfire_db::Agent::find(conn, agent_id)? else {
            return Ok(None);
        };
        let now = app.db.env().now();
        let working_presence = agent.working_presence_text(now).map(str::to_string);
        Ok(Some(api::AgentStatusChanged {
            agent_id,
            user_id: agent.user_id,
            status: dto::agent_status(&agent.status),
            status_note: agent.status_note.clone(),
            status_changed_at: agent.status_changed_at.map(dto::time),
            updated_at: dto::row_version(agent.updated_at),
            suspended: agent.suspended_at.is_some(),
            working_presence_expires_at: working_presence
                .as_ref()
                .and(agent.working_presence_expires_at)
                .map(dto::time),
            working_presence,
        }))
    }

    fn agent_steps(
        &self,
        conn: &Connection,
        message_id: Option<i64>,
        thread_id: Option<i64>,
    ) -> campfire_db::Result<Option<api::AgentStepsChanged>> {
        if let Some(id) = message_id {
            let Some(message) = Message::find_by_id(conn, id)? else {
                return Ok(None);
            };
            let steps = dto::message_steps(conn, &[id])?
                .remove(&id)
                .unwrap_or_default();
            return Ok(Some(api::AgentStepsChanged {
                room_id: message.room_id,
                message_id: Some(id),
                thread_id: message.thread_id,
                steps,
            }));
        }
        let Some(id) = thread_id else {
            return Ok(None);
        };
        let Some(thread) = campfire_db::ChannelThread::find_by_id(conn, id)? else {
            return Ok(None);
        };
        Ok(Some(api::AgentStepsChanged {
            room_id: thread.room_id,
            message_id: None,
            thread_id: Some(id),
            steps: dto::thread_steps(conn, id)?,
        }))
    }

    fn approval_updated(
        &self,
        conn: &Connection,
        approval_id: i64,
        user_id: i64,
    ) -> campfire_db::Result<Option<api::ApprovalUpdated>> {
        let Some(app) = self.app.upgrade() else {
            return Ok(None);
        };
        let Some(viewer) = campfire_db::User::find_by_id(conn, user_id)? else {
            return Ok(None);
        };
        crate::agents::approval_update(conn, &app, approval_id, &viewer, app.db.env().now())
    }

    fn poll(
        &self,
        conn: &Connection,
        poll_id: i64,
        voter_id: Option<i64>,
    ) -> campfire_db::Result<Option<(api::PollUpdated, Option<api::PollBallot>)>> {
        let Some(app) = self.app.upgrade() else {
            return Ok(None);
        };
        crate::cards::poll_changed(
            conn,
            poll_id,
            voter_id,
            app.db.env().now(),
            &*app.storage.verifier,
        )
    }

    fn message_cards(
        &self,
        conn: &Connection,
        messages: &[Message],
    ) -> campfire_db::Result<Vec<api::MessageCards>> {
        let Some(app) = self.app.upgrade() else {
            return Ok(Vec::new());
        };
        crate::cards::message_cards(conn, &app, messages)
    }

    fn defer(&self, job: Box<dyn FnOnce(&Connection) + Send>) {
        let Some(app) = self.app.upgrade() else {
            return;
        };
        #[cfg(feature = "test-support")]
        let deferred = self.deferred.start();
        self.runtime.spawn(async move {
            let result = app
                .db
                .read(move |conn| {
                    // A cancelled async read may leave its blocking reader running. Keep the
                    // guard with that reader until its job has really finished publishing.
                    #[cfg(feature = "test-support")]
                    let _deferred = deferred;
                    job(conn);
                    Ok(())
                })
                .await;
            if let Err(error) = result {
                tracing::warn!(%error, "sync: deferred twin not read");
            }
        });
    }

    fn defer_unread(&self, job: twins::UnreadJob) {
        let Some(app) = self.app.upgrade() else {
            return;
        };
        #[cfg(feature = "test-support")]
        let deferred = self.deferred.start();
        // A blocking thread, not a reader: the job may wait on a lock before it reads.
        self.runtime.spawn_blocking(move || {
            #[cfg(feature = "test-support")]
            let _deferred = deferred;
            job(&PooledReader(&app.db));
        });
    }

    #[cfg(feature = "test-support")]
    fn settle(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + '_>> {
        Box::pin(self.deferred.settle())
    }
}

/// A reader from the app's pool, borrowed for one `read` at a time.
struct PooledReader<'a>(&'a campfire_db::Database);

impl twins::Reader for PooledReader<'_> {
    fn read(&self, read: &mut dyn FnMut(&Connection)) {
        let result = self.0.read_blocking(|conn| {
            read(conn);
            Ok(())
        });
        if let Err(error) = result {
            tracing::warn!(%error, "sync: deferred twin not read");
        }
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

    async fn open(
        &self,
        user: Arc<CableUser>,
        device_class: Option<api::DeviceClass>,
    ) -> Box<dyn SyncSession> {
        let mut session = Session {
            app: self.app.clone(),
            user,
            lease: None,
            device_class: device_class.map(dto::device_class),
            present: BTreeSet::new(),
        };
        if let Some(app) = session.app.upgrade() {
            session.establish(&app).await;
        }
        Box::new(session)
    }
}

/// One connection's person, the workspace presence lease it holds, and the rooms it's looking at.
pub struct Session {
    app: Weak<AppState>,
    user: Arc<CableUser>,
    lease: Option<WorkspacePresenceLease>,
    device_class: Option<campfire_db::DeviceClass>,
    present: BTreeSet<i64>,
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
                let Some(thread) = campfire_db::ChannelThread::find_by_id(conn, thread_id)? else {
                    return Ok(None);
                };
                if !thread.readable_by(conn, user_id)? {
                    return Ok(None);
                }
                thread.room_id
            }
        };
        Room::find_for_user(conn, user_id, room_id)
    }
}

impl Session {
    /// `WorkspacePresenceChannel#subscribed`: a lease for this connection.
    async fn establish(&mut self, app: &AppState) {
        let (user_id, session_id) = (self.user.id, self.user.session_id);
        let user = self.user.clone();
        let presence_app = self.app.clone();
        let device_class = self.device_class;
        match app
            .db
            .write(move |tx| {
                let lease = WorkspacePresenceLease::establish_for_device(
                    tx,
                    user_id,
                    session_id,
                    device_class,
                )?;
                if lease.is_some() {
                    publish_presence_after_commit(tx, presence_app, &user);
                }
                Ok(lease)
            })
            .await
        {
            Ok(lease) => self.lease = lease,
            Err(error) => tracing::warn!(%error, user_id, "sync: presence lease not established"),
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

fn publish_presence_after_commit(
    tx: &mut campfire_db::Tx<'_>,
    app: Weak<AppState>,
    user: &CableUser,
) {
    if !user.active_human() {
        return;
    }
    let user_id = user.id;
    tx.after_commit(move |after| {
        let Some(app) = app.upgrade() else {
            return Ok(());
        };
        if !app.cable.sync_wanted() {
            return Ok(());
        }
        match dto::presences(after.conn(), &[user_id], after.now()) {
            Ok(presences) => {
                for presence in presences {
                    app.cable.sync_presence(presence);
                }
            }
            Err(error) => tracing::warn!(%error, user_id, "sync: presence not read"),
        }
        Ok(())
    });
}

fn event_thread_id(event: &api::SyncEvent) -> Option<i64> {
    if matches!(event.payload, api::SyncPayload::ThreadRemoved(_)) {
        return None;
    }
    if let Some(id) = event
        .topic
        .strip_prefix("thread:")
        .and_then(|id| id.parse().ok())
    {
        return Some(id);
    }
    match &event.payload {
        api::SyncPayload::ThreadCreated(thread) | api::SyncPayload::ThreadUpdated(thread) => {
            Some(thread.id)
        }
        api::SyncPayload::ThreadIndicator(change) => {
            change.thread.as_ref().map(|thread| thread.thread_id)
        }
        api::SyncPayload::ThreadUnread(change) => Some(change.thread_id),
        api::SyncPayload::MessageNotified(notified) => notified.thread_id,
        api::SyncPayload::RoomFilesChanged(change) => Some(change.thread_id),
        api::SyncPayload::ThreadRead(change) => Some(change.thread_id),
        api::SyncPayload::ThreadGithubUpdated(change) => Some(change.thread_id),
        api::SyncPayload::MessageCreated(message) | api::SyncPayload::MessageUpdated(message) => {
            message.thread_id
        }
        api::SyncPayload::MessageRemoved(message) => message.thread_id,
        api::SyncPayload::MessageReactions(reactions) => reactions.thread_id,
        api::SyncPayload::MessageCards(cards) => cards.thread_id,
        api::SyncPayload::PollUpdated(poll) => poll.thread_id,
        api::SyncPayload::PollBallot(ballot) => ballot.thread_id,
        api::SyncPayload::AgentSteps(steps) => steps.thread_id,
        api::SyncPayload::ActivityItem(change) => {
            change.item.as_ref().and_then(|item| item.source.thread_id)
        }
        api::SyncPayload::BlocksUpdated(_)
        | api::SyncPayload::SettingsUpdated(_)
        | api::SyncPayload::UserUpdated(_)
        | api::SyncPayload::WorkspaceUpdated(_)
        | api::SyncPayload::WorkspaceStylesUpdated(_)
        | api::SyncPayload::Typing(_)
        | api::SyncPayload::RoomUnread(_)
        | api::SyncPayload::RoomRead(_)
        | api::SyncPayload::SidebarRowUpserted(_)
        | api::SyncPayload::SidebarRowRemoved(_)
        | api::SyncPayload::Presence(_)
        | api::SyncPayload::MessagePinned(_)
        | api::SyncPayload::ThreadRemoved(_)
        | api::SyncPayload::BoardAutomationsChanged(_)
        | api::SyncPayload::SavedChanged(_)
        | api::SyncPayload::ActivityRemoved(_)
        | api::SyncPayload::ScheduledChanged(_)
        | api::SyncPayload::ScheduledRemoved(_)
        | api::SyncPayload::SidebarCategoryUpserted(_)
        | api::SyncPayload::SidebarCategoryRemoved(_)
        | api::SyncPayload::EventsChanged(_)
        | api::SyncPayload::AgentStatus(_)
        | api::SyncPayload::ApprovalUpdated(_)
        | api::SyncPayload::HuddleAfkMoved(_)
        | api::SyncPayload::HuddlePresence(_)
        | api::SyncPayload::HuddleRole(_)
        | api::SyncPayload::HuddleNotice(_)
        | api::SyncPayload::HuddleRing(_)
        | api::SyncPayload::HuddleSoundboard(_)
        | api::SyncPayload::StageUpdated(_)
        | api::SyncPayload::StageStreamStopped(_)
        | api::SyncPayload::WorkspaceLayoutUpdated(_) => None,
    }
}

#[async_trait::async_trait]
impl SyncSession for Session {
    async fn events_for_viewer(&mut self, wires: &[&str]) -> Vec<String> {
        let events: Vec<_> = wires
            .iter()
            .filter_map(|wire| {
                serde_json::from_str::<api::SyncEvent>(wire)
                    .ok()
                    .map(|event| (*wire, event))
            })
            .collect();
        #[cfg(feature = "test-support")]
        if let Some(app) = self.app.upgrade() {
            crate::test_hooks::before_sync_delivery(app.db.path(), self.user.id, &events).await;
        }
        let mut room_ids = BTreeSet::new();
        let mut message_ids = BTreeSet::new();
        let mut thread_ids = BTreeSet::new();
        let mut pin_ids = BTreeSet::new();
        let mut scheduled_ids = BTreeSet::new();
        for (_, event) in &events {
            if let api::SyncPayload::ScheduledChanged(message) = &event.payload {
                scheduled_ids.insert(message.id);
            }
            if let api::SyncPayload::MessagePinned(state) = &event.payload {
                pin_ids.insert(state.message_id);
            }
            if let Some(id) = event_thread_id(event) {
                thread_ids.insert(id);
            }
            if let api::SyncPayload::MessageCreated(message)
            | api::SyncPayload::MessageUpdated(message) = &event.payload
            {
                room_ids.insert(message.room_id);
                message_ids.insert(message.id);
                if let Some(indicator) = &message.thread {
                    thread_ids.insert(indicator.thread_id);
                }
            }
        }
        if message_ids.is_empty()
            && thread_ids.is_empty()
            && pin_ids.is_empty()
            && scheduled_ids.is_empty()
        {
            return events
                .into_iter()
                .map(|(wire, _)| wire.to_owned())
                .collect();
        }
        let viewer = self.user.id;
        let access = if let Some(app) = self.app.upgrade() {
            let render_app = app.clone();
            app.db.read_snapshot(move |conn| {
                let messages = campfire_db::models::announcement::replay_access(
                    conn, viewer, room_ids, message_ids,
                )?;
                let predicate = campfire_db::models::channel_thread::readable_by_sql("channel_threads.id", "?2");
                let mut statement = conn.prepare_cached(&format!(
                    "SELECT id FROM channel_threads WHERE id IN (SELECT value FROM json_each(?1)) AND {predicate}"
                ))?;
                let readable = statement.query_map(
                    rusqlite::params![serde_json::json!(thread_ids).to_string(), viewer],
                    |row| row.get::<_, i64>(0),
                )?.collect::<rusqlite::Result<BTreeSet<_>>>()?;
                let mut pins = std::collections::HashMap::new();
                let pin_access = campfire_db::models::channel_thread::readable_by_sql("m.thread_id", "?2");
                let mut statement = conn.prepare_cached(&format!(
                    "SELECT m.id, m.room_id FROM messages m JOIN memberships mm ON mm.room_id=m.room_id AND mm.user_id=?2 JOIN rooms r ON r.id=m.room_id AND r.deleted_at IS NULL WHERE m.id IN (SELECT value FROM json_each(?1)) AND (m.thread_id IS NULL OR {pin_access})"
                ))?;
                let pin_messages = statement.query_map(
                    rusqlite::params![serde_json::json!(pin_ids).to_string(), viewer],
                    |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
                )?.collect::<rusqlite::Result<Vec<_>>>()?;
                let mut counts = std::collections::HashMap::new();
                for (message_id, room_id) in pin_messages {
                    let count = match counts.get(&room_id) {
                        Some(count) => *count,
                        None => {
                            let count = campfire_db::MessagePin::count_for_viewer(conn, room_id, viewer)?;
                            counts.insert(room_id, count);
                            count
                        },
                    };
                    pins.insert(message_id, (room_id, count));
                }
                let mut scheduled_rows = Vec::new();
                for id in scheduled_ids {
                    if let Some(row) = campfire_db::ScheduledMessage::find_by_id(conn, id)?
                        && row.user_id == viewer {
                        scheduled_rows.push(row);
                    }
                }
                let scheduled = crate::composer::scheduled_rows(conn, &scheduled_rows, render_app.db.env().now(), &render_app)?
                    .into_iter().map(|message| (message.id, message)).collect::<std::collections::HashMap<_, _>>();
                Ok((messages, readable, pins, scheduled))
            }).await.unwrap_or_else(|error| {
                tracing::warn!(%error, "sync: batch delivery not authorized");
                app.cable.sync_skipped_for(viewer);
                Default::default()
            })
        } else {
            Default::default()
        };
        events
            .into_iter()
            .filter_map(|(wire, mut event)| {
                // Removal is a control event: it must reach a revoked viewer to clear their pane
                // and unsubscribe even though their access has already gone.
                if let Some(id) = event_thread_id(&event)
                    && !access.1.contains(&id)
                {
                    return None;
                }
                match &mut event.payload {
                    api::SyncPayload::ScheduledChanged(message) => {
                        *message = access.3.get(&message.id)?.clone();
                        serde_json::to_string(&event).ok()
                    }
                    api::SyncPayload::MessagePinned(state) => {
                        let (room_id, count) = access.2.get(&state.message_id)?;
                        state.room_id = *room_id;
                        state.pin_count = *count;
                        serde_json::to_string(&event).ok()
                    }
                    api::SyncPayload::MessageCreated(message)
                    | api::SyncPayload::MessageUpdated(message) => {
                        if !access.0.rooms.contains(&message.room_id) {
                            return None;
                        }
                        if message
                            .thread
                            .as_ref()
                            .is_some_and(|indicator| !access.1.contains(&indicator.thread_id))
                        {
                            message.thread = None;
                        }
                        message.announcement_source =
                            access.0.sources.get(&message.id).map(|source| {
                                api::AnnouncementSource {
                                    room_id: source.room_id,
                                    message_id: source.message_id,
                                    room_name: source.room_name.clone(),
                                }
                            });
                        serde_json::to_string(&event).ok()
                    }
                    _ => Some(wire.to_owned()),
                }
            })
            .collect()
    }

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
        let Ok(Some((room, name))) = app
            .db
            .read(move |conn| {
                let Some(room) = conversation.room(conn, user_id)? else {
                    return Ok(None);
                };
                let name = campfire_db::User::find(conn, user_id)?
                    .display_name()
                    .to_owned();
                Ok(Some((room, name)))
            })
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
            "user": { "id": self.user.id, "name": name },
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
            app.broadcasts.sync_read_row(self.user.id, room_id);
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
        let user = self.user.clone();
        let presence_app = self.app.clone();
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
                        if refreshed || lease.destroyed {
                            publish_presence_after_commit(tx, presence_app, &user);
                        }
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
        if let Some(mut lease) = self.lease.take() {
            let user = self.user.clone();
            let presence_app = self.app.clone();
            if let Err(error) = app
                .db
                .write(move |tx| {
                    lease.delete(tx)?;
                    publish_presence_after_commit(tx, presence_app, &user);
                    Ok(())
                })
                .await
            {
                tracing::warn!(%error, user_id = self.user.id, "sync: presence lease not removed");
            }
        }
    }
}

/// The shared hierarchy in the same snapshot as the member's sidebar or live publication.
pub(crate) fn workspace_layout(
    conn: &Connection,
    user_id: i64,
) -> campfire_db::Result<api::WorkspaceLayout> {
    let viewer = campfire_db::User::find(conn, user_id)?;
    let rooms = campfire_db::WorkspaceCategory::visible_rooms(conn, user_id)?;
    Ok(api::WorkspaceLayout {
        categories: campfire_db::WorkspaceCategory::ordered(conn)?
            .into_iter()
            .filter(|category| {
                viewer.is_administrator()
                    || rooms
                        .iter()
                        .any(|room| room.workspace_category_id == Some(category.id))
            })
            .map(|row| api::WorkspaceCategory {
                id: row.id,
                name: row.name,
                position: row.position,
            })
            .collect(),
        rooms: rooms
            .into_iter()
            .map(|row| api::WorkspaceRoomPosition {
                room_id: row.room_id,
                workspace_category_id: row.workspace_category_id,
                position: row.position,
            })
            .collect(),
    })
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
