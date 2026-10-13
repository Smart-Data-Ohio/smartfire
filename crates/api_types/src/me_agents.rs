//! Member-owned agents. All secrets appear only in creation responses.
use crate::{CredentialState, GrantRoom, Timestamp};
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
    pub rooms: Vec<GrantRoom>,
    /// Null until a valid APP_URL is configured; request headers never supply this URL.
    pub mcp_url: Option<String>,
    pub oauth_available: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PersonalAgent {
    pub id: i64,
    pub user_id: i64,
    pub name: String,
    pub icon_name: Option<String>,
    pub provider: Option<String>,
    pub runtime: Option<String>,
    pub description: Option<String>,
    pub daily_message_cap: Option<i64>,
    pub suspended: bool,
    pub room_ids: Vec<i64>,
    pub capabilities: Vec<String>,
    pub last_used_at: Option<Timestamp>,
    pub credentials: Vec<PersonalAgentCredential>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum PersonalCredentialKind {
    Token,
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
