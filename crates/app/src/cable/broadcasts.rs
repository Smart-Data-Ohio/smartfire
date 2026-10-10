//! JSON sync publications and the retained Action Cable channel notifications.
use campfire_db::models::activity_item::ActivityItemsRemoved;
use campfire_db::rich_text::RichText;
use campfire_db::{Connection, Membership, Message, Room};
use rails_compat::global_id::GlobalId;
use serde::Serialize;

use campfire_db::broadcasts::unread_rooms_stream_name;

use super::sync::{self, RendererSlot, SyncRenderer};
use super::{Cable, read_rooms_stream_name, room_gid, thread_gid, user_gid};

/// `dom_id(record, prefix)`.
pub fn dom_id(param_key: &str, key: impl std::fmt::Display, prefix: Option<&str>) -> String {
    match prefix {
        Some(prefix) => format!("{prefix}_{param_key}_{key}"),
        None => format!("{param_key}_{key}"),
    }
}

/// `Room.model_name.param_key` for the room's STI class: `Rooms::Open` is `rooms_open`.
pub fn room_param_key(room: &Room) -> String {
    room.room_type
        .class_name()
        .replace("::", "_")
        .to_ascii_lowercase()
}

/// `dom_id(room, prefix)`.
pub fn room_dom_id(room: &Room, prefix: &str) -> String {
    dom_id(&room_param_key(room), room.id, Some(prefix))
}

/// `dom_id(message, prefix)`: messages are keyed by `client_message_id`.
pub fn message_dom_id(message: &Message, prefix: Option<&str>) -> String {
    dom_id("message", &message.client_message_id, prefix)
}

/// `dom_id(thread, prefix)`.
pub fn thread_dom_id(thread_id: i64, prefix: &str) -> String {
    dom_id("channel_thread", thread_id, Some(prefix))
}

pub const ROOMS: &str = "rooms";
pub const MESSAGES: &str = "messages";

/// A Turbo stream: the streamables `broadcast_*_to` and `turbo_stream_from` take.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stream(Vec<String>);

impl Stream {
    /// `record, :suffix`
    pub fn record(gid: &GlobalId, suffix: &str) -> Self {
        Self(vec![gid.to_param(), suffix.to_string()])
    }

    /// A single name (`:rooms`, `AgentsChannel::STREAM_NAME`).
    pub fn named(name: &str) -> Self {
        Self(vec![name.to_string()])
    }

    /// `:rooms`: every signed-in user's sidebar.
    pub fn rooms() -> Self {
        Self::named(ROOMS)
    }

    /// `user, :rooms`: one user's sidebar.
    pub fn user_rooms(user_id: i64) -> Self {
        Self::record(&user_gid(user_id), ROOMS)
    }

    /// `user, :status`
    #[allow(dead_code)] // WS14's status broadcasts use this API.
    pub fn user_status(user_id: i64) -> Self {
        Self::record(&user_gid(user_id), "status")
    }

    /// `member, :ooo_notice`
    #[allow(dead_code)] // WS14's OOO broadcasts use this API.
    pub fn ooo_notice(user_id: i64) -> Self {
        Self::record(&user_gid(user_id), "ooo_notice")
    }

    /// `room, :messages`
    pub fn room_messages(room: &Room) -> Self {
        Self::record(&room_gid(room), MESSAGES)
    }

    /// `thread, :messages`
    pub fn thread_messages(thread_id: i64) -> Self {
        Self::record(&thread_gid(thread_id), MESSAGES)
    }

    /// `message.conversation, :messages` (`thread || room`); `room` is the message's.
    pub fn conversation(room: &Room, message: &Message) -> Self {
        match message.thread_id {
            Some(thread_id) => Self::thread_messages(thread_id),
            None => Self::room_messages(room),
        }
    }

    pub fn streamables(&self) -> Vec<&str> {
        self.0.iter().map(String::as_str).collect()
    }
}

/// `dom_id(message.conversation, :messages)`: where a conversation's messages are appended.
pub fn conversation_messages_target(room: &Room, message: &Message) -> String {
    match message.thread_id {
        Some(thread_id) => thread_dom_id(thread_id, MESSAGES),
        None => room_dom_id(room, MESSAGES),
    }
}


/// `ActionCable.server.broadcast "user_#{id}_reads", { room_id: }`
/// (reference/app/channels/presence_channel.rb, reference/app/controllers/rooms/reads_controller.rb).
pub fn read_room(server: &Cable, user_id: i64, room_id: i64) -> usize {
    #[derive(Serialize)]
    struct ReadRoom {
        room_id: i64,
    }
    let reached = server.broadcast(&read_rooms_stream_name(user_id), &ReadRoom { room_id });
    sync::room_read(server, user_id, room_id);
    reached
}

#[derive(Clone)]
pub struct Broadcasts {
    server: Cable,
    sync: RendererSlot,
}

impl Broadcasts {
    pub fn new(server: Cable) -> Self {
        Self {
            server,
            sync: RendererSlot::default(),
        }
    }

    /// Lets the twins that carry rendered DTOs publish ([`sync`]).
    pub fn install_sync_renderer(&self, renderer: std::sync::Arc<dyn SyncRenderer>) {
        self.sync.install(renderer);
    }

    /// Drains this app's deferred twins before test assertions count socket publications.
    #[cfg(feature = "test-support")]
    pub async fn settle_sync(&self) {
        self.sync.settle().await;
    }

    /// `sidebar.row.upserted` after an open-room join. The joiner's row is new. Every other
    /// member's row carries `refreshRoom`, so a room they have open reloads its member count and
    /// members pane. The classic page only prepends HTML on the joiner's sidebar.
    pub fn joined_open_room(&self, conn: &Connection, membership_id: i64) {
        sync::joined_open_room(&self.server, &self.sync, conn, membership_id);
    }

    /// `message.created` (or `message.updated`) for a message a broadcast point outside this
    /// type rendered.
    pub fn sync_message(&self, conn: &Connection, message: &Message, created: bool) {
        sync::message(&self.server, &self.sync, conn, message, created);
    }

    /// `thread.indicator` (and `thread.updated`) for the indicator replace of a thread's parent
    /// message, which a broadcast point outside this type rendered: read afresh later.
    pub fn sync_thread_indicator(&self, parent_message_id: i64) {
        sync::thread_indicator_later(&self.server, &self.sync, parent_message_id);
    }

    /// `sidebar.row.upserted` for the people a new message made the room unread for, read
    /// afresh later: their counts changed. For an unread ping a broadcast point outside this
    /// type sent.
    pub fn sync_unread_rows(&self, room_id: i64, user_ids: Vec<i64>) {
        sync::sidebar_rows_later(&self.server, &self.sync, room_id, Some(user_ids));
    }

    /// `activity.item` for an `ActivityChannel` frame (`user_<id>_activity`) a broadcast point
    /// outside this type sent: the item is read afresh later.
    pub fn sync_activity_stream(&self, stream: &str, payload: &serde_json::Value) {
        sync::activity_stream(&self.server, &self.sync, stream, payload);
    }

    /// `activity.item` for an item changed without an `ActivityChannel` frame.
    pub fn sync_activity_item(&self, user_id: i64, item_id: i64) {
        sync::activity_item_later(&self.server, &self.sync, user_id, item_id);
    }

    /// `activity.removed` for removed inbox items.
    pub fn sync_activity_removed(&self, removed: ActivityItemsRemoved) {
        sync::activity_removed_later(&self.server, &self.sync, removed);
    }

    /// `scheduled.changed` or `scheduled.removed` for a scheduled message's change.
    pub fn sync_scheduled(
        &self,
        change: campfire_db::models::scheduled_message::ScheduledMessageChange,
    ) {
        sync::scheduled_later(&self.server, &self.sync, change);
    }

    /// `agent.status` for an agent's status, note, suspension or working presence change.
    pub fn sync_agent_status(&self, agent_id: i64) {
        sync::agent_status_later(&self.server, &self.sync, agent_id);
    }

    /// `agent.steps` for a parent whose steps changed.
    pub fn sync_agent_steps(&self, message_id: Option<i64>, thread_id: Option<i64>) {
        sync::agent_steps_later(&self.server, &self.sync, message_id, thread_id);
    }

    /// `approval.updated` for an approval request that was decided, cancelled or expired.
    pub fn sync_approval(&self, approval_id: i64) {
        sync::approval_updated_later(&self.server, &self.sync, approval_id);
    }

    /// `sidebar.row.upserted`, `sidebar.category.upserted` and `sidebar.category.removed` for a
    /// change to a person's sidebar organisation.
    pub fn sync_organized(&self, change: campfire_db::models::room_category::SidebarOrganized) {
        sync::organized_later(&self.server, &self.sync, change);
    }

    /// `poll.updated` and the voter's `poll.ballot` for a vote or a close.
    pub fn sync_poll(&self, change: campfire_db::models::poll::PollChanged) {
        sync::poll_later(&self.server, &self.sync, change);
    }

    /// `events.changed` on the room's topic for a change to one of its events.
    pub fn sync_events_changed(&self, change: campfire_db::models::calendar_event::EventsChanged) {
        sync::events_changed(&self.server, change);
    }

    /// `message.cards` for each of `messages`, for a card slot's replace a broadcast point
    /// outside this type sent.
    pub fn sync_message_cards(&self, conn: &Connection, messages: &[campfire_db::Message]) {
        sync::message_cards(&self.server, &self.sync, conn, messages);
    }

    /// A management write's viewer-qualified row, including a hidden row refresh, after
    /// a broadcast point outside this type replaced its metadata or members.
    /// Read later, under the room's lock: a caller holding a reader (or on the database
    /// writer) never waits for that lock.
    pub fn sync_membership_row(&self, membership_id: i64) {
        sync::membership_row_later(&self.server, &self.sync, membership_id);
    }

    /// `sidebar.row.removed` after a person left a room (`Membership#broadcast_room_removal_to_user`,
    /// which the cable sink sends on the database writer): published later under the room's
    /// lock, after any row of the room read before the leave.
    pub fn sync_row_removed(&self, user_id: i64, room_id: i64) {
        sync::sidebar_row_removed_later(&self.server, &self.sync, user_id, room_id);
    }

    /// `ActionCable.server.broadcast broadcasting, payload`, for the channels' own streams
    /// (`UnreadThreadsChannel.stream_name_for`, `ActivityChannel.stream_name_for`, ...).
    pub fn channel<T: Serialize + ?Sized>(&self, broadcasting: &str, payload: &T) -> usize {
        self.server.broadcast(broadcasting, payload)
    }

    /// `Rooms::ReadsController#destroy`: the requester's other sessions mark the row unread.
    pub fn mark_room_unread(&self, user_id: i64, room_id: i64) -> usize {
        #[derive(Serialize)]
        struct UnreadRoom {
            #[serde(rename = "roomId")]
            room_id: i64,
        }
        let reached = self.channel(&unread_rooms_stream_name(user_id), &UnreadRoom { room_id });
        sync::room_unread(&self.server, user_id, room_id, None, false);
        // Its red count now takes in the pings after the new read position.
        self.sync_read_row(user_id, room_id);
        reached
    }

    // Channel threads. Only the refresh has a classic frame; the others are the sync socket's
    // alone (the classic pages reload the thread list and pane).

    /// `UnreadThreadsChannel.broadcast_to(user, thread_id:, room_id:, refresh_only: true)`: a
    /// reply or the parent was deleted, so the member's row needs refreshing.
    pub fn thread_refresh(&self, user_id: i64, room_id: i64, thread_id: i64) -> usize {
        let reached = self.channel(
            &format!("user_{user_id}_unread_threads"),
            &serde_json::json!({"threadId": thread_id, "roomId": room_id, "refreshOnly": true}),
        );
        sync::thread_unread(&self.server, user_id, thread_id, room_id, true);
        reached
    }

    /// A thread was started.
    pub fn thread_created(&self, thread_id: i64) {
        sync::thread_changed_later(&self.server, &self.sync, thread_id, true);
    }

    /// A thread was renamed, closed, reopened, locked or unlocked.
    pub fn thread_updated(&self, thread_id: i64) {
        sync::thread_changed_later(&self.server, &self.sync, thread_id, false);
    }

    /// A thread was deleted: its pings leave the room's red counts.
    pub fn thread_removed(&self, thread_id: i64, room_id: i64) {
        sync::thread_removed(&self.server, thread_id, room_id);
        sync::sidebar_rows_later(&self.server, &self.sync, room_id, None);
    }

    /// A board's tag rules or SLA timers changed.
    pub fn board_automations_changed(&self, room_id: i64) {
        sync::board_automations_changed(&self.server, room_id);
    }

    /// The person read a thread: its pings leave the room's red count.
    pub fn thread_read(&self, user_id: i64, thread_id: i64, room_id: i64) {
        sync::thread_read(&self.server, user_id, thread_id, room_id);
        self.sync_read_row(user_id, room_id);
    }

    /// `sidebar.row.upserted` for the person's own row after they read the room or one of its
    /// threads, left a thread, or marked it unread, read afresh later: the server's count agrees
    /// with the read (and with the client's own clearing of it), so a reload doesn't bring a
    /// badge back.
    pub fn sync_read_row(&self, user_id: i64, room_id: i64) {
        sync::sidebar_rows_later(&self.server, &self.sync, room_id, Some(vec![user_id]));
    }

    // Message::Broadcasts (reference/app/models/message/broadcasts.rb)

    /// `message.broadcast_create`: append the message to its conversation (`thread || room`),
    /// then, unless it's a thread message or a system note, `broadcast_unread_room`.
    pub fn message_create(
        &self, conn: &Connection, room: &Room, message: &Message,
        rich_text: &dyn RichText,
    ) -> campfire_db::Result<()> {
        self.sync_message(conn, message, true);
        if message.thread_id.is_none() && !message.system_note {
            self.unread_room(conn, room, message, rich_text)?;
        }
        Ok(())
    }

    /// `broadcast_unread_room`: `{ roomId: }` to each member's `user_<id>_unreads`, leaving out
    /// muted members the message doesn't mention (`unread_user_ids`).
    pub fn unread_room(
        &self,
        conn: &Connection,
        room: &Room,
        message: &Message,
        rich_text: &dyn RichText,
    ) -> campfire_db::Result<()> {
        #[derive(Serialize)]
        struct UnreadRoom {
            #[serde(rename = "roomId")]
            room_id: i64,
        }
        let user_ids = unread_user_ids(conn, room, message, rich_text)?;
        for &user_id in &user_ids {
            self.channel(
                &unread_rooms_stream_name(user_id),
                &UnreadRoom { room_id: room.id },
            );
        }
        if self.server.sync_wanted() {
            let mentioned: Vec<i64> = message
                .mentionees(conn, rich_text)?
                .iter()
                .map(|user| user.id)
                .collect();
            for &user_id in &user_ids {
                sync::room_unread(
                    &self.server,
                    user_id,
                    room.id,
                    Some(message.id),
                    mentioned.contains(&user_id),
                );
            }
            // A direct row previews its newest root, so every member's row changes with one.
            if room.direct() && previews(message) {
                self.direct_preview_later(room, message);
            } else {
                sync::sidebar_rows_later(&self.server, &self.sync, room.id, Some(user_ids));
            }
        }
        Ok(())
    }

    /// `message.broadcast_remove`: `broadcast_remove_to message_stream_target, :messages`.
    /// MessagesController#destroy and `User#remove_banned_content`.
    pub fn message_remove(&self, room: &Room, message: &Message) {
        sync::message_removed(&self.server, message);
        // Its unread count and pings, and their inbox items, went with it; every member's row
        // is read afresh, which also moves a direct row's preview back to the message before.
        sync::sidebar_rows_later(&self.server, &self.sync, room.id, None);
    }

    /// A direct row previews its newest root message (`SidebarRow.lastMessage`): after one is
    /// created, edited or removed, every member's row is read afresh if it is (or was) the
    /// newest, so the preview follows (or falls back to the message before).
    fn direct_preview_later(&self, room: &Room, message: &Message) {
        if room.direct() && previews(message) {
            sync::direct_preview_later(&self.server, &self.sync, message);
        }
    }

    /// A message edit also changes the newest direct-message preview.
    pub fn message_replace(&self, room: &Room, message: &Message) {
        sync::message_updated_later(&self.server, &self.sync, message.id);
        self.direct_preview_later(room, message);
    }

    pub fn message_reactions_replace(&self, message: &Message) {
        sync::message_reactions_later(&self.server, &self.sync, message.id);
    }

    pub fn room_remove(&self, room: &Room) {
        sync::room_removed(&self.server, &self.sync, room.id);
    }

    pub fn open_room_create(&self, room: &Room) {
        sync::management_sidebar_rows_later(&self.server, &self.sync, room.id, None);
    }

    pub fn open_room_update(&self, room: &Room) {
        sync::management_sidebar_rows_later(&self.server, &self.sync, room.id, None);
    }

    pub fn closed_room_create(&self, conn: &Connection, room: &Room) -> campfire_db::Result<()> {
        let user_ids = room.user_ids(conn)?;
        sync::management_sidebar_rows_later(&self.server, &self.sync, room.id, Some(user_ids));
        Ok(())
    }

    pub fn closed_room_update(&self, conn: &Connection, room: &Room) -> campfire_db::Result<()> {
        self.closed_room_create(conn, room)
    }

    pub fn direct_room_create(&self, room: &Room) {
        sync::management_sidebar_rows_later(&self.server, &self.sync, room.id, None);
    }

    pub fn involvement_change(&self, room: &Room, membership: &Membership) {
        sync::sidebar_rows_later(&self.server, &self.sync, room.id, Some(vec![membership.user_id]));
    }
}

/// `unread_user_ids`: every member, except that when any is muted, muted members the message
/// doesn't mention are left out.
fn unread_user_ids(
    conn: &Connection,
    room: &Room,
    message: &Message,
    rich_text: &dyn RichText,
) -> campfire_db::Result<Vec<i64>> {
    let memberships = Membership::for_room(conn, room.id)?;
    let muted = |membership: &Membership| membership.involvement == Some(Involvement::Muted);
    if !memberships.iter().any(muted) {
        return Ok(memberships
            .iter()
            .map(|membership| membership.user_id)
            .collect());
    }
    let mentioned: Vec<i64> = message
        .mentionees(conn, rich_text)?
        .iter()
        .map(|user| user.id)
        .collect();
    Ok(memberships
        .iter()
        .filter(|m| !muted(m) || mentioned.contains(&m.user_id))
        .map(|m| m.user_id)
        .collect())
}

/// Whether a direct row could preview `message`: a root message that isn't a system note.
fn previews(message: &Message) -> bool {
    message.thread_id.is_none() && !message.system_note
}
