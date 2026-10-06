//! The room's side panes: members and files (pins live with the other pin types).

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{Attachment, Presence, Timestamp, User};

/// `GET /api/v1/rooms/:id/members` (`rooms/members#index`): every active member, by
/// `(LOWER(name), id)`, in one response. Deactivated members are left out, so the count shown
/// here can be below `RoomDetail.memberCount`, which counts every membership.
///
/// The client groups them as the classic panel does: **Starred** (if any), then **Online**
/// (anything but `offline`), then **Offline**, starred people only in the first. Presence then
/// follows `presence` events; there's no members event (the classic panel polls every 12 s).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct MemberList {
    pub members: Vec<Member>,
    pub users: Vec<User>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Member {
    pub user_id: i64,
    /// Humans: their workspace presence. Bots: `online` while their agent is active (not
    /// suspended and seen), else `offline`.
    pub presence: Presence,
    /// Humans: `status_text_display`. Bots: the agent's working note, its status note, or its
    /// state ("Idle"). `null` for none.
    pub status_text: Option<String>,
    /// The viewer starred this person (`user_stars`); never the viewer's own row.
    pub starred: bool,
}

/// The reply to `PUT` / `DELETE /api/v1/users/:id/star` (`users/stars`): star or unstar
/// someone. Not yourself (422); active humans only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct StarState {
    pub user_id: i64,
    pub starred: bool,
}

/// The `type` filter of `GET /api/v1/rooms/:id/files` (`RoomFiles`'s content-type groups).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum FileType {
    All,
    Images,
    Videos,
    Documents,
    Other,
}

/// `GET /api/v1/rooms/:id/files?type=&filename=&page=` (`rooms/files#index`): files attached
/// to the room's messages, replies included, newest first (`attachments.created_at DESC, id
/// DESC`), 30 a page. `filename` matches anywhere, case-insensitive; `page` starts at 1 and stops
/// at 20. Google Drive attachments aren't listed here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct FileList {
    pub files: Vec<RoomFile>,
    /// The files' posters, once each.
    pub users: Vec<User>,
    /// The page to ask for next; `null` on the last one.
    pub next_page: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RoomFile {
    /// The message it's attached to (open it with the permalink, in its thread when set).
    pub message_id: i64,
    pub thread_id: Option<i64>,
    /// Who posted the message.
    pub creator_id: i64,
    pub attachment: Attachment,
    /// When it was attached.
    pub created_at: Timestamp,
}
