//! Broadcasts the models make (`broadcast_<action>_to`, `Turbo::StreamsChannel.broadcast_*`,
//! `ActionCable.server.broadcast`), described rather than rendered: the database layer knows the
//! stream, the action, the target and what the partial needs, and the app's sink (WS7) renders
//! the partial and delivers the frame through the cable hub. Models emit them as
//! [`Event::Broadcast`](crate::Event::Broadcast), after commit, where Rails broadcasts from an
//! `after_commit` callback (or right after the transaction, where Rails calls the broadcast
//! explicitly once it has committed).
//!
//! Stream names follow Turbo's `stream_name_from`: records are their GlobalID param
//! (`gid://campfire/Rooms::Open/1`), symbols themselves, joined with `:`. Targets are `dom_id`s:
//! STI rooms use their own `param_key` (`messages_rooms_open_1`), and a message's `to_key` is its
//! `client_message_id` (`message_<uuid>`).

use crate::models::{Message, Room, RoomType};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Broadcast {
    /// `broadcast_<action>_to(*streamables, target:, partial:, locals:, attributes:)`
    Turbo(TurboStream),
    /// `ActionCable.server.broadcast(stream, payload)`: a plain channel broadcast, such as
    /// `UnreadThreadsChannel`'s `user_<id>_unread_threads`.
    Cable { stream: String, payload: serde_json::Value },
}

/// One `<turbo-stream>` frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TurboStream {
    pub streamables: Vec<Streamable>,
    pub action: TurboAction,
    /// The `target` DOM id.
    pub target: String,
    /// The partial rendered into the frame's `<template>`; `None` for `remove`.
    pub partial: Option<Partial>,
    /// `attributes: { maintain_scroll: true }`
    pub maintain_scroll: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurboAction {
    Append,
    Prepend,
    Replace,
    Update,
    Remove,
}

/// One part of a stream name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Streamable {
    /// A room, by its STI class (`gid://campfire/Rooms::Open/1`).
    Room { id: i64, room_type: RoomType },
    /// `gid://campfire/ChannelThread/1`
    Thread(i64),
    /// `gid://campfire/User/1`
    User(i64),
    /// A symbol or string: `:messages`, `:rooms`.
    Name(&'static str),
}

impl Streamable {
    /// The GlobalID param (or the name) Turbo joins into the stream name.
    pub fn to_param(&self) -> String {
        match self {
            Streamable::Room { id, room_type } => format!("gid://campfire/{}/{id}", room_type.class_name()),
            Streamable::Thread(id) => format!("gid://campfire/ChannelThread/{id}"),
            Streamable::User(id) => format!("gid://campfire/User/{id}"),
            Streamable::Name(name) => (*name).to_string(),
        }
    }
}

/// The partial a frame renders, with the records it's rendered for. Each names its ERB partial;
/// the sink loads the records it needs and renders it (`ApplicationController.render`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Partial {
    /// `messages/_message` with `message:` (an append of a new message).
    Message { message_id: i64 },
    /// The message's own partial, rendered for `broadcast_replace_to conversation, :messages,
    /// target: message`: `messages/_message`.
    MessageReplace { message_id: i64 },
    /// `messages/_thread_indicator` with `message:` (the parent) and `reply_count:`.
    ThreadIndicator { message_id: i64, reply_count: i64 },
    /// `polls/_poll` with `poll:`.
    Poll { poll_id: i64 },
    /// `messages/_pin_badge` with `message:`.
    PinBadge { message_id: i64 },
    /// `rooms/pins/_count` with `room:`.
    PinsCount { room_id: i64 },
    /// `rooms/pins/_list` with `room:`.
    PinsList { room_id: i64 },
    /// `messages/message_links/_cards` with `message:` (the quoting message).
    QuoteCards { message_id: i64 },
}

/// `dom_id(record, prefix)`
pub fn dom_id(param_key: &str, key: impl std::fmt::Display, prefix: Option<&str>) -> String {
    match prefix {
        Some(prefix) => format!("{prefix}_{param_key}_{key}"),
        None => format!("{param_key}_{key}"),
    }
}

/// `Room.model_name.param_key` for the room's STI class: `Rooms::Open` is `rooms_open`.
pub fn room_param_key(room_type: RoomType) -> String {
    room_type.class_name().replace("::", "_").to_ascii_lowercase()
}

/// `dom_id(room, prefix)`
pub fn room_dom_id(room: &Room, prefix: Option<&str>) -> String {
    dom_id(&room_param_key(room.room_type), room.id, prefix)
}

/// `dom_id(message, prefix)`: a message's `to_key` is its `client_message_id`.
pub fn message_dom_id(message: &Message, prefix: Option<&str>) -> String {
    dom_id("message", &message.client_message_id, prefix)
}

/// `[room, :messages]`
pub fn room_messages(room: &Room) -> Vec<Streamable> {
    vec![Streamable::Room { id: room.id, room_type: room.room_type }, Streamable::Name("messages")]
}

/// `[thread, :messages]`
pub fn thread_messages(thread_id: i64) -> Vec<Streamable> {
    vec![Streamable::Thread(thread_id), Streamable::Name("messages")]
}

/// `[message.conversation, :messages]` (`message_stream_target`): the message's thread, else
/// its room.
pub fn conversation_messages(conn: &rusqlite::Connection, message: &Message) -> crate::Result<Vec<Streamable>> {
    Ok(match message.thread_id {
        Some(thread_id) => thread_messages(thread_id),
        None => room_messages(&Room::find(conn, message.room_id)?),
    })
}

impl Broadcast {
    pub fn replace(streamables: Vec<Streamable>, target: String, partial: Partial) -> Self {
        Broadcast::Turbo(TurboStream { streamables, action: TurboAction::Replace, target, partial: Some(partial), maintain_scroll: false })
    }

    /// A replace with `attributes: { maintain_scroll: true }`.
    pub fn replace_keeping_scroll(streamables: Vec<Streamable>, target: String, partial: Partial) -> Self {
        Broadcast::Turbo(TurboStream { streamables, action: TurboAction::Replace, target, partial: Some(partial), maintain_scroll: true })
    }

    pub fn append(streamables: Vec<Streamable>, target: String, partial: Partial) -> Self {
        Broadcast::Turbo(TurboStream { streamables, action: TurboAction::Append, target, partial: Some(partial), maintain_scroll: false })
    }

    pub fn remove(streamables: Vec<Streamable>, target: String) -> Self {
        Broadcast::Turbo(TurboStream { streamables, action: TurboAction::Remove, target, partial: None, maintain_scroll: false })
    }

    /// The stream name, before signing: the streamables' params joined with `:`.
    pub fn stream_name(&self) -> String {
        match self {
            Broadcast::Turbo(stream) => stream.streamables.iter().map(Streamable::to_param).collect::<Vec<_>>().join(":"),
            Broadcast::Cable { stream, .. } => stream.clone(),
        }
    }

    /// The turbo frame's target, if it's one.
    pub fn target(&self) -> Option<&str> {
        match self {
            Broadcast::Turbo(stream) => Some(&stream.target),
            Broadcast::Cable { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stream_names_join_gid_params_and_symbols() {
        let broadcast = Broadcast::remove(vec![Streamable::Room { id: 7, room_type: RoomType::Open }, Streamable::Name("messages")], "x".into());
        assert_eq!(broadcast.stream_name(), "gid://campfire/Rooms::Open/7:messages");
        assert_eq!(Broadcast::remove(thread_messages(3), "x".into()).stream_name(), "gid://campfire/ChannelThread/3:messages");
    }

    #[test]
    fn room_dom_ids_use_the_sti_param_key() {
        assert_eq!(room_param_key(RoomType::Direct), "rooms_direct");
        assert_eq!(dom_id("message", "abc", Some("thread_indicator")), "thread_indicator_message_abc");
    }
}
