//! Sidebar organisation: the viewer's categories, favourites and notification levels.
//!
//! Ports `room_categories#create/update/destroy` (`crates/rooms/src/controllers/room_categories.rs`,
//! from `app/controllers/room_categories_controller.rb`), `rooms/categories#update`
//! (`rooms/categories_controller.rb`), `rooms/favorites#create/destroy/update`
//! (`rooms/favorites_controller.rb`) and `rooms/involvements#update`
//! (`rooms/involvements_controller.rb`), all under `crates/rooms/src/controllers/rooms/`.
//!
//! Everything here is per person: categories belong to their creator (`room_categories.user_id`)
//! and favourites and involvement live on the viewer's membership. Another person's category is
//! a 404, and so is a room the viewer isn't a member of.
//!
//! What a drag can do, given what the server stores:
//! - reorder **favourites** ([`MoveFavorite`]);
//! - reorder **categories** ([`ReorderRoomCategories`]);
//! - move a channel **into or out of a category** ([`AssignRoomCategory`]) or **into or out of
//!   favourites**.
//!
//! Rooms within a category (and within Channels, Voice and Direct messages) stay in name order:
//! there's no per-room position.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{Involvement, RoomCategory, SidebarRow};

/// `POST /api/v1/room_categories`: add a category at the end (`position` one past the viewer's
/// highest; `RoomCategory::create`). Answers the [`RoomCategory`] (201) and publishes
/// `sidebar.category.upserted`. 422 when the name is blank or over 50 characters
/// (`RoomCategory::NAME_LIMIT`); the classic form silently does nothing instead.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CreateRoomCategory {
    pub name: String,
    /// `null` for `false`.
    pub collapsed: Option<bool>,
}

/// `PATCH /api/v1/room_categories/:id`: rename it or fold it (`room_categories#update`; a
/// `null` field keeps its value). Answers the [`RoomCategory`] and publishes
/// `sidebar.category.upserted`, so folding follows the person to their other tabs. 422 as for
/// [`CreateRoomCategory`].
///
/// `DELETE /api/v1/room_categories/:id` removes it (204; `room_categories#destroy`): its rooms
/// go back to Channels (`memberships.room_category_id` is cleared), each published as
/// `sidebar.row.upserted`, then `sidebar.category.removed`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct UpdateRoomCategory {
    pub name: Option<String>,
    pub collapsed: Option<bool>,
}

/// `PUT /api/v1/room_categories/order`: the viewer's categories in their new order, every one of
/// them exactly once (422 otherwise). Sets `position` to 1, 2, … in this order and answers
/// [`RoomCategoryList`], publishing `sidebar.category.upserted` for each one that moved.
///
/// New: the classic app keeps creation order (`position` is set once and isn't a permitted
/// parameter), though it already sorts by `(position, id)`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ReorderRoomCategories {
    pub category_ids: Vec<i64>,
}

/// The viewer's categories by `(position, id)`: the reply to [`ReorderRoomCategories`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RoomCategoryList {
    pub categories: Vec<RoomCategory>,
}

/// The `sidebar.category.removed` event on the owner's `user` topic. New, like
/// `sidebar.category.upserted` (which carries the [`RoomCategory`]): the classic app has no
/// broadcast for categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RoomCategoryRemoved {
    pub id: i64,
}

/// `PUT /api/v1/rooms/:id/category`: put a room in one of the viewer's categories, or `null`
/// to take it out (`rooms/categories#update`, `Membership#update_category`). Answers the
/// room's [`SidebarRow`] and publishes it as `sidebar.row.upserted`.
///
/// Only open and closed channels can be categorized (422 for direct, voice, stage and board
/// rooms). A favourite keeps its category but shows under Favourites while it's one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AssignRoomCategory {
    pub room_category_id: Option<i64>,
}

/// `PATCH /api/v1/rooms/:id/favorite`: move a favourite to `position`, 0-based among the
/// viewer's favourites (`rooms/favorites#update`, `Membership#move_favorite_to`). The position
/// is clamped to the list, and every favourite is renumbered 0, 1, … Answers
/// [`FavoriteList`] and publishes `sidebar.row.upserted` for each favourite whose position
/// changed. A room that isn't a favourite is left alone (200 with the list unchanged).
///
/// `POST /api/v1/rooms/:id/favorite` adds a room of any kind to the end of the favourites
/// (`Membership#favorite`; already a favourite: unchanged). `DELETE` removes it
/// (`Membership#unfavorite`), leaving a gap in the others' positions: sort favourites by
/// `(favoritePosition, membership.id)`, never index by position. Both answer the room's
/// [`SidebarRow`] and publish it as `sidebar.row.upserted`.
///
/// The classic favourite actions don't broadcast; the JSON twins are new.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct MoveFavorite {
    pub position: i64,
}

/// The viewer's favourites in order after [`MoveFavorite`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct FavoriteList {
    pub rows: Vec<SidebarRow>,
}

/// `PUT /api/v1/rooms/:id/involvement`: the viewer's notification level for a room
/// (`rooms/involvements#update`, `Broadcasts::involvement_change`). Answers the updated
/// [`crate::Membership`].
///
/// Any level is accepted for any room kind (an unknown one is a 422; the classic action fails
/// with a 500). The classic bell cycles direct messages through `everything`, `muted`,
/// `nothing`, and other rooms through `mentions`, `everything`, `muted`, `nothing`,
/// `invisible`; the defaults are `everything` for direct messages and `mentions` otherwise
/// (`RoomType::default_involvement`). "Mute" in the sidebar menu toggles between `muted` and that
/// default.
///
/// Effects, mirroring the classic broadcasts on the person's `[user, :rooms]` stream:
/// - to `invisible`: the row leaves the sidebar (`sidebar.row.removed`);
/// - from `invisible`: it comes back (`sidebar.row.upserted`);
/// - any other change: `sidebar.row.upserted` with the new involvement;
/// - to `muted`: the room is also marked read (`room.read`), and from then on goes unread only
///   when the person is mentioned.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct UpdateInvolvement {
    pub involvement: Involvement,
}
