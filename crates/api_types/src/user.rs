use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{AgentBadge, Icon, Timestamp};

/// A person or bot as every viewer sees them: the directory entry and profile card.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct User {
    pub id: i64,
    pub name: String,
    pub role: UserRole,
    pub status: UserStatus,
    pub bio: Option<String>,
    /// The avatar image path, versioned so a changed avatar gets a new URL. Always set: without
    /// an uploaded picture it draws the classic initials (or a bot's icon) as an SVG.
    pub avatar_url: String,
    /// Whether the person uploaded a picture (`has_one_attached :avatar`). When `false`, the
    /// client may draw its own initials tile instead of loading `avatarUrl`.
    pub has_avatar: bool,
    /// `null` when unset or expired.
    pub custom_status: Option<CustomStatus>,
    /// A bot without an uploaded avatar shows this icon instead of `avatarUrl`'s default
    /// (`users.icon_name`, resolved as the classic presenters do: a brand logo, else a workspace
    /// icon, else the built-in icon or emoji of that name). `null` for people, for a bot with an
    /// uploaded avatar, and for a name that resolves to nothing.
    pub avatar_icon: Option<Icon>,
    /// Set for an agent (a bot with an `agents` row); `null` for people and for bots without
    /// one, which the classic pages label "Bot". Kept current by `agent.status`.
    pub agent: Option<AgentBadge>,
    pub created_at: Timestamp,
    /// `users.updated_at`: moves on every change to the users row, including status changes
    /// from bans and unbans. UTC with exactly six fractional digits and a `Z` suffix, for example
    /// `2026-10-07T10:15:00.123456Z`. String order equals time order; clients keep whichever copy
    /// of a user has the later value. Whole-second and millisecond rows are padded with zeros.
    pub updated_at: Timestamp,
    /// The legal/account name; `name` is the shared display name.
    pub account_name: String,
    pub pronouns: Option<String>,
}

/// `users.role`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum UserRole {
    Member,
    Administrator,
    Bot,
}

/// `users.status`. Deactivated and banned people still author old messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum UserStatus {
    Active,
    Deactivated,
    Banned,
}

/// The emoji and text a person shows beside their name until it expires.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CustomStatus {
    pub emoji: Option<String>,
    pub text: Option<String>,
    pub expires_at: Option<Timestamp>,
}

/// `GET /api/v1/users?ids=1,2,3`: the directory entries for up to 100 ids, in id order. Unknown
/// ids are left out. The client asks for authors it hasn't seen (a live message from someone new).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct UserList {
    pub users: Vec<User>,
}
