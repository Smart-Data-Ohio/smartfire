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
    /// The avatar image path, versioned so a changed avatar gets a new URL.
    pub avatar_url: String,
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
