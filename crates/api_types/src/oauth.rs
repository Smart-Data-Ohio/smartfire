//! Consent API for the OAuth screen. Secrets never appear in consent data.
use crate::{PersonalAgent, PersonalAgentRoom};
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
