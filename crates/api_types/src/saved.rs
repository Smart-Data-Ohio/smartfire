//! The saved-for-later page: listing and updating the viewer's saved items.
//!
//! Ports `saved_items#index/update` (`crates/campfire/src/controllers/saved_items.rs`, from
//! `app/controllers/saved_items_controller.rb`). Saving and unsaving are
//! [`crate::SaveMessage`] and `DELETE /api/v1/saved/:id` from the S2 contract.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{ConversationName, MessageDTO, SavedItem, SavedStatus, User};

/// The page's status filter (`saved_items#index` `status`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum SavedFilter {
    /// Every item, the default (and what an unknown value reads as).
    All,
    InProgress,
    Done,
}

/// `GET /api/v1/saved?status=&before=`: the viewer's saved items, newest saved first
/// (`created_at DESC, id DESC`), filtered by `status` ([`SavedFilter`]).
///
/// Only items the viewer can still reach are listed (`SavedItem::accessible_to`): they're an
/// active human, and a current member of the message's room, which isn't deleted. An item hidden
/// that way comes back if access does.
///
/// Keyset paging, 50 a page: `before` is the previous page's `nextCursor`. A cursor that
/// doesn't decode is a 422 (`ApiError::Validation` on `before`). New: the classic page lists
/// every item at once.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SavedItemList {
    pub items: Vec<SavedItem>,
    /// The saved messages, one per item, in full (the classic row shows a 500-character
    /// plain-text excerpt; the SPA renders the message).
    pub messages: Vec<MessageDTO>,
    /// The messages' creators, once each.
    pub users: Vec<User>,
    /// The rooms and threads the messages are in.
    pub conversations: Vec<ConversationName>,
    /// Pass as `before` for the next page; `null` on the last (set only when an older row
    /// exists: the server reads 51).
    ///
    /// Opaque to the client: it encodes the last row's `(createdAt, id)`, and the next page
    /// holds the rows strictly after that key in `createdAt DESC, id DESC` order. So it stays
    /// valid when that item is unsaved, moves to another status or becomes unreachable.
    pub next_cursor: Option<String>,
}

/// `PATCH /api/v1/saved/:id`: mark an item done or reopen it (`saved_items#update`, which takes
/// only `saved_item[status]`). Answers the [`SavedItem`] and publishes `saved.changed`. The
/// reminder can't be changed here: `POST /api/v1/saved` again with a new `remindAt` (or `null`
/// to clear it). An item the viewer can't reach is a 404.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct UpdateSavedItem {
    pub status: SavedStatus,
}
