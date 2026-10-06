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
    MessageDTO, MessageRemoved, Presence, RoomRead, RoomUnread, SidebarRow, SidebarRowRemoved,
    SyncPayload, Typing, UserPresence,
};
use campfire_cable::sync::{Audience, SyncPublication};
use campfire_db::{Connection, Database, Membership, Message, Room};

use super::Cable;

/// Builds the twins that need the presenters, which live above this crate.
pub trait SyncRenderer: Send + Sync + 'static {
    /// The message as `GET /api/v1/rooms/:id/messages` serves it.
    fn message(&self, conn: &Connection, message: &Message) -> Option<MessageDTO>;
    /// The membership's sidebar row, or `None` when the room isn't in that sidebar (an
    /// invisible membership, a deleted room).
    fn sidebar_row(
        &self,
        conn: &Connection,
        room: &Room,
        membership: &Membership,
    ) -> campfire_db::Result<Option<SidebarRow>>;
    /// Runs `job` soon with a reader connection, off the caller's thread: for broadcast points
    /// that have no connection at hand (taking a second reader there could wait on the pool).
    fn defer(&self, job: Box<dyn FnOnce(&Connection) + Send>);
}

/// Where [`Broadcasts`](super::Broadcasts) finds the installed [`SyncRenderer`].
#[derive(Clone, Default)]
pub struct RendererSlot(Arc<OnceLock<Arc<dyn SyncRenderer>>>);

impl RendererSlot {
    /// Only the first call takes effect.
    pub fn install(&self, renderer: Arc<dyn SyncRenderer>) {
        let _ = self.0.set(renderer);
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
        &["message.created", "room.unread"],
    ),
    ("Broadcasts::unread_room", &["room.unread"]),
    ("Broadcasts::mark_room_unread", &["room.unread"]),
    ("Broadcasts::message_remove", &["message.removed"]),
    ("Broadcasts::message_replace", &["message.updated"]),
    ("Broadcasts::message_part_replace", &["message.updated"]),
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
    ("TypingNotificationsChannel", &["typing"]),
    // The domain's Turbo and cable frames: appends and replaces of `Partial::Message`, and
    // `user_<id>_unreads`/`user_<id>_reads`. Its other frames (message features, room
    // composition, polls, pins, threads and directory partials) have no twin yet.
    (
        "broadcasts::Broadcast",
        &[
            "message.created",
            "message.updated",
            "room.unread",
            "room.read",
        ],
    ),
    ("RoomRemovalBroadcast", &["sidebar.row.removed"]),
    (
        "user_status_settings::updates::StatusBadgeBroadcast",
        &["presence"],
    ),
    ("user::lifecycle::QuietStreamFinal", &["message.updated"]),
];

/// Broadcast points with no sync event yet: the SPA slices after S1 port them.
pub const NOT_YET_TWINNED: &[&str] = &[
    "Broadcasts::message_reactions_replace",
    "Broadcasts::boost_create",
    "Broadcasts::boost_remove",
    "board_automations::DigestNotes",
    "huddle_effects::StageEndedNote",
    "huddle_effects::StagePanel",
    "huddle_effects::StreamChanged",
    "huddle_effects::StreamStopped",
    "huddle_effects::StageRoster",
    "huddle_effects::RoleEvent",
    "huddle_effects::Presence",
    "user_status_settings::updates::OooNoticeBroadcast",
    "calendar_event::CardUpdate",
    "link_embed::store::CardUpdate",
    "fizzy::cards::CardUpdate",
    "twitter::post::CardUpdate",
    "github::notifier::MessageCreated",
    "github::pull_requests::CardUpdated",
    "agent::AgentStatusChange",
    "agent_step::StepParentChange",
];

/// Sync events the contract defines that no broadcast point publishes yet (the S2 events: their
/// endpoints and twins come with the S2 server work). The coverage test fails when an event is
/// in neither this list nor [`TWINS`], or in both.
pub const NOT_YET_EMITTED: &[&str] = &[
    "message.reactions",
    "message.pinned",
    "thread.indicator",
    "thread.created",
    "thread.updated",
    "thread.removed",
    "thread.unread",
    "thread.read",
    "saved.changed",
];

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
/// pings. Anything else has none.
pub fn cable_stream(server: &Cable, stream: &str, payload: &serde_json::Value) {
    if !server.sync_wanted() {
        return;
    }
    let user_id = |suffix: &str| {
        stream
            .strip_prefix("user_")?
            .strip_suffix(suffix)?
            .parse::<i64>()
            .ok()
    };
    let room_id = |key: &str| payload.get(key).and_then(serde_json::Value::as_i64);
    if let (Some(user_id), Some(room_id)) = (user_id("_unreads"), room_id("roomId")) {
        room_unread(server, user_id, room_id, None, false);
    } else if let (Some(user_id), Some(room_id)) = (user_id("_reads"), room_id("room_id")) {
        room_read(server, user_id, room_id);
    }
}

/// `sidebar.row.upserted` (or `sidebar.row.removed` when the room left their sidebar) for the
/// room's members, or just `user_ids` among them. A member who hid the room is still a member:
/// their connections keep following it, as the classic pages keep streaming it. A row that
/// can't be read is skipped (and logged), not taken for gone.
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
