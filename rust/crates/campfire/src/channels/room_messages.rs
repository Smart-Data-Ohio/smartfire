//! `RoomMessagesChannel` (reference/app/channels/room_messages_channel.rb) and the
//! `RoomStreamsAreAuthorized` guard prepended onto `Turbo::StreamsChannel`
//! (reference/app/channels/concerns/room_streams_are_authorized.rb).
//!
//! A room's or a thread's message stream (`<gid param>:messages`, `<gid param>:threads`) is only
//! served here, and only to a current member of the room the verified stream name points at (a
//! thread's parent room).
use campfire_cable::turbo::StreamsChannel;
use campfire_cable::{Channel, ChannelError, ChannelResult, Params, Subscription};
use campfire_db::{Connection, Database, Room, RoomType};
use rails_compat::global_id::GlobalId;

use super::{CableUser, threads};

pub const STREAM_SUFFIX: &str = "messages";
/// `GUARDED_STREAM_SUFFIXES`
pub const GUARDED_STREAM_SUFFIXES: [&str; 2] = [STREAM_SUFFIX, "threads"];

/// `RoomMessagesChannel.guarded_stream?`: true for the stream names this channel guards, whoever
/// is asking. Also the guard on `Turbo::StreamsChannel`.
pub fn guarded_stream(stream_name: &str) -> bool {
    stream_name.split_once(':').is_some_and(|(_, suffix)| GUARDED_STREAM_SUFFIXES.contains(&suffix))
}

/// `RoomMessagesChannel.subscribable_room(user, stream_name)` (alias `subscribable_target`): the
/// room, among the user's, that the stream's room or thread belongs to.
pub fn subscribable_room(conn: &Connection, user_id: i64, stream_name: &str) -> campfire_db::Result<Option<Room>> {
    let Some((gid_param, suffix)) = stream_name.split_once(':') else {
        return Ok(None);
    };
    if !GUARDED_STREAM_SUFFIXES.contains(&suffix) {
        return Ok(None);
    }
    let room_id = match target_from(conn, gid_param)? {
        Some(Target::Room(room)) => room.id,
        Some(Target::ChannelThread { room_id }) => room_id,
        None => return Ok(None),
    };
    Room::find_for_user(conn, user_id, room_id)
}

enum Target {
    Room(Room),
    ChannelThread { room_id: i64 },
}

/// `GlobalID::Locator.locate gid_param`, with `RecordNotFound` and `NameError` as nil, narrowed
/// to the two targets `subscribable_room` accepts (any other record is nil there too). The name
/// was signed by this app, so it only ever names records the app streams.
fn target_from(conn: &Connection, gid_param: &str) -> campfire_db::Result<Option<Target>> {
    // `GlobalID.parse` takes the URI form or its Base64 param.
    let Some(gid) = GlobalId::parse(gid_param).or_else(|| GlobalId::from_param(gid_param)) else {
        return Ok(None);
    };
    let Ok(id) = gid.id.parse::<i64>() else {
        return Ok(None);
    };
    match gid.model_name.as_str() {
        "ChannelThread" => Ok(threads::room_id_of(conn, id)?.map(|room_id| Target::ChannelThread { room_id })),
        // `Room.find`, or an STI subclass's, whose `find` also requires the type to match.
        name => {
            let required_type = match name {
                "Room" => None,
                name => match RoomType::from_class_name(name) {
                    Some(room_type) => Some(room_type),
                    None => return Ok(None),
                },
            };
            let room = Room::find_by_id(conn, id)?;
            Ok(room.filter(|room| required_type.is_none_or(|t| t == room.room_type)).map(Target::Room))
        }
    }
}

pub struct RoomMessagesChannel {
    db: Database,
    streams: StreamsChannel,
}

impl RoomMessagesChannel {
    pub fn new(db: Database, streams: StreamsChannel) -> Self {
        Self { db, streams }
    }

    /// `authorized_stream_name`: the verified stream name, if present and for a room the user
    /// belongs to.
    async fn authorized_stream_name(&self, sub: &Subscription<CableUser>) -> ChannelResult<Option<String>> {
        let Some(stream_name) = self.streams.verified_stream_name_from_params(&sub.params())? else {
            return Ok(None);
        };
        if stream_name.trim().is_empty() {
            return Ok(None);
        }
        let user_id = sub.current_user().id;
        let name = stream_name.clone();
        let room = self.db.read(move |conn| subscribable_room(conn, user_id, &name)).await.map_err(|error| ChannelError(error.to_string()))?;
        Ok(room.map(|_| stream_name))
    }
}

#[async_trait::async_trait]
impl Channel<CableUser> for RoomMessagesChannel {
    async fn subscribed(&mut self, sub: &mut Subscription<CableUser>) -> ChannelResult {
        match self.authorized_stream_name(sub).await? {
            Some(stream_name) => sub.stream_from(stream_name),
            None => sub.reject(),
        }
        Ok(())
    }

    /// Its public methods: `subscribed`, and `verified_stream_name_from_params` from
    /// `include Turbo::Streams::StreamName::ClassMethods` (which returns without transmitting).
    async fn perform(&mut self, action: &str, _data: &Params, sub: &mut Subscription<CableUser>) -> ChannelResult<bool> {
        if sub.rejected() {
            return Ok(false);
        }
        match action {
            "subscribed" => self.subscribed(sub).await.map(|()| true),
            "verified_stream_name_from_params" => self.streams.verified_stream_name_from_params(&sub.params()).map(|_| true),
            _ => Ok(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guards_only_message_streams() {
        assert!(guarded_stream("Z2lkOi8vY2FtcGZpcmUvUm9vbXM6Ok9wZW4vMQ:messages"));
        assert!(!guarded_stream("rooms"));
        assert!(!guarded_stream("Z2lk:rooms"));
        assert!(!guarded_stream("Z2lk:messages:more"));
        assert!(!guarded_stream(""));
        assert!(guarded_stream(":messages"));
        assert!(guarded_stream("Z2lkOi8vY2FtcGZpcmUvQ2hhbm5lbFRocmVhZC8x:threads"));
        assert!(!guarded_stream("Z2lk:threads:more"));
        assert!(!guarded_stream("Z2lk:status"));
        assert!(!guarded_stream("Z2lk:ooo_notice"));
    }
}
