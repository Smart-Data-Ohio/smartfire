//! `TypingNotificationsChannel` (reference/app/channels/typing_notifications_channel.rb): typing
//! in a room, or in one of its threads (`thread_id`) on the thread's own stream. Every `start`
//! and `stop` re-checks the membership (and that the thread is still the room's), so a revoked
//! member's open subscription goes quiet.
use campfire_cable::{Channel, ChannelError, ChannelResult, Params, Subscription};
use campfire_db::{Database, Room};
use rails_compat::global_id::GlobalId;
use serde::Serialize;

use super::{CableUser, room, room_gid, threads};

pub struct TypingNotificationsChannel {
    db: Database,
    room: Option<Room>,
    conversation: Option<Conversation>,
}

/// `@conversation`: the room, or one of its threads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Conversation {
    Room,
    Thread(i64),
}

impl TypingNotificationsChannel {
    pub fn new(db: Database) -> Self {
        Self { db, room: None, conversation: None }
    }

    /// `subscribed`: `@room = find_room`; `@conversation` is its thread `params[:thread_id]` (when
    /// present) or the room; `stream_for @conversation`, or `reject` when there's none.
    async fn subscribe(&mut self, sub: &mut Subscription<CableUser>) -> ChannelResult {
        self.room = room::find_room(&self.db, sub).await?;
        let thread_id = sub.param("thread_id").filter(present);
        self.conversation = match (&self.room, thread_id) {
            (None, _) => None,
            (Some(_), None) => Some(Conversation::Room),
            // `@room.channel_threads.find_by(id: params[:thread_id])`
            (Some(room), Some(thread_id)) => match room::cast_id(&thread_id) {
                Some(thread_id) => {
                    let room_id = room.id;
                    let found = self.db.read(move |conn| threads::in_room(conn, room_id, thread_id)).await?;
                    found.then_some(Conversation::Thread(thread_id))
                }
                None => None,
            },
        };
        match self.conversation_gid() {
            Some(gid) => sub.stream_for(&[&gid.to_param()]),
            None => sub.reject(),
        }
        Ok(())
    }

    fn conversation_gid(&self) -> Option<GlobalId> {
        match (self.conversation?, self.room.as_ref()?) {
            (Conversation::Room, room) => Some(room_gid(room)),
            (Conversation::Thread(thread_id), _) => Some(threads::thread_gid(thread_id)),
        }
    }

    /// `broadcast_typing(action)`: only while the user is still in the room (and the thread still
    /// in it), then `broadcast_to @conversation, action:, user: current_user.slice(:id, :name)`.
    async fn broadcast(&self, action: &'static str, sub: &Subscription<CableUser>) -> ChannelResult {
        let (Some(conversation), Some(gid)) = (self.conversation, self.conversation_gid()) else { return Ok(()) };
        // `@conversation` is only set with `@room`.
        let room_id = self.room.as_ref().map(|room| room.id).ok_or_else(|| ChannelError("undefined method 'id' for nil".into()))?;
        let user_id = sub.current_user().id;
        let allowed = self
            .db
            .read(move |conn| {
                if Room::find_for_user(conn, user_id, room_id)?.is_none() {
                    return Ok(false);
                }
                match conversation {
                    Conversation::Room => Ok(true),
                    Conversation::Thread(thread_id) => threads::in_room(conn, room_id, thread_id),
                }
            })
            .await?;
        if allowed {
            let user = sub.current_user();
            let payload = Payload { action, user: UserAttributes { id: user.id, name: &user.name } };
            sub.broadcast_to(&[&gid.to_param()], &payload);
            if sub.server().sync_wanted() {
                let topic = match conversation {
                    Conversation::Room => crate::cable::sync::room_topic(room_id),
                    Conversation::Thread(thread_id) => format!("thread:{thread_id}"),
                };
                crate::cable::sync::typing(sub.server(), &topic, user.id, action == "start");
            }
        }
        Ok(())
    }
}

/// `params[:thread_id].present?`
fn present(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::Null | serde_json::Value::Bool(false) => false,
        serde_json::Value::String(s) => !s.trim().is_empty(),
        serde_json::Value::Array(a) => !a.is_empty(),
        serde_json::Value::Object(o) => !o.is_empty(),
        _ => true,
    }
}

#[derive(Serialize)]
pub struct Payload<'a> {
    pub action: &'static str,
    pub user: UserAttributes<'a>,
}

#[derive(Serialize)]
pub struct UserAttributes<'a> {
    pub id: i64,
    pub name: &'a str,
}

#[async_trait::async_trait]
impl Channel<CableUser> for TypingNotificationsChannel {
    async fn subscribed(&mut self, sub: &mut Subscription<CableUser>) -> ChannelResult {
        self.subscribe(sub).await
    }

    async fn perform(&mut self, action: &str, _data: &Params, sub: &mut Subscription<CableUser>) -> ChannelResult<bool> {
        if sub.rejected() {
            return Ok(false);
        }
        match action {
            "start" => self.broadcast("start", sub).await?,
            "stop" => self.broadcast("stop", sub).await?,
            "subscribed" => self.subscribe(sub).await?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}
