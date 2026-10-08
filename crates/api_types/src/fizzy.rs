use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// One option in the classic flat board select, in Fizzy's response order. No optgroups.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct FizzyBoard {
    pub id: String,
    pub name: String,
}

/// `GET /api/v1/rooms/:room_id[/threads/:thread_id]/messages/:message_id/fizzy_cards/new`.
/// Disconnected viewers still see the source and defaults, with no boards, like classic.
/// The menu item is available on every non-system message, regardless of connection or lock.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct FizzyMessageCardForm {
    pub connected: bool,
    pub boards: Vec<FizzyBoard>,
    pub title: String,
    pub description: String,
    /// Plain text truncated to 280 characters with "...", as classic's blockquote.
    pub excerpt: String,
    pub author_name: String,
    pub room_display_name: String,
    pub fizzy_user_name: String,
    pub account_name: String,
}

/// POST to the form's path without `/new`. Title is stripped as in classic. Only blank board
/// and title are rejected locally. The classic browser limits are 500 and 50000 characters;
/// the server delegates other limits and board access to Fizzy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CreateFizzyCard {
    #[serde(default)]
    pub board_id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub description: String,
}

/// 201: the card and the posted reply, ready to merge into the room or thread timeline.
/// This create is not idempotent, exactly as classic; never automatically retry it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CreatedFizzyCard {
    pub number: String,
    pub url: String,
    pub message: crate::MessageDTO,
    pub notice: String,
}
