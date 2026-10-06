use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Someone's workspace presence as everyone sees it (`UserStatusSettings::effective_presence`
/// over their `workspace_presence_leases`): `invisible` people read as `offline`, and `dnd`
/// applies only while they're connected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum Presence {
    /// Connected and active in the last 10 minutes.
    Online,
    /// Connected, but no activity for 10 minutes.
    Idle,
    Offline,
    /// Connected with do-not-disturb on.
    Dnd,
}

/// One person's presence. A row of `GET /api/v1/presence`, and the `presence` event's data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct UserPresence {
    pub user_id: i64,
    pub presence: Presence,
    /// The status line shown beside the name (`status_text_display`): a custom status, out of
    /// office, in a meeting, and so on; `null` when there's none.
    pub status_text: Option<String>,
}

/// `GET /api/v1/presence?ids=1,2,3`: presence for up to 100 active human users, in id order
/// (`users/presences#show`). Bots, deactivated users and unknown ids are left out. After the
/// first load, `presence` events on the `user` topic keep it current.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PresenceList {
    pub presences: Vec<UserPresence>,
}
