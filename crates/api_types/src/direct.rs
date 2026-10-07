//! Direct messages: one-to-one, group (up to 10 people) and note-to-self rooms.
//!
//! A direct room is **group-capable** when it has more than 2 members or a name
//! (`DirectRoom#group_capable?`); only those can be renamed or grown, by any member. A one-to-one
//! room can never gain members: start a new group instead. Membership changes and renames reach
//! every member as `sidebar.row.upserted` (with the new `displayName` and `directMemberIds`)
//! plus a system note on the room.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::User;

/// `GET /api/v1/directs/candidates` (`rooms/directs#new`): everyone the viewer can message, i.e.
/// active users but themselves, bots included; starred first, then by `LOWER(name)`. The client
/// filters by name as the person types.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DirectCandidateList {
    pub candidates: Vec<DirectCandidate>,
    pub users: Vec<User>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DirectCandidate {
    pub user_id: i64,
    /// An agent (it has an `agents` row): it joins the room but not a huddle.
    pub agent: bool,
    pub starred: bool,
}

/// `POST /api/v1/directs` (`rooms/directs#create`): open the direct room with exactly these
/// people and the viewer, creating it if there's none (matched on the member set, so the same
/// people always get the same room: 200 for an existing one, 201 for a new one). Inactive ids
/// are dropped; an empty list is the note-to-self. More than 9 others is a 422. Answers the
/// viewer's [`crate::SidebarRow`] for it; a new room reaches every member as
/// `sidebar.row.upserted`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CreateDirect {
    /// The other people, the viewer left out.
    pub user_ids: Vec<i64>,
}

/// `POST /api/v1/directs/:id/members` (`rooms/directs#add_members`): add people to a
/// group-capable direct room in place. Answers the updated [`crate::RoomDetail`].
///
/// 422 when the room isn't group-capable or it would pass 10 members. Naming nobody new (people
/// already in it, or nobody active) is a no-op that answers the room unchanged and publishes
/// nothing, as `Room#add_direct_members` treats it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AddDirectMembers {
    pub user_ids: Vec<i64>,
}

/// `PATCH /api/v1/directs/:id` (`rooms/directs#update`): rename a group-capable direct room.
/// Answers the updated [`crate::RoomDetail`]. 422 when it isn't group-capable or the name is
/// over 100 characters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RenameDirect {
    /// Trimmed; blank or `null` clears it, going back to the members' names.
    pub name: Option<String>,
}
