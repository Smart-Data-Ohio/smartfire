//! The JSON twins of the broadcasts, for the single-page app's `/api/v1/sync` socket
//! (`campfire_cable::sync`, `frontend-plan.md` §2.5). Each broadcast point that has a twin calls
//! here independently of HTML rendering.
//!
//! Payloads containing message HTML or viewer facts use the [`SyncRenderer`] installed at
//! boot. Ordinary events skip DTO work when no sync socket is open. Digest acknowledgements
//! and attachment completion still validate their DTOs while idle, preserving retry/claim
//! semantics independently of subscriptions.
//!
//! [`TWINS`] lists which broadcasts have twins and [`NOT_YET_TWINNED`] the ones still to port;
//! a test in the server crate fails when a broadcast is in neither.
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock, Weak};

pub use campfire_api_types::WorkspaceBranding;
use campfire_api_types::{
    BoardAutomationsChanged, CustomStyles, MessageCards, MessageDTO, MessageReactions, MessageRemoved, PinState,
    PollBallot, PollUpdated, Presence, RoomRead, RoomUnread, SavedChanged, SidebarRow,
    SidebarRowRemoved, SyncPayload, Thread, ThreadIndicator, ThreadIndicatorChanged, ThreadRead,
    ThreadRemoved, ThreadUnread, Typing, UserPresence,
};
use campfire_cable::sync::{Audience, SyncPublication};
use campfire_db::models::activity_item::ActivityItemsRemoved;
use campfire_db::{ChannelThread, Connection, Database, Membership, Message, Room, Snapshot};

use super::Cable;

/// Builds the twins that need the presenters, which live above this crate.
pub trait SyncRenderer: Send + Sync + 'static {
    /// The message as `GET /api/v1/rooms/:id/messages` serves it.
    fn message(&self, conn: &Connection, message: &Message) -> Option<MessageDTO>;
    /// A bounded group serialized from the same read snapshot.
    fn messages(&self, conn: &Connection, messages: &[Message]) -> Option<Vec<MessageDTO>> {
        messages.iter().map(|message| self.message(conn, message)).collect()
    }
    /// The message's reactions and boosts, as `POST /api/v1/messages/:id/boosts` answers them.
    fn reactions(&self, conn: &Connection, message: &Message) -> Option<MessageReactions>;
    /// The membership's sidebar row, or `None` when the room isn't in that sidebar (an
    /// invisible membership, a deleted room). Read in the snapshot the membership came from.
    fn sidebar_row(
        &self,
        conn: &Snapshot<'_>,
        room: &Room,
        membership: &Membership,
    ) -> campfire_db::Result<Option<SidebarRow>>;
    /// The thread as `GET /api/v1/threads/:id` serves it.
    fn thread(&self, conn: &Connection, thread: &ChannelThread) -> Option<Thread>;
    /// The reply indicator of `parent`'s thread: `Ok(None)` when it has none (any more).
    fn thread_indicator(
        &self,
        conn: &Connection,
        parent: &Message,
    ) -> campfire_db::Result<Option<ThreadIndicator>>;
    /// The viewer's inbox item as `GET /api/v1/activity` lists it, with their unread count:
    /// `Ok(None)` when they can't see it (any more).
    fn activity_item(
        &self,
        conn: &Connection,
        user_id: i64,
        item_id: i64,
    ) -> campfire_db::Result<Option<campfire_api_types::ActivityItemChanged>>;
    /// The scheduled message as `GET /api/v1/scheduled_messages` lists it; `Ok(None)` when it's
    /// gone.
    fn scheduled_message(
        &self,
        conn: &Connection,
        id: i64,
    ) -> campfire_db::Result<Option<campfire_api_types::ScheduledMessage>>;
    /// The agent's `agent.status`, with its working presence (for the recipients who share a
    /// room with it); `Ok(None)` when the agent is gone.
    fn agent_status(
        &self,
        conn: &Connection,
        agent_id: i64,
    ) -> campfire_db::Result<Option<campfire_api_types::AgentStatusChanged>>;
    /// A parent's steps (a message's, else a work thread's) as `agent.steps` carries them;
    /// `Ok(None)` when the parent is gone.
    fn agent_steps(
        &self,
        conn: &Connection,
        message_id: Option<i64>,
        thread_id: Option<i64>,
    ) -> campfire_db::Result<Option<campfire_api_types::AgentStepsChanged>>;
    /// The approval request as `user_id` sees it on its agent's approvals page; `Ok(None)` when
    /// they can't read that page (any more).
    fn approval_updated(
        &self,
        conn: &Connection,
        approval_id: i64,
        user_id: i64,
    ) -> campfire_db::Result<Option<campfire_api_types::ApprovalUpdated>>;
    /// The poll's `poll.updated`, and `voter_id`'s `poll.ballot` when given: `Ok(None)` when the
    /// poll is gone.
    fn poll(
        &self,
        conn: &Connection,
        poll_id: i64,
        voter_id: Option<i64>,
    ) -> campfire_db::Result<Option<(PollUpdated, Option<PollBallot>)>>;
    /// The messages' cards as `MessageDTO::cards` carries them, one `message.cards` each.
    fn message_cards(
        &self,
        conn: &Connection,
        messages: &[Message],
    ) -> campfire_db::Result<Vec<MessageCards>>;
    /// Runs `job` soon with a reader connection, off the caller's thread: for broadcast points
    /// that have no connection at hand (taking a second reader there could wait on the pool).
    fn defer(&self, job: Box<dyn FnOnce(&Connection) + Send>);
    /// Runs `job` soon off the caller's thread, holding no reader: it borrows one through the
    /// [`Reader`] only once it has the publication lock it may wait for (see [`room_rows`]), so
    /// jobs waiting on a lock never hold pooled readers the writer's own reads need.
    fn defer_unread(&self, job: UnreadJob);
    /// Waits for this renderer's deferred reads, so publication counts include late frames.
    #[cfg(feature = "test-support")]
    fn settle(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + '_>> {
        Box::pin(async {})
    }
}

/// A job [`SyncRenderer::defer_unread`] runs, borrowing readers as it goes.
pub type UnreadJob = Box<dyn FnOnce(&dyn Reader) + Send>;

/// Lends a deferred job a pooled reader for `read`, giving it back on return.
pub trait Reader {
    fn read(&self, read: &mut dyn FnMut(&Connection));
}

/// Where [`Broadcasts`](super::Broadcasts) finds the installed [`SyncRenderer`].
#[derive(Clone, Default)]
pub struct RendererSlot(Arc<RendererState>);

#[derive(Default)]
struct RendererState {
    renderer: OnceLock<Arc<dyn SyncRenderer>>,
    threads: PublicationLocks,
    rooms: PublicationLocks,
    /// Per parent message, for its `thread.indicator`.
    indicators: PublicationLocks,
}

/// One lock per thread or room while a publication holds it, so its deferred readers run one
/// at a time from read through publication.
type PublicationLocks = Mutex<HashMap<i64, Weak<Mutex<()>>>>;

fn publication_lock(locks: &PublicationLocks, id: i64) -> Arc<Mutex<()>> {
    let mut locks = locks.lock().unwrap_or_else(|error| error.into_inner());
    if let Some(lock) = locks.get(&id).and_then(Weak::upgrade) {
        return lock;
    }
    locks.retain(|_, lock| lock.strong_count() > 0);
    let lock = Arc::new(Mutex::new(()));
    locks.insert(id, Arc::downgrade(&lock));
    lock
}

/// Holds `lock` until dropped; a panicked holder doesn't wedge later publications.
fn hold(lock: &Mutex<()>) -> std::sync::MutexGuard<'_, ()> {
    lock.lock().unwrap_or_else(|error| error.into_inner())
}

impl RendererSlot {
    /// Only the first call takes effect.
    pub fn install(&self, renderer: Arc<dyn SyncRenderer>) {
        let _ = self.0.renderer.set(renderer);
    }

    #[cfg(feature = "test-support")]
    pub async fn settle(&self) {
        if let Some(renderer) = self.0.renderer.get() {
            renderer.settle().await;
        }
    }

    fn get(&self, server: &Cable) -> Option<&Arc<dyn SyncRenderer>> {
        server
            .sync_wanted()
            .then(|| self.0.renderer.get())
            .flatten()
    }

    fn thread_lock(&self, thread_id: i64) -> Arc<Mutex<()>> {
        publication_lock(&self.0.threads, thread_id)
    }

    /// The room's sidebar rows, as [`Self::thread_lock`] is the thread's.
    fn room_lock(&self, room_id: i64) -> Arc<Mutex<()>> {
        publication_lock(&self.0.rooms, room_id)
    }

    /// A parent message's thread indicator, as [`Self::thread_lock`] is the thread's.
    fn indicator_lock(&self, parent_message_id: i64) -> Arc<Mutex<()>> {
        publication_lock(&self.0.indicators, parent_message_id)
    }
}

/// The broadcast points and the sync events they produce. A point is named as the server
/// crate's coverage test finds it: a [`Broadcasts`](super::Broadcasts) method
/// (`Broadcasts::<name>`), a free function here (`broadcasts::<name>`), a channel, or a kind the
/// cable sink handles (its type's path, as the sink names it, without `campfire_db::models::`,
/// `campfire_db::` or `crate::integrations::`).
pub const TWINS: &[(&str, &[&str])] = &[
    ("workspace_branding::publish", &["workspace.updated"]),
    ("sync::workspace_styles_updated", &["workspace.styles.updated"]),
    (
        "Broadcasts::message_create",
        &["message.created", "room.unread", "sidebar.row.upserted"],
    ),
    (
        "Broadcasts::unread_room",
        &["room.unread", "sidebar.row.upserted"],
    ),
    (
        "Broadcasts::mark_room_unread",
        &["room.unread", "sidebar.row.upserted"],
    ),
    (
        "Broadcasts::message_remove",
        &["message.removed", "sidebar.row.upserted"],
    ),
    (
        "Broadcasts::message_replace",
        &["message.updated", "sidebar.row.upserted"],
    ),
    (
        "Broadcasts::message_reactions_replace",
        &["message.reactions"],
    ),
    ("Broadcasts::thread_refresh", &["thread.unread"]),
    ("Broadcasts::thread_created", &["thread.created"]),
    ("Broadcasts::thread_updated", &["thread.updated"]),
    (
        "Broadcasts::thread_removed",
        &["thread.removed", "sidebar.row.upserted"],
    ),
    (
        "Broadcasts::board_automations_changed",
        &["board.automations.changed"],
    ),
    ("Broadcasts::thread_read", &["thread.read", "sidebar.row.upserted"]),
    ("Broadcasts::room_remove", &["sidebar.row.removed"]),
    ("Broadcasts::open_room_create", &["sidebar.row.upserted"]),
    ("Broadcasts::open_room_update", &["sidebar.row.upserted"]),
    ("Broadcasts::closed_room_create", &["sidebar.row.upserted"]),
    ("Broadcasts::closed_room_update", &["sidebar.row.upserted"]),
    ("Broadcasts::direct_room_create", &["sidebar.row.upserted"]),
    (
        "Broadcasts::involvement_change",
        &["sidebar.row.upserted", "sidebar.row.removed"],
    ),
    // The joiner's new row, and every other member's row with `refreshRoom`.
    ("Broadcasts::joined_open_room", &["sidebar.row.upserted"]),
    ("broadcasts::read_room", &["room.read"]),
    ("messages::rendered::broadcast_tombstones", &["message.updated"]),
    ("link_embeds::broadcast_message", &["message.cards"]),
    // No classic frame: the sink publishes these for the single-page app only.
    (
        "scheduled_message::ScheduledMessageChange",
        &["scheduled.changed", "scheduled.removed"],
    ),
    ("activity_item::ActivityItemsRemoved", &["activity.removed"]),
    ("agent::AgentSyncChange", &["agent.status"]),
    ("agent_approval::ApprovalChange", &["approval.updated"]),
    ("channel_thread::ThreadBoardCreation", &["thread.created"]),
    ("channel_thread::ThreadWorkChange", &["thread.updated"]),
    ("activity_item::ActivityItemTouched", &["activity.item"]),
    ("membership::PresentRead", &["sidebar.row.upserted"]),
    (
        "room_category::SidebarOrganized",
        &[
            "sidebar.row.upserted",
            "sidebar.category.upserted",
            "sidebar.category.removed",
        ],
    ),
    ("poll::PollChanged", &["poll.updated", "poll.ballot"]),
    ("calendar_event::EventsChanged", &["events.changed"]),
    // The card slots' replaces after a fetch, a refresh or an event's change.
    ("calendar_event::CardUpdate", &["message.cards"]),
    ("link_embed::store::CardUpdate", &["message.cards"]),
    ("fizzy::cards::CardUpdate", &["message.cards"]),
    ("twitter::post::CardUpdate", &["message.cards"]),
    ("github::pull_requests::CardUpdated", &["message.cards", "thread.github.updated"]),
    ("TypingNotificationsChannel", &["typing"]),
    // Committed messages, pins, indicators, memberships, activity and read/unread effects.
    (
        "broadcasts::Broadcast",
        &[
            "activity.item",
            "message.created",
            "message.updated",
            "room.unread",
            "room.read",
            "message.pinned",
            "thread.unread",
            "thread.indicator",
            "thread.created",
            "thread.updated",
            "thread.removed",
            "sidebar.row.upserted",
            "huddle.notice",
            "huddle.ring",
            "message.cards",
        ],
    ),
    // `POST /api/v1/saved` and `DELETE /api/v1/saved/:id` publish to the person's other tabs.
    ("campfire_api::saved_items", &["saved.changed"]),
    ("huddle_effects::Presence", &["huddle.presence"]),
    (
        "huddle_effects::StreamChanged",
        &["stage.updated", "huddle.presence"],
    ),
    ("huddle_effects::StreamStopped", &["stage.stream.stopped"]),
    ("huddle_effects::StageRoster", &["stage.updated"]),
    ("huddle_effects::RoleEvent", &["huddle.role"]),
    ("huddle_effects::StageEndedNote", &["message.created"]),
    ("RoomRemovalBroadcast", &["sidebar.row.removed"]),
    (
        "user_status_settings::updates::StatusBadgeBroadcast",
        &["presence"],
    ),
    ("user::lifecycle::QuietStreamFinal", &["message.updated"]),
    ("board_automations::DigestNotes", &["message.created"]),
    ("github::notifier::MessageCreated", &["message.created", "room.unread"]),
    // The message or its thread's agent steps.
    ("agent_step::StepParentChange", &["agent.steps"]),
];

/// Broadcast points with no sync event yet: the SPA slices after S1 port them.
pub const NOT_YET_TWINNED: &[&str] = &[];

/// Sync events the contract defines that no broadcast point publishes yet: none since the S3 and
/// S4 server work. The coverage test fails when an event is in neither this list nor [`TWINS`],
/// or in both.
pub const NOT_YET_EMITTED: &[&str] = &[];

/// The conversation topic a message's events go to: its thread's, or its room's.
pub fn message_topic(message: &Message) -> String {
    match message.thread_id {
        Some(thread_id) => format!("thread:{thread_id}"),
        None => format!("room:{}", message.room_id),
    }
}

pub fn room_topic(room_id: i64) -> String {
    format!("room:{room_id}")
}

pub fn thread_topic(thread_id: i64) -> String {
    format!("thread:{thread_id}")
}

/// Publishes `payload` to `audience`. Serializes nothing while no sync socket is open.
pub fn publish(server: &Cable, audience: Audience, payload: &SyncPayload) {
    if server.sync_wanted() {
        send(server, audience, payload, |publication| publication);
    }
}

/// Publishes; the caller has checked [`Cable::sync_wanted`](campfire_cable::Server::sync_wanted).
fn send(
    server: &Cable,
    audience: Audience,
    payload: &SyncPayload,
    adjust: impl FnOnce(SyncPublication) -> SyncPublication,
) {
    let payload = serde_json::to_string(payload).expect("sync payloads serialize");
    server.sync_publish(adjust(SyncPublication::new(audience, payload)));
}

/// `message.created` or `message.updated` on the message's conversation.
pub fn message(
    server: &Cable,
    slot: &RendererSlot,
    conn: &Connection,
    message: &Message,
    created: bool,
) {
    let Some(renderer) = slot.get(server) else {
        return;
    };
    let Some(dto) = renderer.message(conn, message) else {
        return;
    };
    let payload = if created {
        SyncPayload::MessageCreated(dto)
    } else {
        SyncPayload::MessageUpdated(dto)
    };
    send(
        server,
        Audience::Topic(message_topic(message)),
        &payload,
        |publication| publication,
    );
}

/// Validate publication even without subscribers, before claims or held jobs are released.
pub fn checked_message(server: &Cable, slot: &RendererSlot, conn: &Connection, message: &Message, created: bool) -> bool {
    let Some(renderer) = slot.0.renderer.get() else { return false; };
    let Some(dto) = renderer.message(conn, message) else { return false; };
    let payload = if created { SyncPayload::MessageCreated(dto) } else { SyncPayload::MessageUpdated(dto) };
    publish(server, Audience::Topic(message_topic(message)), &payload);
    true
}

/// Serialize the entire batch before acknowledging any digest publication IDs.
pub fn checked_messages(server: &Cable, slot: &RendererSlot, conn: &Connection, messages: &[Message]) -> Option<Vec<i64>> {
    let renderer = slot.0.renderer.get()?;
    let dtos = renderer.messages(conn, messages)?;
    if dtos.len() != messages.len() { return None; }
    for (message, dto) in messages.iter().zip(dtos) {
        publish(server, Audience::Topic(message_topic(message)), &SyncPayload::MessageCreated(dto));
    }
    Some(messages.iter().map(|message| message.id).collect())
}

/// `message.updated`, read afresh later:
pub fn message_updated_later(server: &Cable, slot: &RendererSlot, message_id: i64) {
    let Some(renderer) = slot.get(server) else {
        return;
    };
    let (server, slot) = (server.downgrade(), slot.clone());
    renderer.defer(Box::new(move |conn| {
        let (Some(server), Ok(Some(found))) =
            (server.upgrade(), Message::find_by_id(conn, message_id))
        else {
            return;
        };
        message(&server, &slot, conn, &found, false);
    }));
}

/// `message.reactions` on the message's conversation, read afresh later (the boost
/// broadcasts have no connection at hand).
pub fn message_reactions_later(server: &Cable, slot: &RendererSlot, message_id: i64) {
    let Some(renderer) = slot.get(server) else {
        return;
    };
    let (server, slot) = (server.downgrade(), slot.clone());
    renderer.defer(Box::new(move |conn| {
        let (Some(server), Ok(Some(found))) =
            (server.upgrade(), Message::find_by_id(conn, message_id))
        else {
            return;
        };
        let Some(renderer) = slot.get(&server) else {
            return;
        };
        if let Some(reactions) = renderer.reactions(conn, &found) {
            send(
                &server,
                Audience::Topic(message_topic(&found)),
                &SyncPayload::MessageReactions(reactions),
                |publication| publication,
            );
        }
    }));
}

/// `message.pinned` on the message's room: the pin badge's twin, sent as the badge is
/// replaced. Nothing for a message that's gone (its `message.removed` says so).
pub fn message_pinned(server: &Cable, conn: &Connection, message_id: i64) {
    if !server.sync_wanted() {
        return;
    }
    let state = (|| -> campfire_db::Result<Option<PinState>> {
        let Some(message) = Message::find_by_id(conn, message_id)? else {
            return Ok(None);
        };
        Ok(Some(PinState {
            message_id,
            room_id: message.room_id,
            pinned: campfire_db::MessagePin::pinned(conn, message_id)?,
            pin_count: campfire_db::MessagePin::count_for_room(conn, message.room_id)?,
        }))
    })();
    match state {
        Ok(Some(state)) => send(
            server,
            Audience::Topic(room_topic(state.room_id)),
            &SyncPayload::MessagePinned(state),
            |publication| publication,
        ),
        Ok(None) => {}
        Err(error) => tracing::warn!(%error, message_id, "sync: pin state not read"),
    }
}

/// `saved.changed` on the person's `user` topic: they saved (`Some` item, as it is now) or
/// unsaved a message.
pub fn saved_changed(
    server: &Cable,
    user_id: i64,
    message_id: i64,
    item: Option<campfire_api_types::SavedItem>,
) {
    if !server.sync_wanted() {
        return;
    }
    send(
        server,
        Audience::User(user_id),
        &SyncPayload::SavedChanged(SavedChanged { message_id, item }),
        |publication| publication,
    );
}

/// `message.removed` on the message's conversation.
pub fn message_removed(server: &Cable, message: &Message) {
    if !server.sync_wanted() {
        return;
    }
    let payload = SyncPayload::MessageRemoved(MessageRemoved {
        id: message.id,
        room_id: message.room_id,
        thread_id: message.thread_id,
    });
    send(
        server,
        Audience::Topic(message_topic(message)),
        &payload,
        |publication| publication,
    );
}

/// `room.unread` on the person's `user` topic.
pub fn room_unread(
    server: &Cable,
    user_id: i64,
    room_id: i64,
    message_id: Option<i64>,
    mentioned: bool,
) {
    if !server.sync_wanted() {
        return;
    }
    let payload = SyncPayload::RoomUnread(RoomUnread {
        room_id,
        message_id,
        mentioned,
    });
    send_unread_change(server, user_id, room_id, &payload);
}

/// The twin of a `Broadcast::UnreadRoom` the database layer emits: `room.unread` with the message
/// that made the room unread, and whether it mentions the person (read from `db` when there is
/// one; without a database, unmentioned).
pub fn unread_room(
    server: &Cable,
    db: Option<&Database>,
    user_id: i64,
    room_id: i64,
    message_id: Option<i64>,
) {
    if !server.sync_wanted() {
        return;
    }
    let mentioned = match (db, message_id) {
        (Some(db), Some(message_id)) => {
            let rich_text = db.env().rich_text.clone();
            let mentioned = db.read_blocking(move |conn| {
                let message = Message::find(conn, message_id)?;
                let mentionees = message.mentionees(conn, &*rich_text)?;
                Ok(mentionees.iter().any(|user| user.id == user_id))
            });
            match mentioned {
                Ok(mentioned) => mentioned,
                Err(error) => {
                    return tracing::warn!(%error, message_id, "sync: unread message not read");
                }
            }
        }
        _ => false,
    };
    room_unread(server, user_id, room_id, message_id, mentioned);
}

/// `room.read` on the person's `user` topic.
pub fn room_read(server: &Cable, user_id: i64, room_id: i64) {
    if !server.sync_wanted() {
        return;
    }
    send_unread_change(
        server,
        user_id,
        room_id,
        &SyncPayload::RoomRead(RoomRead { room_id }),
    );
}

/// The fence for a person's unread state in a room: `room.read` and `room.unread` change it on
/// the client, and a sidebar row read before one of them carries the state from before it.
fn unread_fence(user_id: i64, room_id: i64) -> String {
    format!("unread:{user_id}:{room_id}")
}

/// Publishes a change to the person's unread state of the room on their `user` topic, fencing
/// it: a row read before it is refused and read again ([`RoomRows::row`]), so no row can undo
/// the change on the client however long its publication waited.
fn send_unread_change(server: &Cable, user_id: i64, room_id: i64, payload: &SyncPayload) {
    let payload = serde_json::to_string(payload).expect("sync payloads serialize");
    server.sync_publish_fencing(
        SyncPublication::new(Audience::User(user_id), payload),
        unread_fence(user_id, room_id),
    );
}

/// The twin of a `Broadcast::Cable` the database layer emits, by its stream: unread and read
/// pings, and the huddle notices and invitations (`huddle_sync::cable_stream`). Anything else
/// has none.
pub fn cable_stream(server: &Cable, stream: &str, payload: &serde_json::Value) {
    if !server.sync_wanted() || super::huddle_sync::cable_stream(server, stream, payload) {
        return;
    }
    let user_id = |suffix: &str| {
        stream
            .strip_prefix("user_")?
            .strip_suffix(suffix)?
            .parse::<i64>()
            .ok()
    };
    let id = |key: &str| payload.get(key).and_then(serde_json::Value::as_i64);
    if let (Some(user_id), Some(room_id)) = (user_id("_unreads"), id("roomId")) {
        room_unread(server, user_id, room_id, None, false);
    } else if let (Some(user_id), Some(room_id)) = (user_id("_reads"), id("room_id")) {
        room_read(server, user_id, room_id);
    } else if let (Some(user_id), Some(thread_id), Some(room_id)) =
        (user_id("_unread_threads"), id("threadId"), id("roomId"))
    {
        let refresh_only = payload
            .get("refreshOnly")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        thread_unread(server, user_id, thread_id, room_id, refresh_only);
    }
}

/// `activity.item` on the owner's `user` topic: the twin of `ActivityChannel`'s
/// `{activityItemId}` frame on `user_<id>_activity` (a huddle ring's frame carries the id too).
/// The frame is sent after commit, so the item is read afresh; one the owner can't see (any
/// more) publishes nothing.
pub fn activity_stream(
    server: &Cable,
    slot: &RendererSlot,
    stream: &str,
    payload: &serde_json::Value,
) {
    let (Some(user_id), Some(item_id)) = (
        stream
            .strip_prefix("user_")
            .and_then(|rest| rest.strip_suffix("_activity"))
            .and_then(|id| id.parse::<i64>().ok()),
        payload
            .get("activityItemId")
            .and_then(serde_json::Value::as_i64)
            .filter(|id| *id != 0),
    ) else {
        return;
    };
    activity_item_later(server, slot, user_id, item_id);
}

/// `activity.item` for the owner's item `item_id`, read afresh later: for an `ActivityChannel`
/// frame, or a change the classic inbox doesn't hear (a grouped item re-pointed at a newer
/// reply while still unread). Only read for an owner with a sync socket open; others get a gap
/// marker, so a resume of theirs refetches.
pub fn activity_item_later(server: &Cable, slot: &RendererSlot, user_id: i64, item_id: i64) {
    let Some(renderer) = slot.get(server) else {
        return;
    };
    if !server.sync_connected(user_id) {
        server.sync_skipped_for(user_id);
        return;
    }
    let (server, slot) = (server.downgrade(), slot.clone());
    renderer.defer(Box::new(move |conn| {
        let Some(server) = server.upgrade() else {
            return;
        };
        let Some(renderer) = slot.get(&server) else {
            return;
        };
        match renderer.activity_item(conn, user_id, item_id) {
            Ok(Some(changed)) => send(
                &server,
                Audience::User(user_id),
                &SyncPayload::ActivityItem(changed),
                |publication| publication,
            ),
            Ok(None) => {}
            Err(error) => tracing::warn!(%error, item_id, "sync: activity item not read"),
        }
        // A message's item counts toward its room's red pill (`notificationCount`): a thread
        // reply pings without making the room unread, and reading the item clears it.
        // Read afresh later, under the room's lock, as every row is ([`RoomRows::publish`]).
        match item_room_id(conn, item_id) {
            Ok(Some(room_id)) => {
                sidebar_rows_later(&server, &slot, room_id, Some(vec![user_id]));
            }
            Ok(None) => {}
            Err(error) => tracing::warn!(%error, item_id, "sync: activity item's room not read"),
        }
    }));
}

/// The room of the message an activity item is about, if it's about a message.
fn item_room_id(conn: &Connection, item_id: i64) -> campfire_db::Result<Option<i64>> {
    Ok(conn
        .prepare_cached(
            r#"SELECT "messages"."room_id" FROM "activity_items" INNER JOIN "messages" ON "messages"."id" = "activity_items"."source_id" WHERE "activity_items"."id" = ? AND "activity_items"."source_type" = 'Message'"#,
        )?
        .query_map([item_id], |row| row.get(0))?
        .next()
        .transpose()?)
}

/// `activity.removed` on each owner's `user` topic, with their unread count afterwards.
/// Removing a mention from a retained message also refreshes its room's sidebar counts.
pub fn activity_removed_later(server: &Cable, slot: &RendererSlot, removed: ActivityItemsRemoved) {
    let Some(renderer) = slot.get(server) else {
        return;
    };
    // Only owners with a sync socket open; the others get a gap marker.
    let items = removed
        .items
        .into_iter()
        .filter(|&(_, user_id)| {
            let connected = server.sync_connected(user_id);
            if !connected {
                server.sync_skipped_for(user_id);
            }
            connected
        })
        .collect::<Vec<_>>();
    if items.is_empty() {
        return;
    }
    let (server, slot) = (server.downgrade(), slot.clone());
    renderer.defer(Box::new(move |conn| {
        let Some(server) = server.upgrade() else {
            return;
        };
        for (id, user_id) in items {
            let count = campfire_db::ActivityItem::unread_snapshot(conn, user_id);
            match count {
                Ok(unread) => send(
                    &server,
                    Audience::User(user_id),
                    &SyncPayload::ActivityRemoved(campfire_api_types::ActivityItemRemoved {
                        id,
                        unread_count: unread.count,
                        unread_revision: unread.revision,
                    }),
                    |publication| publication,
                ),
                Err(error) => tracing::warn!(%error, id, "sync: activity count not read"),
            }
            if let Some(room_id) = removed.room_id {
                sidebar_rows_later(&server, &slot, room_id, Some(vec![user_id]));
            }
        }
    }));
}

/// `scheduled.changed` (read afresh) or `scheduled.removed` on the author's `user` topic.
pub fn scheduled_later(
    server: &Cable,
    slot: &RendererSlot,
    change: campfire_db::models::scheduled_message::ScheduledMessageChange,
) {
    if !server.sync_wanted() {
        return;
    }
    if !server.sync_connected(change.user_id) {
        server.sync_skipped_for(change.user_id);
        return;
    }
    if change.removed {
        send(
            server,
            Audience::User(change.user_id),
            &SyncPayload::ScheduledRemoved(campfire_api_types::ScheduledMessageRemoved {
                id: change.id,
                room_id: change.room_id,
            }),
            |publication| publication,
        );
        return;
    }
    let Some(renderer) = slot.get(server) else {
        return;
    };
    let (server, slot) = (server.downgrade(), slot.clone());
    renderer.defer(Box::new(move |conn| {
        let Some(server) = server.upgrade() else {
            return;
        };
        let Some(renderer) = slot.get(&server) else {
            return;
        };
        match renderer.scheduled_message(conn, change.id) {
            Ok(Some(row)) => send(
                &server,
                Audience::User(change.user_id),
                &SyncPayload::ScheduledChanged(row),
                |publication| publication,
            ),
            Ok(None) => {}
            Err(error) => {
                tracing::warn!(%error, id = change.id, "sync: scheduled message not read")
            }
        }
    }));
}

/// `events.changed` on the room's topic: one of its events was scheduled, edited, cancelled or
/// removed. Carries only the room, so it needs no renderer or read.
pub fn events_changed(server: &Cable, change: campfire_db::models::calendar_event::EventsChanged) {
    publish(
        server,
        Audience::Topic(room_topic(change.room_id)),
        &SyncPayload::EventsChanged(campfire_api_types::EventsChanged {
            room_id: change.room_id,
        }),
    );
}

/// A vote or a close, read afresh later: `poll.updated` on the poll message's conversation, and
/// the voter's `poll.ballot` on their `user` topic. Without a connection the voter gets a gap
/// marker instead, and their next resume refetches.
pub fn poll_later(
    server: &Cable,
    slot: &RendererSlot,
    change: campfire_db::models::poll::PollChanged,
) {
    let Some(renderer) = slot.get(server) else {
        return;
    };
    let voter_id = change.voter_id.filter(|&voter_id| {
        let connected = server.sync_connected(voter_id);
        if !connected {
            server.sync_skipped_for(voter_id);
        }
        connected
    });
    let (server, slot) = (server.downgrade(), slot.clone());
    renderer.defer(Box::new(move |conn| {
        let Some(server) = server.upgrade() else {
            return;
        };
        let Some(renderer) = slot.get(&server) else {
            return;
        };
        let (updated, ballot) = match renderer.poll(conn, change.poll_id, voter_id) {
            Ok(Some(found)) => found,
            Ok(None) => return,
            Err(error) => {
                return tracing::warn!(%error, poll_id = change.poll_id, "sync: poll not read");
            }
        };
        let topic = match updated.thread_id {
            Some(thread_id) => thread_topic(thread_id),
            None => room_topic(updated.room_id),
        };
        send(
            &server,
            Audience::Topic(topic),
            &SyncPayload::PollUpdated(updated),
            |publication| publication,
        );
        if let (Some(voter_id), Some(ballot)) = (voter_id, ballot) {
            send(
                &server,
                Audience::User(voter_id),
                &SyncPayload::PollBallot(ballot),
                |publication| publication,
            );
        }
    }));
}

/// `message.cards` on each message's conversation: the twin of a card slot's replace.
pub fn message_cards(server: &Cable, slot: &RendererSlot, conn: &Connection, messages: &[Message]) {
    let Some(renderer) = slot.get(server) else {
        return;
    };
    let cards = match renderer.message_cards(conn, messages) {
        Ok(cards) => cards,
        Err(error) => return tracing::warn!(%error, "sync: message cards not rendered"),
    };
    for cards in cards {
        let topic = match cards.thread_id {
            Some(thread_id) => thread_topic(thread_id),
            None => room_topic(cards.room_id),
        };
        send(
            server,
            Audience::Topic(topic),
            &SyncPayload::MessageCards(cards),
            |publication| publication,
        );
    }
}

/// `thread.unread` on the member's `user` topic: the twin of `user_<id>_unread_threads`.
pub fn thread_unread(
    server: &Cable,
    user_id: i64,
    thread_id: i64,
    room_id: i64,
    refresh_only: bool,
) {
    if !server.sync_wanted() {
        return;
    }
    let payload = SyncPayload::ThreadUnread(ThreadUnread {
        thread_id,
        room_id,
        refresh_only,
    });
    send(server, Audience::User(user_id), &payload, |publication| {
        publication
    });
}

/// `thread.read` on the person's `user` topic: they read the thread (new: no Turbo frame).
pub fn thread_read(server: &Cable, user_id: i64, thread_id: i64, room_id: i64) {
    if !server.sync_wanted() {
        return;
    }
    let payload = SyncPayload::ThreadRead(ThreadRead { thread_id, room_id });
    send(server, Audience::User(user_id), &payload, |publication| {
        publication
    });
}

/// `thread.indicator` on the parent's room, for the indicator replace of `parent_message_id`;
/// then, while the thread is there, `thread.updated` with its new count and activity. Nothing
/// for a parent that's gone (its `message.removed` says so).
///
/// The cable sink calls this on the database writer, so it only queues: later, the parent's
/// indicator lock is taken, then a reader, and the indicator is read and published under both
/// (as [`thread_changed_later`] does for the thread), so a slower snapshot can't publish an older
/// count last. The `thread.updated` is queued after the indicator goes out.
pub fn thread_indicator_later(server: &Cable, slot: &RendererSlot, parent_message_id: i64) {
    let Some(renderer) = slot.get(server) else {
        return;
    };
    let (server, slot) = (server.downgrade(), slot.clone());
    renderer.defer_unread(Box::new(move |reader| {
        let Some(server) = server.upgrade() else {
            return;
        };
        let Some(renderer) = slot.get(&server) else {
            return;
        };
        let lock = slot.indicator_lock(parent_message_id);
        let publication = hold(&lock);
        let mut thread_id = None;
        reader.read(&mut |conn| {
            thread_id = thread_indicator(&server, renderer.as_ref(), conn, parent_message_id);
        });
        drop(publication);
        if let Some(thread_id) = thread_id {
            thread_changed_later(&server, &slot, thread_id, false);
        }
    }));
}

/// Publishes the parent's `thread.indicator`, returning the thread it shows.
fn thread_indicator(
    server: &Cable,
    renderer: &dyn SyncRenderer,
    conn: &Connection,
    parent_message_id: i64,
) -> Option<i64> {
    let parent = match Message::find_by_id(conn, parent_message_id) {
        Ok(Some(parent)) => parent,
        Ok(None) => return None,
        Err(error) => {
            tracing::warn!(%error, parent_message_id, "sync: thread indicator not read");
            return None;
        }
    };
    let indicator = match renderer.thread_indicator(conn, &parent) {
        Ok(indicator) => indicator,
        Err(error) => {
            tracing::warn!(%error, parent_message_id, "sync: thread indicator not rendered");
            return None;
        }
    };
    let thread_id = indicator.as_ref().map(|indicator| indicator.thread_id);
    send(
        server,
        Audience::Topic(room_topic(parent.room_id)),
        &SyncPayload::ThreadIndicator(ThreadIndicatorChanged {
            room_id: parent.room_id,
            parent_message_id,
            thread: indicator,
        }),
        |publication| publication,
    );
    thread_id
}

/// `thread.created` on the thread's room, or `thread.updated` on its room and its own topic.
/// The caller holds the thread's lock and the reader `conn`, taken in that order (see
/// [`thread_changed_later`]).
fn thread_changed(
    server: &Cable,
    renderer: &dyn SyncRenderer,
    conn: &Connection,
    thread_id: i64,
    created: bool,
) {
    let thread = match ChannelThread::find_by_id(conn, thread_id) {
        Ok(Some(thread)) => thread,
        Ok(None) => return,
        Err(error) => return tracing::warn!(%error, thread_id, "sync: thread not read"),
    };
    let Some(dto) = renderer.thread(conn, &thread) else {
        return;
    };
    if created {
        let payload = SyncPayload::ThreadCreated(dto);
        send(
            server,
            Audience::Topic(room_topic(thread.room_id)),
            &payload,
            |publication| publication,
        );
        return;
    }
    let payload = SyncPayload::ThreadUpdated(dto);
    for topic in [room_topic(thread.room_id), thread_topic(thread.id)] {
        send(server, Audience::Topic(topic), &payload, |publication| {
            publication
        });
    }
}

/// [`thread_changed`], read afresh later. Deferred readers can render an older snapshot more
/// slowly than a newer one, so each takes the thread's lock and reads through publication; a late
/// reader cannot send stale state last. The lock comes first and the reader only once it is held
/// (the order [`room_rows`] keeps for rooms): waiting holds no reader, so however many updates
/// queue behind a slow render, the pool stays free for the database writer's reads. No
/// publication holds a room's lock and a thread's together.
pub fn thread_changed_later(server: &Cable, slot: &RendererSlot, thread_id: i64, created: bool) {
    let Some(renderer) = slot.get(server) else {
        return;
    };
    let (server, slot) = (server.downgrade(), slot.clone());
    renderer.defer_unread(Box::new(move |reader| {
        let Some(server) = server.upgrade() else {
            return;
        };
        let Some(renderer) = slot.get(&server) else {
            return;
        };
        let lock = slot.thread_lock(thread_id);
        let _publication = hold(&lock);
        reader.read(&mut |conn| {
            thread_changed(&server, renderer.as_ref(), conn, thread_id, created);
        });
    }));
}

/// `thread.removed` on the thread's room and its own topic, whose followers then stop following
/// it.
pub fn thread_removed(server: &Cable, thread_id: i64, room_id: i64) {
    if !server.sync_wanted() {
        return;
    }
    let payload = SyncPayload::ThreadRemoved(ThreadRemoved { thread_id, room_id });
    send(
        server,
        Audience::Topic(room_topic(room_id)),
        &payload,
        |publication| publication,
    );
    send(
        server,
        Audience::Topic(thread_topic(thread_id)),
        &payload,
        |publication| SyncPublication {
            unsubscribe: Some(thread_topic(thread_id)),
            ..publication
        },
    );
}

/// `board.automations.changed` on the board's room topic, which only its members follow.
pub fn board_automations_changed(server: &Cable, room_id: i64) {
    if !server.sync_wanted() {
        return;
    }
    let payload = SyncPayload::BoardAutomationsChanged(BoardAutomationsChanged { room_id });
    send(
        server,
        Audience::Topic(room_topic(room_id)),
        &payload,
        |publication| publication,
    );
}

/// Who a room's row publication is for.
#[derive(Clone, Copy)]
enum Viewers<'a> {
    /// The room's members as they are now.
    Members,
    /// These people: a row for each who still belongs, `sidebar.row.removed` for any who don't.
    Users(&'a [i64]),
}

/// Proof that the room's lock is held: only [`room_locked`] makes one, for as long as it holds
/// the lock, and every sidebar row (upsert or removal) is published through it.
struct RoomRows {
    room_id: i64,
}

/// Runs `publish` holding the room's lock and nothing else: no reader is held while waiting.
fn room_locked<T>(slot: &RendererSlot, room_id: i64, publish: impl FnOnce(&RoomRows) -> T) -> T {
    let lock = slot.room_lock(room_id);
    let _publication = hold(&lock);
    publish(&RoomRows { room_id })
}

/// Runs `publish` holding the room's lock, then a reader borrowed once the lock is held (and
/// given back before the lock is), always in that order. Deferred readers run concurrently, and
/// a slower one could otherwise publish an older snapshot after a newer one (see
/// `thread_changed`): under the lock, each publication reads its inputs afresh and goes out
/// before the next one reads. Waiting for the lock holds no reader, so however many
/// publications queue behind a slow one, the pool stays free for the database writer's reads.
fn room_rows(
    slot: &RendererSlot,
    reader: &dyn Reader,
    room_id: i64,
    publish: impl FnOnce(&RoomRows, &Connection),
) {
    room_locked(slot, room_id, |rows| {
        let mut publish = Some(publish);
        reader.read(&mut |conn| {
            if let Some(publish) = publish.take() {
                publish(rows, conn);
            }
        });
    });
}

/// Runs `job` later on a blocking thread with no reader held: every row publication starts here,
/// then takes the room's lock before it borrows a reader (see [`room_rows`]).
fn defer_rows(
    server: &Cable,
    slot: &RendererSlot,
    job: impl FnOnce(&Cable, &RendererSlot, &dyn Reader) + Send + 'static,
) {
    let Some(renderer) = slot.get(server) else {
        return;
    };
    let (server, slot) = (server.downgrade(), slot.clone());
    renderer.defer_unread(Box::new(move |reader| {
        if let Some(server) = server.upgrade() {
            job(&server, &slot, reader);
        }
    }));
}

/// How long a sidebar row pass may keep its snapshot open before it is logged.
const SLOW_SNAPSHOT: std::time::Duration = std::time::Duration::from_millis(250);

/// Runs `read` in one read transaction ([`Snapshot`]; the pooled readers otherwise commit each
/// statement on its own), so everything it reads comes from one SQLite snapshot, taken at its
/// first read: no read in it can see a commit an earlier one predates. `None` (logged) when the
/// transaction can't be opened; nothing is read then.
///
/// A pass that keeps its snapshot open longer than [`SLOW_SNAPSHOT`] is logged: a long read
/// transaction holds back WAL checkpoints for as long as it lasts.
fn in_snapshot<T>(
    conn: &Connection,
    room_id: i64,
    read: impl FnOnce(&Snapshot<'_>) -> T,
) -> Option<T> {
    match Snapshot::begin(conn) {
        // Dropped once read, rolling back a transaction that wrote nothing.
        Ok(snapshot) => {
            let started = std::time::Instant::now();
            let read = read(&snapshot);
            let took = started.elapsed();
            if took > SLOW_SNAPSHOT {
                tracing::warn!(?took, room_id, "sync: slow sidebar snapshot");
            }
            Some(read)
        }
        Err(error) => {
            tracing::warn!(%error, room_id, "sync: sidebar snapshot not opened");
            None
        }
    }
}

impl RoomRows {
    /// The one way a sidebar row is published: the room, the memberships and each row are all
    /// read now, under the lock, from one snapshot. Each viewer gets their row
    /// (`sidebar.row.upserted`), a removal that keeps them following when they hid the room, or
    /// a removal that stops it when they no longer belong (no membership, or the room is gone or
    /// deleted).
    ///
    /// One snapshot, so a row only shows what its membership could see: a membership read
    /// before a revocation renders the room as it was before it, never a message posted after
    /// it. A row refused for a newer unread change (see [`Self::row`]) is read again from a
    /// fresh snapshot, membership and all; nothing read in one snapshot is used in another.
    ///
    /// Only members with a sync socket open get a row: a row costs a few reads, and a big room
    /// has many members. For the others the ring records a gap on their `user` topic, so their
    /// next resume refetches the sidebar. A row that can't be read is skipped (and logged), not
    /// taken for gone.
    fn publish(
        &self,
        server: &Cable,
        slot: &RendererSlot,
        conn: &Connection,
        viewers: Viewers<'_>,
        refresh_room: Option<bool>,
    ) {
        const TRIES: usize = 8;
        if !server.sync_wanted() {
            return;
        }
        let renderer = slot.get(server);
        // Before anything is read: a row is published only if no change to its unread state was
        // published since (see `row`).
        let since = server.sync_head();
        let Some(mut refused) = in_snapshot(conn, self.room_id, |conn| {
            self.rows(server, renderer, conn, viewers, since, refresh_room)
        }) else {
            return;
        };
        for _ in 1..TRIES {
            if refused.is_empty() {
                return;
            }
            let since = server.sync_head();
            let Some(again) = in_snapshot(conn, self.room_id, |conn| {
                self.rows(
                    server,
                    renderer,
                    conn,
                    Viewers::Users(&refused),
                    since,
                    refresh_room,
                )
            }) else {
                return;
            };
            refused = again;
        }
        // Their unread state keeps changing faster than a row renders: they refetch the sidebar.
        for user_id in refused {
            server.sync_skipped_for(user_id);
        }
    }

    /// One pass of [`Self::publish`], inside one snapshot: returns who had their row refused.
    fn rows(
        &self,
        server: &Cable,
        renderer: Option<&Arc<dyn SyncRenderer>>,
        conn: &Snapshot<'_>,
        viewers: Viewers<'_>,
        since: u64,
        refresh_room: Option<bool>,
    ) -> Vec<i64> {
        let room_id = self.room_id;
        let room = match Room::find_by_id(conn, room_id) {
            Ok(room) => room,
            Err(error) => {
                tracing::warn!(%error, room_id, "sync: sidebar room not read");
                return Vec::new();
            }
        };
        let memberships = match &room {
            Some(_) => match Membership::for_room(conn, room_id) {
                Ok(memberships) => memberships,
                Err(error) => {
                    tracing::warn!(%error, room_id, "sync: sidebar rows not read");
                    return Vec::new();
                }
            },
            None => Vec::new(),
        };
        let mut refused = Vec::new();
        let mut row = |membership: &Membership| match (&room, renderer) {
            (Some(room), Some(renderer)) => {
                let published = self.row(
                    server,
                    renderer.as_ref(),
                    conn,
                    (room, membership),
                    since,
                    refresh_room,
                );
                if !published {
                    refused.push(membership.user_id);
                }
            }
            _ => self.removed(server, membership.user_id),
        };
        match viewers {
            Viewers::Members => memberships.iter().for_each(&mut row),
            Viewers::Users(user_ids) => {
                for &user_id in user_ids {
                    match memberships.iter().find(|m| m.user_id == user_id) {
                        Some(membership) => row(membership),
                        None => self.removed(server, user_id),
                    }
                }
            }
        }
        refused
    }

    /// The person's row, read from `membership` in the caller's snapshot, which started after
    /// `since` (a [`Cable::sync_head`](campfire_cable::Server::sync_head)). A `room.read` or
    /// `room.unread` of theirs published after that has already changed the row's unread state
    /// on the client, and this row may predate it: the publication is refused, and `false` asks
    /// the caller to read it again from a fresh snapshot. `true` otherwise (published, hidden,
    /// skipped, or not rendered).
    fn row(
        &self,
        server: &Cable,
        renderer: &dyn SyncRenderer,
        conn: &Snapshot<'_>,
        (room, membership): (&Room, &Membership),
        since: u64,
        refresh_room: Option<bool>,
    ) -> bool {
        let user_id = membership.user_id;
        if room.deleted() {
            self.removed(server, user_id);
            return true;
        }
        if !server.sync_connected(user_id) {
            server.sync_skipped_for(user_id);
            return true;
        }
        let mut row = match renderer.sidebar_row(conn, room, membership) {
            Ok(Some(row)) => row,
            Ok(None) => {
                self.hidden(server, user_id, refresh_room);
                return true;
            }
            Err(error) => {
                tracing::warn!(
                    %error,
                    room_id = room.id,
                    user_id,
                    "sync: sidebar row not rendered"
                );
                return true;
            }
        };
        row.refresh_room = refresh_room;
        let payload = serde_json::to_string(&SyncPayload::SidebarRowUpserted(row))
            .expect("sync payloads serialize");
        let publication = SyncPublication::new(Audience::User(user_id), payload);
        server
            .sync_publish_fresh(publication, &unread_fence(user_id, room.id), since)
            .is_ok()
    }

    /// The person no longer belongs: `sidebar.row.removed` on their `user` topic, and their
    /// connections stop following the room.
    fn removed(&self, server: &Cable, user_id: i64) {
        let room_id = self.room_id;
        let payload = SyncPayload::SidebarRowRemoved(SidebarRowRemoved {
            room_id,
            refresh_room: Some(true),
        });
        send(server, Audience::User(user_id), &payload, |publication| {
            SyncPublication {
                unsubscribe: Some(room_topic(room_id)),
                ..publication
            }
        });
    }

    /// The person hid the room (an invisible membership): `sidebar.row.removed` on their `user`
    /// topic, while their connections go on following it.
    fn hidden(&self, server: &Cable, user_id: i64, refresh_room: Option<bool>) {
        let payload = SyncPayload::SidebarRowRemoved(SidebarRowRemoved {
            room_id: self.room_id,
            refresh_room,
        });
        send(server, Audience::User(user_id), &payload, |publication| {
            publication
        });
    }
    /// The room is gone: `sidebar.row.removed` for everyone, and nobody follows it any more.
    /// Sent to everyone rather than to members read now, since the records may be gone.
    fn removed_everywhere(&self, server: &Cable) {
        let room_id = self.room_id;
        let payload = SyncPayload::SidebarRowRemoved(SidebarRowRemoved {
            room_id,
            refresh_room: Some(true),
        });
        send(server, Audience::Everyone, &payload, |publication| {
            SyncPublication {
                unsubscribe: Some(room_topic(room_id)),
                ..publication
            }
        });
    }
}

/// `sidebar.row.upserted` (or `sidebar.row.removed` when the room left their sidebar) for the
/// room's members, or just `user_ids`: [`RoomRows::publish`], read afresh later. A member who
/// hid the room is still a member: their connections keep following it, as the classic pages
/// keep streaming it.
pub fn sidebar_rows_later(
    server: &Cable,
    slot: &RendererSlot,
    room_id: i64,
    user_ids: Option<Vec<i64>>,
) {
    sidebar_rows_later_with_refresh(server, slot, room_id, user_ids, None);
}

/// [`sidebar_rows_later`] for a management change: the rows ask a loaded room to reload.
pub fn management_sidebar_rows_later(
    server: &Cable,
    slot: &RendererSlot,
    room_id: i64,
    user_ids: Option<Vec<i64>>,
) {
    sidebar_rows_later_with_refresh(server, slot, room_id, user_ids, Some(true));
}

fn sidebar_rows_later_with_refresh(
    server: &Cable,
    slot: &RendererSlot,
    room_id: i64,
    user_ids: Option<Vec<i64>>,
    refresh_room: Option<bool>,
) {
    defer_rows(server, slot, move |server, slot, reader| {
        let viewers = user_ids.as_deref().map_or(Viewers::Members, Viewers::Users);
        room_rows(slot, reader, room_id, |rows, conn| {
            rows.publish(server, slot, conn, viewers, refresh_room);
        });
    });
}

/// `sidebar.row.upserted` for every member of the room `membership_id` just joined, each with
/// `refreshRoom`. The joiner's row is the new sidebar entry (`store.ts` `membershipChanged`);
/// the others are unchanged rows whose flag reloads an open room. Read afresh later, under the
/// room's lock, as every row is ([`RoomRows::publish`]).
pub fn joined_open_room(
    server: &Cable,
    slot: &RendererSlot,
    conn: &Connection,
    membership_id: i64,
) {
    if slot.get(server).is_none() {
        return;
    }
    match Membership::find(conn, membership_id) {
        Ok(membership) => management_sidebar_rows_later(server, slot, membership.room_id, None),
        Err(campfire_db::Error::RecordNotFound(_)) => {}
        Err(error) => tracing::warn!(%error, membership_id, "sync: joined room not read"),
    }
}

/// `sidebar.row.upserted` for one membership's own row, when its person's sidebar shows it (or
/// the matching removal, when it no longer does), read afresh later.
pub fn membership_row_later(server: &Cable, slot: &RendererSlot, membership_id: i64) {
    defer_rows(server, slot, move |server, slot, reader| {
        // Only which room and person, with a reader given back at once: the row itself is read
        // under the room's lock.
        let mut found = None;
        reader.read(&mut |conn| {
            found = match Membership::find(conn, membership_id) {
                Ok(membership) => Some((membership.room_id, membership.user_id)),
                Err(campfire_db::Error::RecordNotFound(_)) => None,
                Err(error) => {
                    tracing::warn!(%error, membership_id, "sync: membership row not read");
                    None
                }
            };
        });
        let Some((room_id, user_id)) = found else {
            return;
        };
        room_rows(slot, reader, room_id, |rows, conn| {
            rows.publish(server, slot, conn, Viewers::Users(&[user_id]), Some(true));
        });
    });
}

/// Every member's row of a direct room, read afresh later under the room's lock, after a root
/// message was created, edited or removed: when it is (or, removed, was) the room's newest
/// root, the row's preview (`SidebarRow.lastMessage`) follows it or falls back to the one
/// before. An older root's edit leaves the preview as it was, so no row is read for it.
pub fn direct_preview_later(server: &Cable, slot: &RendererSlot, message: &Message) {
    let (room_id, message_id, created_at) = (message.room_id, message.id, message.created_at);
    defer_rows(server, slot, move |server, slot, reader| {
        room_rows(slot, reader, room_id, |rows, conn| {
            let newer = conn.query_row(
                r#"SELECT EXISTS (SELECT 1 FROM "messages" WHERE "room_id" = ?1 AND "thread_id" IS NULL AND NOT "system_note" AND ("created_at" > ?2 OR ("created_at" = ?2 AND "id" > ?3)))"#,
                rusqlite::params![room_id, created_at, message_id],
                |row| row.get::<_, bool>(0),
            );
            match newer {
                Ok(false) => rows.publish(server, slot, conn, Viewers::Members, None),
                Ok(true) => {}
                Err(error) => tracing::warn!(%error, room_id, "sync: direct preview not read"),
            }
        });
    });
}

/// A change to a person's sidebar organisation, read afresh later and published in order: each
/// membership's `sidebar.row.upserted` (none for a hidden room, which has no row), each
/// category's `sidebar.category.upserted`, then `sidebar.category.removed`. Only the person's own
/// connections get them; without one, their next resume refetches.
pub fn organized_later(
    server: &Cable,
    slot: &RendererSlot,
    change: campfire_db::models::room_category::SidebarOrganized,
) {
    let user_id = change.user_id;
    if slot.get(server).is_none() {
        return;
    }
    if !server.sync_connected(user_id) {
        server.sync_skipped_for(user_id);
        return;
    }
    defer_rows(server, slot, move |server, slot, reader| {
        for membership_id in change.membership_ids {
            // Only which room, with the reader given back: the row is read under the lock.
            let mut room_id = None;
            reader.read(&mut |conn| {
                room_id = match Membership::find(conn, membership_id) {
                    Ok(membership) if membership.user_id == user_id => Some(membership.room_id),
                    Ok(_) | Err(campfire_db::Error::RecordNotFound(_)) => None,
                    Err(error) => {
                        tracing::warn!(%error, membership_id, "sync: organised row not read");
                        None
                    }
                };
            });
            let Some(room_id) = room_id else {
                continue;
            };
            room_rows(slot, reader, room_id, |rows, conn| {
                rows.publish(server, slot, conn, Viewers::Users(&[user_id]), None);
            });
        }
        reader.read(&mut |conn| {
            for &category_id in &change.category_ids {
                match campfire_db::RoomCategory::find_by_id(conn, category_id) {
                    Ok(Some(category)) if category.user_id == user_id => send(
                        server,
                        Audience::User(user_id),
                        &SyncPayload::SidebarCategoryUpserted(campfire_api_types::RoomCategory {
                            id: category.id,
                            name: category.name,
                            collapsed: category.collapsed,
                            position: category.position,
                        }),
                        |publication| publication,
                    ),
                    Ok(_) => {}
                    Err(error) => tracing::warn!(%error, category_id, "sync: category not read"),
                }
            }
            if let Some(id) = change.removed_category_id {
                send(
                    server,
                    Audience::User(user_id),
                    &SyncPayload::SidebarCategoryRemoved(campfire_api_types::RoomCategoryRemoved {
                        id,
                    }),
                    |publication| publication,
                );
            }
        });
    });
}

/// The person left the room: `sidebar.row.removed` on their `user` topic, and their connections
/// stop following the room. Published under the room's lock, after any row read before the
/// leave, and only if they still don't belong when it goes out.
///
/// The cable sink calls this on the database writer, after the leave commits, so the room's
/// lock and a reader are taken later, off the writer. The leave's disconnect doesn't wait for
/// it: the sink closes the person's sockets at once, and a reconnecting client resumes past its
/// cursor (or refetches), so it gets the removal either way.
pub fn sidebar_row_removed_later(server: &Cable, slot: &RendererSlot, user_id: i64, room_id: i64) {
    defer_rows(server, slot, move |server, slot, reader| {
        room_rows(slot, reader, room_id, |rows, conn| {
            rows.publish(server, slot, conn, Viewers::Users(&[user_id]), None);
        });
    });
}

/// The room is gone: `sidebar.row.removed` for everyone, and nobody follows it any more. Later,
/// under the room's lock (no reader needed), so a row read before the deletion can't follow it
/// out.
pub fn room_removed(server: &Cable, slot: &RendererSlot, room_id: i64) {
    defer_rows(server, slot, move |server, slot, _| {
        room_locked(slot, room_id, |rows| rows.removed_everywhere(server));
    });
}

/// `typing` on the conversation's topic, for everyone there but the typist. Only the latest
/// state per person and conversation goes out in a batch, and none is replayed on resume.
pub fn typing(server: &Cable, topic: &str, user_id: i64, on: bool) {
    if !server.sync_wanted() {
        return;
    }
    let payload = SyncPayload::Typing(Typing { user_id, on });
    send(
        server,
        Audience::Topic(topic.to_string()),
        &payload,
        |publication| SyncPublication {
            except_user: Some(user_id),
            coalesce: Some(format!("typing:{topic}:{user_id}")),
            ephemeral: true,
            ..publication
        },
    );
}

/// `presence` on every connection's `user` topic, latest per person in a batch.
pub fn presence(server: &Cable, presence: UserPresence) {
    if !server.sync_wanted() {
        return;
    }
    let user_id = presence.user_id;
    send(
        server,
        Audience::Everyone,
        &SyncPayload::Presence(presence),
        |publication| SyncPublication {
            coalesce: Some(format!("presence:{user_id}")),
            ..publication
        },
    );
}

/// Latest workspace name and images on everyone's `user` topic.
pub fn workspace_updated(server: &Cable, branding: WorkspaceBranding) {
    if !server.sync_wanted() {
        return;
    }
    send(
        server,
        Audience::Everyone,
        &SyncPayload::WorkspaceUpdated(branding),
        |publication| SyncPublication {
            coalesce: Some("workspace".into()),
            ..publication
        },
    );
}

/// Called after the workspace CSS save and audit commit, from both UIs.
pub fn workspace_styles_updated(server: &Cable, css: Option<String>) {
    if !server.sync_wanted() {
        return;
    }
    send(
        server,
        Audience::Everyone,
        &SyncPayload::WorkspaceStylesUpdated(CustomStyles { css }),
        |publication| SyncPublication { coalesce: Some("workspace.styles".into()), ..publication },
    );
}

/// The twin of a status badge update (`StatusBadgeBroadcast`), whose presence is the badge's
/// word.
pub fn status_badge(server: &Cable, user_id: i64, presence_word: &str, status_text: Option<&str>) {
    if !server.sync_wanted() {
        return;
    }
    let presence_value = match presence_word {
        "online" => Presence::Online,
        "idle" => Presence::Idle,
        "dnd" => Presence::Dnd,
        _ => Presence::Offline,
    };
    let status_text = status_text.map(str::to_string);
    presence(
        server,
        UserPresence {
            user_id,
            presence: presence_value,
            status_text,
        },
    );
}

/// `agent.status` on every active human's `user` topic (the classic `agents:all` stream, which
/// any signed-in person may follow), read afresh later. Its working presence goes only to those
/// who share a room with the agent. Only people with a sync socket open get it; the others get a
/// gap on their `user` topic, so their next resume refetches.
pub fn agent_status_later(server: &Cable, slot: &RendererSlot, agent_id: i64) {
    let Some(renderer) = slot.get(server) else {
        return;
    };
    let (server, slot) = (server.downgrade(), slot.clone());
    renderer.defer(Box::new(move |conn| {
        let Some(server) = server.upgrade() else {
            return;
        };
        let Some(renderer) = slot.get(&server) else {
            return;
        };
        let read = (|| -> campfire_db::Result<_> {
            let Some(changed) = renderer.agent_status(conn, agent_id)? else {
                return Ok(None);
            };
            Ok(Some((changed, agent_audience(conn, agent_id)?)))
        })();
        let (changed, audience) = match read {
            Ok(Some(found)) => found,
            Ok(None) => return,
            Err(error) => return tracing::warn!(%error, agent_id, "sync: agent status not read"),
        };
        let without_presence = campfire_api_types::AgentStatusChanged {
            working_presence: None,
            working_presence_expires_at: None,
            ..changed.clone()
        };
        for (user_id, shares_room) in audience {
            if !server.sync_connected(user_id) {
                server.sync_skipped_for(user_id);
                continue;
            }
            let payload = if shares_room {
                &changed
            } else {
                &without_presence
            };
            send(
                &server,
                Audience::User(user_id),
                &SyncPayload::AgentStatus(payload.clone()),
                |publication| publication,
            );
        }
    }));
}

/// Every active human, and whether they share a room with the agent's user.
fn agent_audience(conn: &Connection, agent_id: i64) -> campfire_db::Result<Vec<(i64, bool)>> {
    let mut statement = conn.prepare_cached(
        "SELECT users.id, EXISTS (SELECT 1 FROM memberships AS theirs \
           INNER JOIN memberships AS agents ON agents.room_id = theirs.room_id \
           INNER JOIN agents AS agent ON agent.user_id = agents.user_id AND agent.id = ?1 \
           WHERE theirs.user_id = users.id) \
         FROM users WHERE users.status = 0 AND users.role != 2 ORDER BY users.id",
    )?;
    let rows = statement
        .query_map([agent_id], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<rusqlite::Result<_>>()?;
    Ok(rows)
}

/// `agent.steps` on the parent's conversation topic (a message's room or thread, or the work
/// thread's), read afresh later.
pub fn agent_steps_later(
    server: &Cable,
    slot: &RendererSlot,
    message_id: Option<i64>,
    thread_id: Option<i64>,
) {
    let Some(renderer) = slot.get(server) else {
        return;
    };
    let (server, slot) = (server.downgrade(), slot.clone());
    renderer.defer(Box::new(move |conn| {
        let Some(server) = server.upgrade() else {
            return;
        };
        let Some(renderer) = slot.get(&server) else {
            return;
        };
        match renderer.agent_steps(conn, message_id, thread_id) {
            Ok(Some(changed)) => {
                let topic = changed
                    .thread_id
                    .map_or_else(|| room_topic(changed.room_id), thread_topic);
                send(
                    &server,
                    Audience::Topic(topic),
                    &SyncPayload::AgentSteps(changed),
                    |publication| publication,
                );
            }
            Ok(None) => {}
            Err(error) => {
                tracing::warn!(%error, ?message_id, ?thread_id, "sync: agent steps not read")
            }
        }
    }));
}

/// `approval.updated` on the `user` topic of everyone who received the request's activity item
/// and can still see it, each with the card as they see it; read afresh later.
pub fn approval_updated_later(server: &Cable, slot: &RendererSlot, approval_id: i64) {
    let Some(renderer) = slot.get(server) else {
        return;
    };
    let (server, slot) = (server.downgrade(), slot.clone());
    renderer.defer(Box::new(move |conn| {
        let Some(server) = server.upgrade() else {
            return;
        };
        let Some(renderer) = slot.get(&server) else {
            return;
        };
        let read = (|| -> campfire_db::Result<Vec<(i64, campfire_api_types::ApprovalUpdated)>> {
            let items: Vec<(i64, i64)> = conn
                .prepare_cached(
                    "SELECT id, user_id FROM activity_items WHERE source_type = 'AgentApproval' \
                     AND source_id = ? AND event_type = 'agent_approval_request' ORDER BY id",
                )?
                .query_map([approval_id], |row| Ok((row.get(0)?, row.get(1)?)))?
                .collect::<rusqlite::Result<_>>()?;
            let mut updates = Vec::new();
            let mut seen = std::collections::BTreeSet::new();
            for (item_id, user_id) in items {
                if !seen.insert(user_id) {
                    continue;
                }
                let Some(user) = campfire_db::User::find_by_id(conn, user_id)? else {
                    continue;
                };
                if campfire_db::ActivityItem::find_accessible(conn, &user, item_id)?.is_none() {
                    continue;
                }
                if let Some(update) = renderer.approval_updated(conn, approval_id, user_id)? {
                    updates.push((user_id, update));
                }
            }
            Ok(updates)
        })();
        match read {
            Ok(updates) => {
                for (user_id, update) in updates {
                    send(
                        &server,
                        Audience::User(user_id),
                        &SyncPayload::ApprovalUpdated(update),
                        |publication| publication,
                    );
                }
            }
            Err(error) => tracing::warn!(%error, approval_id, "sync: approval not read"),
        }
    }));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thread_publications_share_locks_per_app_and_thread_until_finished() {
        let slot = RendererSlot::default();
        let first = slot.thread_lock(1);
        let concurrent = slot.clone().thread_lock(1);
        assert!(Arc::ptr_eq(&first, &concurrent));
        assert!(!Arc::ptr_eq(&first, &slot.thread_lock(2)));
        assert!(!Arc::ptr_eq(
            &first,
            &RendererSlot::default().thread_lock(1)
        ));
        drop(first);
        assert!(Arc::ptr_eq(&concurrent, &slot.thread_lock(1)));
        drop(concurrent);
        let _next = slot.thread_lock(3);
        assert_eq!(slot.0.threads.lock().unwrap().len(), 1);
    }

    #[test]
    fn twins_and_the_backlog_dont_overlap() {
        for (kind, events) in TWINS {
            assert!(!NOT_YET_TWINNED.contains(kind), "{kind} is in both lists");
            assert!(!events.is_empty(), "{kind} has no events");
        }
    }

    #[test]
    fn board_row_frames_have_creation_update_and_removal_twins() {
        let events = TWINS
            .iter()
            .find(|(kind, _)| *kind == "broadcasts::Broadcast")
            .unwrap()
            .1;
        for event in ["thread.created", "thread.updated", "thread.removed"] {
            assert!(events.contains(&event), "board rows need {event}");
        }
        assert!(TWINS.contains(&(
            "channel_thread::ThreadBoardCreation",
            &["thread.created"] as &[&str],
        )));
    }
}
