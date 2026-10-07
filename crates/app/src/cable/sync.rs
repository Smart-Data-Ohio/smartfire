//! The JSON twins of the broadcasts, for the single-page app's `/api/v1/sync` socket
//! (`campfire_cable::sync`, `frontend-plan.md` §2.5). Each broadcast point that has a twin calls
//! in here next to its Turbo frame, which stays byte for byte what it was.
//!
//! Twins that carry only ids are built here. Those that carry a rendered DTO (a message's
//! `bodyHtml`, a sidebar row's label and counts) go through the [`SyncRenderer`] the server
//! installs at boot (`campfire_api`); until one is installed, and whenever the sync engine isn't,
//! every twin is a no-op. So is every twin while no sync socket is open
//! ([`Cable::sync_wanted`](campfire_cable::Server::sync_wanted)): each checks that before
//! building anything.
//!
//! [`TWINS`] lists which broadcasts have twins and [`NOT_YET_TWINNED`] the ones still to port;
//! a test in the server crate fails when a broadcast is in neither.
use std::sync::{Arc, OnceLock};

use campfire_api_types::{
    MessageCards, MessageDTO, MessageReactions, MessageRemoved, PinState, PollBallot, PollUpdated,
    Presence, RoomRead, RoomUnread, SavedChanged, SidebarRow, SidebarRowRemoved, SyncPayload,
    Thread, ThreadIndicator, ThreadIndicatorChanged, ThreadRead, ThreadRemoved, ThreadUnread,
    Typing, UserPresence,
};
use campfire_cable::sync::{Audience, SyncPublication};
use campfire_db::{ChannelThread, Connection, Database, Membership, Message, Room};

use super::Cable;

/// Builds the twins that need the presenters, which live above this crate.
pub trait SyncRenderer: Send + Sync + 'static {
    /// The message as `GET /api/v1/rooms/:id/messages` serves it.
    fn message(&self, conn: &Connection, message: &Message) -> Option<MessageDTO>;
    /// The message's reactions and boosts, as `POST /api/v1/messages/:id/boosts` answers them.
    fn reactions(&self, conn: &Connection, message: &Message) -> Option<MessageReactions>;
    /// The membership's sidebar row, or `None` when the room isn't in that sidebar (an
    /// invisible membership, a deleted room).
    fn sidebar_row(
        &self,
        conn: &Connection,
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
    /// Waits for this renderer's deferred reads, so publication counts include late frames.
    #[cfg(feature = "test-support")]
    fn settle(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + '_>> {
        Box::pin(async {})
    }
}

/// Where [`Broadcasts`](super::Broadcasts) finds the installed [`SyncRenderer`].
#[derive(Clone, Default)]
pub struct RendererSlot(Arc<OnceLock<Arc<dyn SyncRenderer>>>);

impl RendererSlot {
    /// Only the first call takes effect.
    pub fn install(&self, renderer: Arc<dyn SyncRenderer>) {
        let _ = self.0.set(renderer);
    }

    #[cfg(feature = "test-support")]
    pub async fn settle(&self) {
        if let Some(renderer) = self.0.get() {
            renderer.settle().await;
        }
    }

    fn get(&self, server: &Cable) -> Option<&Arc<dyn SyncRenderer>> {
        server.sync_wanted().then(|| self.0.get()).flatten()
    }
}

/// The broadcast points and the sync events they produce. A point is named as the server
/// crate's coverage test finds it: a [`Broadcasts`](super::Broadcasts) method
/// (`Broadcasts::<name>`), a free function here (`broadcasts::<name>`), a channel, or a kind the
/// cable sink handles (its type's path, as the sink names it, without `campfire_db::models::`,
/// `campfire_db::` or `crate::integrations::`).
pub const TWINS: &[(&str, &[&str])] = &[
    (
        "Broadcasts::message_create",
        &["message.created", "room.unread", "sidebar.row.upserted"],
    ),
    (
        "Broadcasts::unread_room",
        &["room.unread", "sidebar.row.upserted"],
    ),
    ("Broadcasts::mark_room_unread", &["room.unread"]),
    ("Broadcasts::message_remove", &["message.removed"]),
    ("Broadcasts::message_replace", &["message.updated"]),
    ("Broadcasts::message_part_replace", &["message.updated"]),
    (
        "Broadcasts::message_thread_part_replace",
        &["message.updated"],
    ),
    (
        "Broadcasts::message_reactions_replace",
        &["message.reactions"],
    ),
    ("Broadcasts::thread_refresh", &["thread.unread"]),
    ("Broadcasts::thread_created", &["thread.created"]),
    ("Broadcasts::thread_updated", &["thread.updated"]),
    ("Broadcasts::thread_removed", &["thread.removed"]),
    ("Broadcasts::thread_read", &["thread.read"]),
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
    ("broadcasts::read_room", &["room.read"]),
    // No classic frame: the sink publishes these for the single-page app only.
    (
        "scheduled_message::ScheduledMessageChange",
        &["scheduled.changed", "scheduled.removed"],
    ),
    ("activity_item::ActivityItemsRemoved", &["activity.removed"]),
    ("agent::AgentSyncChange", &["agent.status"]),
    ("agent_approval::ApprovalChange", &["approval.updated"]),
    ("channel_thread::ThreadWorkChange", &["thread.updated"]),
    ("activity_item::ActivityItemTouched", &["activity.item"]),
    (
        "room_category::SidebarOrganized",
        &[
            "sidebar.row.upserted",
            "sidebar.category.upserted",
            "sidebar.category.removed",
        ],
    ),
    ("poll::PollChanged", &["poll.updated", "poll.ballot"]),
    // The card slots' replaces after a fetch, a refresh or an event's change.
    ("calendar_event::CardUpdate", &["message.cards"]),
    ("link_embed::store::CardUpdate", &["message.cards"]),
    ("fizzy::cards::CardUpdate", &["message.cards"]),
    ("twitter::post::CardUpdate", &["message.cards"]),
    ("github::pull_requests::CardUpdated", &["message.cards"]),
    ("TypingNotificationsChannel", &["typing"]),
    // The domain's Turbo and cable frames: appends and replaces of `Partial::Message`,
    // `user_<id>_unreads`/`user_<id>_reads`/`user_<id>_unread_threads`, the pin badge
    // (`Partial::PinBadge`) and the thread indicator (`Partial::ThreadIndicator`, which also
    // carries the thread's new count and activity), a direct room's sidebar row
    // (`Partial::DirectSidebar`, with the member's row), and the huddle notices and invitations on
    // `user_<id>_huddle_notices`/`user_<id>_activity`. Its other frames (message features, room
    // headers, polls, board rows and the other directory partials) have no twin yet. Its
    // `ActivityChannel` frames (`user_<id>_activity`) have `activity.item`.
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
            "thread.updated",
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
    ("huddle_effects::StagePanel", &["stage.updated"]),
    ("huddle_effects::RoleEvent", &["huddle.role"]),
    ("huddle_effects::StageEndedNote", &["message.created"]),
    ("RoomRemovalBroadcast", &["sidebar.row.removed"]),
    (
        "user_status_settings::updates::StatusBadgeBroadcast",
        &["presence"],
    ),
    ("user::lifecycle::QuietStreamFinal", &["message.updated"]),
    // The status badge and directory row replaces on `agents:all`. Their twin is
    // `agent::AgentSyncChange`'s `agent.status`, which the same save emits (once, not per frame).
    ("agent::AgentStatusChange", &["agent.status"]),
    // The message replace, or the thread's `agent_steps_channel_thread_<id>` list.
    ("agent_step::StepParentChange", &["agent.steps"]),
];

/// Broadcast points with no sync event yet: the SPA slices after S1 port them.
pub const NOT_YET_TWINNED: &[&str] = &[
    "Broadcasts::boost_create",
    "Broadcasts::boost_remove",
    "board_automations::DigestNotes",
    "user_status_settings::updates::OooNoticeBroadcast",
    "github::notifier::MessageCreated",
];

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

/// `message.updated`, read afresh later: for broadcast points without a connection.
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
    send(server, Audience::User(user_id), &payload, |publication| {
        publication
    });
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
    send(
        server,
        Audience::User(user_id),
        &SyncPayload::RoomRead(RoomRead { room_id }),
        |publication| publication,
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
        match item_room(conn, item_id) {
            Ok(Some(room)) => sidebar_rows(&server, &slot, conn, &room, Some(&[user_id])),
            Ok(None) => {}
            Err(error) => tracing::warn!(%error, item_id, "sync: activity item's room not read"),
        }
    }));
}

/// The room of the message an activity item is about, if it's about a message.
fn item_room(conn: &Connection, item_id: i64) -> campfire_db::Result<Option<Room>> {
    let room_id: Option<i64> = conn
        .prepare_cached(
            r#"SELECT "messages"."room_id" FROM "activity_items" INNER JOIN "messages" ON "messages"."id" = "activity_items"."source_id" WHERE "activity_items"."id" = ? AND "activity_items"."source_type" = 'Message'"#,
        )?
        .query_map([item_id], |row| row.get(0))?
        .next()
        .transpose()?;
    match room_id {
        Some(room_id) => Room::find_by_id(conn, room_id),
        None => Ok(None),
    }
}

/// `activity.removed` on each owner's `user` topic, with their unread count afterwards.
pub fn activity_removed_later(server: &Cable, slot: &RendererSlot, items: Vec<(i64, i64)>) {
    let Some(renderer) = slot.get(server) else {
        return;
    };
    // Only owners with a sync socket open; the others get a gap marker.
    let items = items
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
    let server = server.downgrade();
    renderer.defer(Box::new(move |conn| {
        let Some(server) = server.upgrade() else {
            return;
        };
        for (id, user_id) in items {
            let count = campfire_db::User::find_by_id(conn, user_id).and_then(|user| match user {
                Some(user) => campfire_db::ActivityItem::unread_count(conn, &user),
                None => Ok(0),
            });
            match count {
                Ok(unread_count) => send(
                    &server,
                    Audience::User(user_id),
                    &SyncPayload::ActivityRemoved(campfire_api_types::ActivityItemRemoved {
                        id,
                        unread_count,
                    }),
                    |publication| publication,
                ),
                Err(error) => tracing::warn!(%error, id, "sync: activity count not read"),
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
pub fn thread_indicator(
    server: &Cable,
    slot: &RendererSlot,
    conn: &Connection,
    parent_message_id: i64,
) {
    let Some(renderer) = slot.get(server) else {
        return;
    };
    let parent = match Message::find_by_id(conn, parent_message_id) {
        Ok(Some(parent)) => parent,
        Ok(None) => return,
        Err(error) => {
            return tracing::warn!(%error, parent_message_id, "sync: thread indicator not read");
        }
    };
    let indicator = match renderer.thread_indicator(conn, &parent) {
        Ok(indicator) => indicator,
        Err(error) => {
            return tracing::warn!(%error, parent_message_id, "sync: thread indicator not rendered");
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
    if let Some(thread_id) = thread_id {
        thread_changed(server, slot, conn, thread_id, false);
    }
}

/// `thread.created` on the thread's room, or `thread.updated` on its room and its own topic.
pub fn thread_changed(
    server: &Cable,
    slot: &RendererSlot,
    conn: &Connection,
    thread_id: i64,
    created: bool,
) {
    let Some(renderer) = slot.get(server) else {
        return;
    };
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

/// [`thread_changed`], read afresh later: for broadcast points without a connection.
pub fn thread_changed_later(server: &Cable, slot: &RendererSlot, thread_id: i64, created: bool) {
    let Some(renderer) = slot.get(server) else {
        return;
    };
    let (server, slot) = (server.downgrade(), slot.clone());
    renderer.defer(Box::new(move |conn| {
        if let Some(server) = server.upgrade() {
            thread_changed(&server, &slot, conn, thread_id, created);
        }
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

/// `sidebar.row.upserted` (or `sidebar.row.removed` when the room left their sidebar) for the
/// room's members, or just `user_ids` among them. A member who hid the room is still a member:
/// their connections keep following it, as the classic pages keep streaming it. A row that
/// can't be read is skipped (and logged), not taken for gone.
///
/// Only members with a sync socket open get theirs: a row costs a few reads, and a big room
/// has many members. For the others the ring records a gap on their `user` topic, so their
/// next resume refetches the sidebar.
pub fn sidebar_rows(
    server: &Cable,
    slot: &RendererSlot,
    conn: &Connection,
    room: &Room,
    user_ids: Option<&[i64]>,
) {
    let Some(renderer) = slot.get(server) else {
        return;
    };
    let memberships = match Membership::for_room(conn, room.id) {
        Ok(memberships) => memberships,
        Err(error) => {
            return tracing::warn!(%error, room_id = room.id, "sync: sidebar rows not read");
        }
    };
    for membership in memberships {
        if user_ids.is_some_and(|ids| !ids.contains(&membership.user_id)) {
            continue;
        }
        if !server.sync_connected(membership.user_id) {
            server.sync_skipped_for(membership.user_id);
            continue;
        }
        match renderer.sidebar_row(conn, room, &membership) {
            Ok(Some(row)) => send(
                server,
                Audience::User(membership.user_id),
                &SyncPayload::SidebarRowUpserted(row),
                |publication| publication,
            ),
            Ok(None) if room.deleted() => sidebar_row_removed(server, membership.user_id, room.id),
            Ok(None) => sidebar_row_hidden(server, membership.user_id, room.id),
            Err(error) => tracing::warn!(
                %error,
                room_id = room.id,
                user_id = membership.user_id,
                "sync: sidebar row not rendered"
            ),
        }
    }
}

/// `sidebar.row.upserted` for one membership's own row, when its person's sidebar shows it.
pub fn membership_row(server: &Cable, slot: &RendererSlot, conn: &Connection, membership_id: i64) {
    if slot.get(server).is_none() {
        return;
    }
    let found = Membership::find(conn, membership_id)
        .and_then(|membership| Ok((membership.room(conn)?, membership)));
    match found {
        Ok((room, membership)) => {
            sidebar_rows(server, slot, conn, &room, Some(&[membership.user_id]));
        }
        Err(campfire_db::Error::RecordNotFound(_)) => {}
        Err(error) => tracing::warn!(%error, membership_id, "sync: membership row not read"),
    }
}

/// [`sidebar_rows`], read afresh later: for broadcast points without a connection.
pub fn sidebar_rows_later(
    server: &Cable,
    slot: &RendererSlot,
    room_id: i64,
    user_ids: Option<Vec<i64>>,
) {
    let Some(renderer) = slot.get(server) else {
        return;
    };
    let (server, slot) = (server.downgrade(), slot.clone());
    renderer.defer(Box::new(move |conn| {
        let (Some(server), Ok(Some(room))) = (server.upgrade(), Room::find_by_id(conn, room_id))
        else {
            return;
        };
        sidebar_rows(&server, &slot, conn, &room, user_ids.as_deref());
    }));
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
    let Some(renderer) = slot.get(server) else {
        return;
    };
    let user_id = change.user_id;
    if !server.sync_connected(user_id) {
        server.sync_skipped_for(user_id);
        return;
    }
    let server = server.downgrade();
    let job_renderer = renderer.clone();
    renderer.defer(Box::new(move |conn| {
        let Some(server) = server.upgrade() else {
            return;
        };
        for membership_id in change.membership_ids {
            let found = match Membership::find(conn, membership_id) {
                Ok(membership) if membership.user_id == user_id => membership,
                Ok(_) | Err(campfire_db::Error::RecordNotFound(_)) => continue,
                Err(error) => {
                    tracing::warn!(%error, membership_id, "sync: organised row not read");
                    continue;
                }
            };
            let row = found
                .room(conn)
                .and_then(|room| job_renderer.sidebar_row(conn, &room, &found));
            match row {
                Ok(Some(row)) => send(
                    &server,
                    Audience::User(user_id),
                    &SyncPayload::SidebarRowUpserted(row),
                    |publication| publication,
                ),
                Ok(None) => {}
                Err(error) => {
                    tracing::warn!(%error, membership_id, "sync: organised row not rendered");
                }
            }
        }
        for category_id in change.category_ids {
            match campfire_db::RoomCategory::find_by_id(conn, category_id) {
                Ok(Some(category)) if category.user_id == user_id => send(
                    &server,
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
                &server,
                Audience::User(user_id),
                &SyncPayload::SidebarCategoryRemoved(campfire_api_types::RoomCategoryRemoved {
                    id,
                }),
                |publication| publication,
            );
        }
    }));
}

/// The person left the room: `sidebar.row.removed` on their `user` topic, and their connections
/// stop following the room.
pub fn sidebar_row_removed(server: &Cable, user_id: i64, room_id: i64) {
    if !server.sync_wanted() {
        return;
    }
    let payload = SyncPayload::SidebarRowRemoved(SidebarRowRemoved { room_id });
    send(server, Audience::User(user_id), &payload, |publication| {
        SyncPublication {
            unsubscribe: Some(room_topic(room_id)),
            ..publication
        }
    });
}

/// The person hid the room (an invisible membership): `sidebar.row.removed` on their `user`
/// topic, while their connections go on following it.
fn sidebar_row_hidden(server: &Cable, user_id: i64, room_id: i64) {
    let payload = SyncPayload::SidebarRowRemoved(SidebarRowRemoved { room_id });
    send(server, Audience::User(user_id), &payload, |publication| {
        publication
    });
}

/// The room is gone: `sidebar.row.removed` for everyone, and nobody follows it any more.
pub fn room_removed(server: &Cable, room_id: i64) {
    if !server.sync_wanted() {
        return;
    }
    let payload = SyncPayload::SidebarRowRemoved(SidebarRowRemoved { room_id });
    send(server, Audience::Everyone, &payload, |publication| {
        SyncPublication {
            unsubscribe: Some(room_topic(room_id)),
            ..publication
        }
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
    fn twins_and_the_backlog_dont_overlap() {
        for (kind, events) in TWINS {
            assert!(!NOT_YET_TWINNED.contains(kind), "{kind} is in both lists");
            assert!(!events.is_empty(), "{kind} has no events");
        }
    }
}
