//! Consent API for the OAuth screen. Secrets never appear in consent data.
use crate::{PersonalAgent, PersonalAgentRoom, Timestamp};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct OAuthConsent {
    pub client_name: String,
    pub redirect_hostname: String,
    pub badge: OAuthClientBadge,
    pub agents: Vec<PersonalAgent>,
    pub selected_agent_id: Option<i64>,
    pub rooms: Vec<PersonalAgentRoom>,
    pub capabilities: Vec<String>,
    pub enabled: bool,
    pub limit: Option<i64>,
    /// Existing agents expose their current rooms and capabilities for the consent diff.
    pub access_applies_to_all_connections: bool,
    /// What each existing agent can do in each room it's in now, so the page can show exactly
    /// what approval changes room by room. Approval replaces all of it with the chosen rooms,
    /// each getting the chosen capabilities.
    pub room_access: Vec<OAuthAgentRoomAccess>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct OAuthAgentRoomAccess {
    pub agent_id: i64,
    pub room_id: i64,
    /// Named here because the owner may no longer be in the room (it's then not in `rooms`).
    pub room_name: String,
    /// Empty when the agent is in the room without any capability there.
    pub capabilities: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum OAuthClientBadge {
    UnverifiedApp,
    AppOnThisComputer,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ApproveOAuthAuthorization {
    /// Omitted creates a personal agent. Otherwise, only an agent owned by this member.
    pub agent_id: Option<i64>,
    pub name: Option<String>,
    pub icon_name: Option<String>,
    pub room_ids: Vec<i64>,
    pub capabilities: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct OAuthRedirect {
    pub redirect_to: String,
}

/// `GET /api/v1/admin/oauth_clients`: the OAuth clients that have connected, or are blocked.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AdminOAuthClientList {
    pub clients: Vec<AdminOAuthClient>,
    pub oauth_available: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AdminOAuthClient {
    pub id: i64,
    pub client_name: String,
    /// The client's id: the URL of its metadata document.
    pub client_id: String,
    pub client_host: String,
    /// Connections that can still be used (unrevoked and unexpired).
    pub live_connections: i64,
    pub last_connected_at: Option<Timestamp>,
    /// Blocked clients can't connect, and their connections stop working until unblocked.
    pub blocked_at: Option<Timestamp>,
}
