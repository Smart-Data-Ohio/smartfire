// The Jbuilder views: `messages/_message.json`, `messages/by_bots/{index,show}.json`,
// `messages/boosts/_boost.json` and `messages/boosts/by_bots/show.json`. Field order is the
// JSON key order Jbuilder emits. Serialize with [`crate::helpers::to_rails_json`].

use serde::{Deserialize, Serialize};

/// `users/_user.json.jbuilder`: `json.(user, :id, :name, :role)` and `avatar_url`.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct UserJson {
    pub id: i64,
    pub name: String,
    /// "member", "administrator" or "bot".
    pub role: String,
    /// `fresh_user_avatar_url(user)`: a full URL.
    pub avatar_url: String,
}

/// `messages/_message.json.jbuilder`.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct MessageJson {
    pub id: i64,
    /// `message.created_at.utc`, formatted by [`super::support::json_time`].
    pub created_at: String,
    pub body: MessageBodyJson,
    pub creator: UserJson,
    pub room: IdJson,
    /// `room_message_url(message.room, message)`.
    pub url: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct MessageBodyJson {
    /// `message.plain_text_body`.
    pub plain_text: String,
    /// `message.body.to_s`: the rich text rendered with its layout.
    pub html: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct IdJson {
    pub id: i64,
}

/// `messages/boosts/_boost.json.jbuilder`.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct BoostJson {
    pub id: i64,
    pub content: String,
    /// `boost.created_at.utc`, formatted by [`super::support::json_time`].
    pub created_at: String,
    pub booster: UserJson,
    pub message: BoostMessageJson,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct BoostMessageJson {
    pub id: i64,
    /// `room_message_url(boost.message.room, boost.message)`.
    pub url: String,
}
