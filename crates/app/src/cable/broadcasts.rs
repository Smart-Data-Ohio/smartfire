//! The broadcast layer: a typed API for every broadcast the Rails app makes, producing exactly
//! the stream names, targets and `<turbo-stream>` markup its Turbo and Action Cable calls do.
//! `crates/cable/BROADCASTS.md` lists each of the app's broadcast calls and where it stands.
//!
//! - [`Stream`] names a stream the way `broadcast_*_to`/`turbo_stream_from` build it: records are
//!   their GID param (`[@room, :messages]` is `<room gid param>:messages`), symbols themselves.
//! - Targets are `dom_id`s ([`dom_id`], [`room_dom_id`], [`message_dom_id`]): STI rooms use their
//!   own `param_key` (`messages_rooms_open_1`), and a message's `to_key` is its
//!   `client_message_id` (`message_<uuid>`).
//! - [`Broadcasts::turbo`] and its shorthands send a Turbo Stream action with HTML the caller
//!   rendered (`campfire_views`, rendered once, outside any request, as
//!   `ApplicationController.render` does); [`Broadcasts::channel`] is `ActionCable.server.broadcast`.
//! - The named methods are the broadcasts of domains already ported, with their partials from
//!   [`Partials`].
//!
//! Broadcast HTML is shared by every subscriber, so it must carry nothing session-bound: the cable
//! server refuses (and logs) any that holds a CSRF token or CSP nonce
//! (`campfire_cable::turbo::session_bound`).
use campfire_cable::turbo::{Action, Target};
use campfire_db::rich_text::RichText;
use campfire_db::{Connection, Involvement, Membership, Message, Room};
#[cfg(any(test, feature = "test-support"))]
use campfire_db::Boost;
use rails_compat::global_id::GlobalId;
use serde::Serialize;

use campfire_db::broadcasts::unread_rooms_stream_name;

use super::sync::{self, RendererSlot, SyncRenderer};
use super::{Cable, read_rooms_stream_name, room_gid, thread_gid, user_gid};

/// The partials Turbo renders for broadcasts (`ApplicationController.render(partial:, locals:)`,
/// html format, no request). Each returns the rendered HTML.
pub trait Partials: Send + Sync {
    /// `messages/_message` with `message:`.
    fn message(&self, message: &Message) -> String;
    /// `messages/_presentation` with `message:`.
    fn message_presentation(&self, message: &Message) -> String;
    /// Legacy append-frame primitive, retained for the cable wire tests. The default serves the
    /// crates above's non-test builds, whose implementations define it only under `cfg(test)`.
    #[cfg(any(test, feature = "test-support"))]
    fn boost(&self, _boost: &Boost) -> String {
        String::new()
    }
    /// `users/sidebars/rooms/_shared` with `room:`.
    fn shared_room(&self, room: &Room) -> String;
    /// `users/sidebars/rooms/_direct` with `membership:`.
    fn direct_room(&self, membership: &Membership) -> String;
    /// The sidebar row for a room that isn't direct (`users/sidebars/rooms/_stage`, `_voice`,
    /// `_board` or `_shared` by its type) with `room:`, `membership:` and, when given, `unread:`.
    fn sidebar_row(&self, room: &Room, membership: &Membership, unread: Option<bool>) -> String;
}

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

const MAINTAIN_SCROLL: &[(&str, Option<&str>)] = &[("maintain_scroll", Some("true"))];

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

    /// `message.created` (or `message.updated`) for a message a broadcast point outside this
    /// type rendered.
    pub fn sync_message(&self, conn: &Connection, message: &Message, created: bool) {
        sync::message(&self.server, &self.sync, conn, message, created);
    }

    /// `thread.indicator` (and `thread.updated`) for the indicator replace of a thread's parent
    /// message, which a broadcast point outside this type rendered.
    pub fn sync_thread_indicator(&self, conn: &Connection, parent_message_id: i64) {
        sync::thread_indicator(&self.server, &self.sync, conn, parent_message_id);
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

    /// `activity.removed` for items deleted with their source.
    pub fn sync_activity_removed(&self, items: Vec<(i64, i64)>) {
        sync::activity_removed_later(&self.server, &self.sync, items);
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

    /// `sidebar.row.upserted` for the membership's own row, for a direct room's sidebar row a
    /// broadcast point outside this type replaced (its members or name changed).
    pub fn sync_membership_row(&self, conn: &Connection, membership_id: i64) {
        sync::membership_row(&self.server, &self.sync, conn, membership_id);
    }

    // The primitives

    /// `broadcast_action_to stream, action:, target:, html:, attributes:` (`maintain_scroll: true`
    /// is the only attribute the app passes). Returns how many subscribers it reached.
    pub fn turbo(
        &self,
        stream: &Stream,
        action: Action,
        target: &str,
        html: Option<&str>,
        maintain_scroll: bool,
    ) -> usize {
        if html.is_some_and(campfire_views::helpers::request_forgery::has_token_slots) {
            tracing::error!("refusing to broadcast an unresolved CSRF token slot");
            return 0;
        }
        let attributes = if maintain_scroll {
            MAINTAIN_SCROLL
        } else {
            &[]
        };
        self.server.broadcast_action_to(
            &stream.streamables(),
            action,
            Target::Target(target),
            html,
            attributes,
        )
    }

    pub fn append(&self, stream: &Stream, target: &str, html: &str) -> usize {
        self.turbo(stream, Action::Append, target, Some(html), false)
    }

    pub fn prepend(&self, stream: &Stream, target: &str, html: &str) -> usize {
        self.turbo(stream, Action::Prepend, target, Some(html), false)
    }

    pub fn replace(&self, stream: &Stream, target: &str, html: &str) -> usize {
        self.turbo(stream, Action::Replace, target, Some(html), false)
    }

    #[allow(dead_code)] // WS14's status and OOO broadcasts use this primitive.
    pub fn update(&self, stream: &Stream, target: &str, html: &str) -> usize {
        self.turbo(stream, Action::Update, target, Some(html), false)
    }

    pub fn remove(&self, stream: &Stream, target: &str) -> usize {
        self.turbo(stream, Action::Remove, target, None, false)
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

    /// A thread was deleted.
    pub fn thread_removed(&self, thread_id: i64, room_id: i64) {
        sync::thread_removed(&self.server, thread_id, room_id);
    }

    /// The person read a thread.
    pub fn thread_read(&self, user_id: i64, thread_id: i64, room_id: i64) {
        sync::thread_read(&self.server, user_id, thread_id, room_id);
    }

    // Message::Broadcasts (reference/app/models/message/broadcasts.rb)

    /// `message.broadcast_create`: append the message to its conversation (`thread || room`),
    /// then, unless it's a thread message or a system note, `broadcast_unread_room`.
    pub fn message_create(
        &self,
        conn: &Connection,
        room: &Room,
        message: &Message,
        partials: &dyn Partials,
        rich_text: &dyn RichText,
    ) -> campfire_db::Result<()> {
        let html = partials.message(message);
        self.append(
            &Stream::conversation(room, message),
            &conversation_messages_target(room, message),
            &html,
        );
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
            sync::sidebar_rows_later(&self.server, &self.sync, room.id, Some(user_ids));
        }
        Ok(())
    }

    /// `message.broadcast_remove`: `broadcast_remove_to message_stream_target, :messages`.
    /// MessagesController#destroy and `User#remove_banned_content`.
    pub fn message_remove(&self, room: &Room, message: &Message) {
        self.remove(
            &Stream::conversation(room, message),
            &message_dom_id(message, None),
        );
        sync::message_removed(&self.server, message);
    }

    /// MessagesController#update: replace `[message, :presentation]` on `[@room, :messages]` (the
    /// room's stream even for a thread message) with `messages/_presentation`, keeping the scroll
    /// position. The controller's other replaces (`:meta` and the card containers) go through
    /// [`Self::message_part_replace`].
    pub fn message_replace(&self, room: &Room, message: &Message, partials: &dyn Partials) {
        let html = partials.message_presentation(message);
        self.message_part_replace(room, message, "presentation", &html);
    }

    /// `@message.broadcast_replace_to @room, :messages, target: [ @message, part ], partial:,
    /// attributes: { maintain_scroll: true }` (MessagesController#update).
    pub fn message_part_replace(&self, room: &Room, message: &Message, part: &str, html: &str) {
        self.turbo(
            &Stream::room_messages(room),
            Action::Replace,
            &message_dom_id(message, Some(part)),
            Some(html),
            true,
        );
        // Every edit replaces the presentation; its other parts don't change the DTO.
        if part == "presentation" {
            sync::message_updated_later(&self.server, &self.sync, message.id);
        }
    }

    /// `broadcast_reactions_replace`: `messages/boosts/_reactions` over `dom_id(message, :boosts)`
    /// on the conversation, keeping the scroll position (Messages::BoostsController).
    #[allow(dead_code)] // WS8 switches the inherited boost broadcasts to this API.
    pub fn message_reactions_replace(&self, room: &Room, message: &Message, html: &str) {
        self.turbo(
            &Stream::conversation(room, message),
            Action::Replace,
            &message_dom_id(message, Some("boosts")),
            Some(html),
            true,
        );
        sync::message_reactions_later(&self.server, &self.sync, message.id);
    }

    /// `broadcast_replace_to` a thread reply's conversation over `dom_id(message, part)`,
    /// keeping the scroll position (ChannelThreadMessagesController#update's edit frames).
    pub fn message_thread_part_replace(&self, room: &Room, message: &Message, part: &str, html: &str) {
        self.turbo(
            &Stream::conversation(room, message),
            Action::Replace,
            &message_dom_id(message, Some(part)),
            Some(html),
            true,
        );
        if part == "presentation" {
            sync::message_updated_later(&self.server, &self.sync, message.id);
        }
    }

    // Messages::BoostsController's `broadcast_create`/`broadcast_remove`

    /// Legacy append frame, retained for the cable wire tests.
    #[cfg(any(test, feature = "test-support"))]
    pub fn boost_create(
        &self,
        room: &Room,
        message: &Message,
        boost: &Boost,
        partials: &dyn Partials,
    ) {
        let html = partials.boost(boost);
        let target = format!("boosts_message_{}", message.client_message_id);
        self.turbo(
            &Stream::conversation(room, message),
            Action::Append,
            &target,
            Some(&html),
            true,
        );
    }

    /// Remove `dom_id(boost)` from the conversation.
    // Current controllers replace grouped reactions. Keep the legacy frame primitive for its wire tests.
    #[cfg(any(test, feature = "test-support"))]
    pub fn boost_remove(&self, room: &Room, message: &Message, boost: &Boost) {
        self.remove(
            &Stream::conversation(room, message),
            &dom_id("boost", boost.id, None),
        );
    }

    // The sidebar's room lists (layouts/application.html.erb streams from `:rooms` and
    // `[Current.user, :rooms]`).

    /// RoomsController#destroy: remove `[room, :list]` from everyone's `:rooms`.
    pub fn room_remove(&self, room: &Room) {
        self.remove(&Stream::rooms(), &room_dom_id(room, "list"));
        sync::room_removed(&self.server, room.id);
    }

    /// Rooms::OpensController#create: prepend to everyone's `shared_rooms`.
    pub fn open_room_create(&self, room: &Room, partials: &dyn Partials) {
        self.prepend(
            &Stream::rooms(),
            "shared_rooms",
            &partials.shared_room(room),
        );
        sync::sidebar_rows_later(&self.server, &self.sync, room.id, None);
    }

    /// Rooms::OpensController#update: replace `[room, :list]` on `:rooms`, then `[room, :header]`
    /// with `rooms/show/header_identity` (`header`, when the caller rendered it). `room` is the
    /// room as an open room (`becomes!(Rooms::Open)`), so the targets name that class even when
    /// the room was closed before.
    pub fn open_room_update(&self, room: &Room, partials: &dyn Partials, header: Option<&str>) {
        self.replace(
            &Stream::rooms(),
            &room_dom_id(room, "list"),
            &partials.shared_room(room),
        );
        if let Some(header) = header {
            self.replace(&Stream::rooms(), &room_dom_id(room, "header"), header);
        }
        sync::sidebar_rows_later(&self.server, &self.sync, room.id, None);
    }

    /// Rooms::ClosedsController#create: render once, prepend to each member's own stream
    /// (`room.users`).
    pub fn closed_room_create(
        &self,
        conn: &Connection,
        room: &Room,
        partials: &dyn Partials,
    ) -> campfire_db::Result<()> {
        let html = partials.shared_room(room);
        let user_ids = room.user_ids(conn)?;
        for &user_id in &user_ids {
            self.prepend(&Stream::user_rooms(user_id), "shared_rooms", &html);
        }
        sync::sidebar_rows(&self.server, &self.sync, conn, room, Some(&user_ids));
        Ok(())
    }

    /// Rooms::ClosedsController#update: after `memberships.revise`, replace `[room, :list]` for
    /// each remaining member (`room` as a closed room), then `[room, :header]` for each.
    pub fn closed_room_update(
        &self,
        conn: &Connection,
        room: &Room,
        partials: &dyn Partials,
        header: Option<&str>,
    ) -> campfire_db::Result<()> {
        let html = partials.shared_room(room);
        let target = room_dom_id(room, "list");
        let user_ids = room.user_ids(conn)?;
        for &user_id in &user_ids {
            self.replace(&Stream::user_rooms(user_id), &target, &html);
        }
        if let Some(header) = header {
            let target = room_dom_id(room, "header");
            for &user_id in &user_ids {
                self.replace(&Stream::user_rooms(user_id), &target, header);
            }
        }
        sync::sidebar_rows(&self.server, &self.sync, conn, room, Some(&user_ids));
        Ok(())
    }

    /// Rooms::DirectsController#create: prepend `users/sidebars/rooms/_direct` to each member's
    /// `direct_rooms`, rendered per membership.
    pub fn direct_room_create(
        &self,
        conn: &Connection,
        room: &Room,
        partials: &dyn Partials,
    ) -> campfire_db::Result<()> {
        for membership in room.memberships(conn)? {
            let html = partials.direct_room(&membership);
            self.prepend(
                &Stream::user_rooms(membership.user_id),
                "direct_rooms",
                &html,
            );
        }
        sync::sidebar_rows(&self.server, &self.sync, conn, room, None);
        Ok(())
    }

    /// Rooms::InvolvementsController#update (`broadcast_visibility_changes`). `previous` is
    /// `involvement_previously_was` (nil reads as no involvement: `nil.to_s.inquiry`).
    pub fn involvement_change(
        &self,
        room: &Room,
        membership: &Membership,
        previous: Option<Involvement>,
        partials: &dyn Partials,
    ) {
        sync::sidebar_rows_later(
            &self.server,
            &self.sync,
            room.id,
            Some(vec![membership.user_id]),
        );
        let stream = Stream::user_rooms(membership.user_id);
        let was = |involvement| previous == Some(involvement);
        let muted_transition =
            membership.involved_in(Involvement::Muted) != was(Involvement::Muted);
        if room.direct() {
            if muted_transition {
                self.replace(
                    &stream,
                    &room_dom_id(room, "list"),
                    &partials.direct_room(membership),
                );
            }
        } else if membership.involved_in(Involvement::Invisible) {
            self.remove(&stream, &room_dom_id(room, "list"));
        } else if was(Involvement::Invisible) {
            let target = if room.stage() {
                "stage_rooms"
            } else if room.voice() {
                "voice_rooms"
            } else if room.board() {
                "board_rooms"
            } else {
                "shared_rooms"
            };
            self.prepend(
                &stream,
                target,
                &partials.sidebar_row(room, membership, None),
            );
        } else if muted_transition {
            let html = partials.sidebar_row(room, membership, Some(membership.unread()));
            self.replace(&stream, &room_dom_id(room, "list"), &html);
        }
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
