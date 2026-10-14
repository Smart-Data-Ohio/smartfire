//! Member-owned agents. All secrets appear only in creation responses.
use crate::{BotAvatarSpec, CredentialState, Timestamp};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PersonalAgentList {
    pub agents: Vec<PersonalAgent>,
    pub enabled: bool,
    /// Null for administrators, who have no per-member creation limit.
    pub limit: Option<i64>,
    pub capabilities: Vec<String>,
    pub rooms: Vec<PersonalAgentRoom>,
    /// Null until a valid APP_URL is configured; request headers never supply this URL.
    pub mcp_url: Option<String>,
    pub oauth_available: bool,
}

/// A room the member can grant their agent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PersonalAgentRoom {
    pub id: i64,
    pub name: String,
    /// Not an open room: its member list shows the agent once it joins.
    pub private: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PersonalAgent {
    pub id: i64,
    pub user_id: i64,
    pub name: String,
    pub icon_name: Option<String>,
    /// The bot's avatar icon image, as `User.avatarIcon.imageUrl` resolves it; null for an emoji
    /// or unknown icon.
    pub icon_url: Option<String>,
    /// The picked bot avatar; the client draws it in place of the icon and picture.
    pub bot_avatar: Option<BotAvatarSpec>,
    /// The uploaded picture (or the bot avatar's still render), versioned; null without one.
    pub avatar_url: Option<String>,
    pub provider: Option<String>,
    pub runtime: Option<String>,
    pub description: Option<String>,
    pub daily_message_cap: Option<i64>,
    pub suspended: bool,
    pub room_ids: Vec<i64>,
    pub capabilities: Vec<String>,
    pub last_used_at: Option<Timestamp>,
    pub created_at: Timestamp,
    pub credentials: Vec<PersonalAgentCredential>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum PersonalCredentialKind {
    Token,
    /// An OAuth connection: a client signed in and the owner approved it.
    Oauth,
}

/// An OAuth connection's client, and why the server ended it when the owner didn't.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PersonalAgentConnection {
    pub client_name: String,
    /// The host of the client's id (its metadata document URL).
    pub client_host: String,
    /// The client's id: the exact URL of its metadata document. It identifies the product; the
    /// host alone doesn't.
    pub client_id: String,
    /// An administrator blocked the client: the connection doesn't work until they unblock it.
    pub client_blocked: bool,
    pub revoked_reason: Option<ConnectionRevokedReason>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ConnectionRevokedReason {
    /// A sign-in token (a used refresh token or authorization code) was presented again.
    TokenReused,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PersonalAgentCredential {
    pub id: i64,
    pub kind: PersonalCredentialKind,
    pub name: String,
    pub last_four: String,
    pub created_at: Timestamp,
    pub expires_at: Option<Timestamp>,
    pub last_used_at: Option<Timestamp>,
    pub last_used_ip: Option<String>,
    pub state: CredentialState,
    /// OAuth connections only; null for a token.
    pub connection: Option<PersonalAgentConnection>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PersonalCredentialList {
    pub credentials: Vec<PersonalAgentCredential>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PersonalAgentCreated {
    pub agent: PersonalAgent,
    pub secret: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PersonalCredentialCreated {
    pub credential: PersonalAgentCredential,
    pub secret: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CreatePersonalAgent {
    pub name: String,
    pub icon_name: Option<String>,
    /// A blob uploaded with `POST /api/v1/uploads`: the picture, or `botAvatar`'s still render.
    pub avatar: Option<String>,
    pub bot_avatar: Option<BotAvatarSpec>,
    pub provider: Option<String>,
    pub runtime: Option<String>,
    pub room_ids: Vec<i64>,
    pub capabilities: Vec<String>,
    pub token_name: Option<String>,
    /// Omitted/null defaults to 90 days; "never" has no expiry.
    pub expires_at: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct UpdatePersonalAgent {
    pub name: Option<String>,
    pub icon_name: Option<String>,
    /// A blob uploaded with `POST /api/v1/uploads` becomes the picture. Without `botAvatar` it
    /// replaces the picked bot avatar too.
    pub avatar: Option<String>,
    /// A picked bot avatar, `avatar` carrying its still render.
    pub bot_avatar: Option<BotAvatarSpec>,
    pub provider: Option<String>,
    pub runtime: Option<String>,
    pub description: Option<String>,
    #[serde(default, deserialize_with = "given")]
    #[ts(optional)]
    pub daily_message_cap: Option<Option<i64>>,
}

fn given<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(deserializer).map(Some)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PersonalAgentAccess {
    pub room_ids: Vec<i64>,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct IssuePersonalCredential {
    pub name: String,
    pub expires_at: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RotatePersonalCredential {
    pub name: Option<String>,
    pub expires_at: Option<String>,
}
