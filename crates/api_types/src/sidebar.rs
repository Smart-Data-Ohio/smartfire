use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{Membership, Room, User};

/// `GET /api/v1/sidebar`: every room in the viewer's sidebar (`users/sidebars#show`), one row
/// per visible membership, plus what the rows refer to.
///
/// The client groups the rows the way the classic sidebar does
/// (`presenters::accounts::sidebar_in_zone`):
/// - **Favourites**: rows whose `membership.favoritePosition` is set, by
///   `(favoritePosition, membership.id)`;
/// - **categories**: each of `categories` in order, holding the non-favourite rows whose
///   `membership.roomCategoryId` is its id;
/// - **Channels**: the other non-favourite rows that aren't direct or voice and have no category;
/// - **Voice**: non-favourite voice rooms;
/// - **Direct messages**: non-favourite direct rooms, newest `room.updatedAt` first.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Sidebar {
    /// One per membership whose involvement isn't `invisible`, in the server's order
    /// (`ORDER BY LOWER(rooms.name)`).
    pub rows: Vec<SidebarRow>,
    /// The viewer's own categories, by `(position, id)`.
    pub categories: Vec<RoomCategory>,
    /// The people the direct rows and `directPlaceholderUserIds` name, once each.
    pub users: Vec<User>,
    /// Up to 20 active people the viewer has no direct room with yet, oldest account first
    /// (`find_direct_placeholder_users`): the "start a conversation" suggestions under the
    /// direct messages.
    pub direct_placeholder_user_ids: Vec<i64>,
    /// Whether the viewer may create channels: an administrator, or anyone when the account
    /// doesn't restrict room creation to administrators.
    pub can_create_rooms: bool,
}

/// A room as it appears in one person's sidebar. Carried by `GET /api/v1/sidebar` and by the
/// `sidebar.row.upserted` event on that person's `user` topic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SidebarRow {
    pub room: Room,
    /// The viewer's membership: involvement (`muted` rows are dimmed), favourite position,
    /// category and read state (`unreadAt` set means unread, shown bold).
    pub membership: Membership,
    /// The row's label: the room's name, or for a direct message its other members' names
    /// (or the room's own name when a group DM was renamed), as `sidebar_direct_label` builds it.
    pub display_name: String,
    /// Direct messages only: the other members in membership order, or just the viewer for a
    /// note-to-self; drives the avatar and presence dot. Empty for every other kind.
    pub direct_member_ids: Vec<i64>,
    /// Root messages from the first unread one to the newest, inclusive (the room page's unread
    /// count); 0 when the room is read.
    pub unread_count: i64,
    /// The viewer's unread `mention` activity items for messages in this room
    /// (`activity_items` with `event_type = 'mention'` and `read_at IS NULL`); drives the numeric
    /// pill.
    pub mention_count: i64,
}

/// `room_categories`: a person's own sidebar section.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RoomCategory {
    pub id: i64,
    pub name: String,
    /// Shown folded.
    pub collapsed: bool,
    /// Sort key among the person's categories (ties by id).
    pub position: i64,
}

/// The `sidebar.row.removed` event: the room left the person's sidebar (they left, were
/// removed, set it invisible, or the room was deleted).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SidebarRowRemoved {
    pub room_id: i64,
}
