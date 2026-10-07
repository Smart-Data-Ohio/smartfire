//! The classic people directory and person page as JSON (S7).

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{Presence, User};

/// `GET /api/v1/people` (`users#index`): everyone active but the viewer, as the classic directory
/// lists them (starred first, then by name, case-insensitively: `presentation::directory`'s order).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PeopleDirectory {
    pub people: Vec<DirectoryPerson>,
    /// The `User` entry for every row (`dto::users`, as `directs/candidates` does).
    pub users: Vec<User>,
}

/// A classic directory row (`users/index.html:27-31`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DirectoryPerson {
    pub user_id: i64,
    /// The classic row's "Online" / "Offline" (`Person.online`).
    pub online: bool,
    /// Starred by the viewer.
    pub starred: bool,
    /// Has an `agents` row (the classic "Agent" badge; a bot without one shows "Bot").
    pub agent: bool,
}

/// `GET /api/v1/people/{id}` (`users#show`): what the classic page shows this viewer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PersonProfile {
    pub user: User,
    /// `profile_status_in_zone`, worded in the request's time zone. Set for a person who isn't
    /// deactivated, including banned people; `None` for bots (`users/show.html:21,45-49`).
    pub status: Option<PersonStatus>,
    /// `Some(allowed)` for an active person other than the viewer (`users/show.html:51` and
    /// `users/statuses/_allowance.html:2-8`); otherwise `None`.
    pub dnd_allowed: Option<bool>,
    /// Administrators only, for a person who isn't deactivated (`users/show.html:45-47`).
    pub email_address: Option<String>,
    /// Administrators only, for an active person (`users/show.html:51,58-61`). The absolute
    /// sign-in transfer URL (`users/profiles/_transfer.html:1`), as `AccountSettings` builds it.
    pub transfer_url: Option<String>,
    /// Administrator, not the viewer, a person, not deactivated (`users/show.html:45,62-65`).
    /// The button bans or removes the ban according to `user.status` (`users/_ban_button.html:1-7`).
    pub can_ban: bool,
}

/// The badge rendered by `users/statuses/_profile_status.html:2`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PersonStatus {
    /// The existing workspace presence.
    pub presence: Presence,
    /// The badge's status line, `None` when there's none.
    pub status_text: Option<String>,
}
