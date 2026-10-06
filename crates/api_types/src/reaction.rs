//! Reactions and boosts. Both are `boosts` rows: a row whose content is a single emoji or a known
//! `:icon:` shortcode is a reaction (toggled, grouped into pills); anything else is a free-text
//! boost (kept one by one, shown as chips with the booster's name).

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::Timestamp;

/// One reaction pill: everyone who reacted with the same content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Reaction {
    /// The stored, canonical content: an emoji character (shortcodes like `:tada:` are stored
    /// as the emoji), or `:name:` for a brand or workspace icon. Posting it again toggles.
    pub content: String,
    /// The tooltip name (`ReactionContent.title`): the quick reaction's label, the emoji's name,
    /// or the icon's title.
    pub title: String,
    /// Icons only: the image to draw instead of text (`/icons/:name`, or a brand asset);
    /// `null` for emoji.
    pub image_url: Option<String>,
    /// Distinct reactors, in order of reaction. The pill's count is its length; the viewer's own
    /// id here means "you reacted" (highlighted, and clicking removes it).
    pub reactor_ids: Vec<i64>,
}

/// A free-text boost, e.g. "nice work".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Boost {
    /// For `DELETE /api/v1/messages/:id/boosts/:boostId`, which only its booster may do.
    pub id: i64,
    pub booster_id: i64,
    /// Up to 16 characters, whitespace-trimmed.
    pub content: String,
    pub created_at: Timestamp,
}

/// `POST /api/v1/messages/:id/boosts`: react or boost (`messages/boosts#create`).
///
/// A reaction (a single emoji grapheme or a known `:shortcode:`) toggles: it's removed when the
/// viewer already has it, else added. Any other content adds a free-text boost. Any human member
/// of the message's room may; replies count. Answers [`MessageReactions`] and publishes
/// `message.reactions`. Blank content or more than 16 characters is a 422.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CreateBoost {
    pub content: String,
}

/// A message's reactions and boosts after a change. The reply to
/// `POST /api/v1/messages/:id/boosts` and `DELETE /api/v1/messages/:id/boosts/:boostId` (a
/// reaction is removed by posting it again, never by `DELETE`), and the `message.reactions` event on the
/// message's conversation topic (the JSON twin of `message_reactions_replace`).
///
/// Replaces the message's `reactions`, `boosts` and `updatedAt` when `updatedAt` is newer than
/// the copy held.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct MessageReactions {
    pub message_id: i64,
    pub room_id: i64,
    pub thread_id: Option<i64>,
    pub reactions: Vec<Reaction>,
    pub boosts: Vec<Boost>,
    /// The message's `updated_at` after the change (boosts touch it).
    pub updated_at: Timestamp,
}
