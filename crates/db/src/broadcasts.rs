//! Domain changes delivered after commit, in callback order.
use serde::{Deserialize, Serialize};

pub fn unread_rooms_stream_name(user_id: i64) -> String { format!("user_{user_id}_unreads") }

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Broadcast {
    MessageCreated { message_id: i64 },
    MessageUpdated { message_id: i64 },
    MessageCards { message_id: i64 },
    MessagePinned { message_id: i64 },
    ThreadIndicator { message_id: i64, reply_count: i64 },
    ThreadRemoved { thread_id: i64, room_id: i64 },
    MembershipChanged { membership_id: i64 },
    UserStatus { user_id: i64 },
    Cable { stream: String, payload: serde_json::Value },
    UnreadRoom { user_ids: Vec<i64>, room_id: i64, message_id: Option<i64>, author_id: i64 },
}
impl Broadcast {
    pub fn channel_frames(&self) -> Vec<(String, serde_json::Value)> {
        match self {
            Self::Cable { stream, payload } => vec![(stream.clone(), payload.clone())],
            Self::UnreadRoom { user_ids, room_id, .. } => user_ids.iter().map(|user_id| (
                unread_rooms_stream_name(*user_id), serde_json::json!({"roomId": room_id}),
            )).collect(),
            _ => Vec::new(),
        }
    }

}
impl crate::events::Broadcast for Broadcast { const KIND: &'static str = "Messaging#broadcast"; }
