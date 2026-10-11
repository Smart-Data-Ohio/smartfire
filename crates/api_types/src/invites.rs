use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::Timestamp;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum WorkspaceInviteExpiry {
    #[serde(rename = "30m")]
    ThirtyMinutes,
    #[serde(rename = "1h")]
    OneHour,
    #[serde(rename = "6h")]
    SixHours,
    #[serde(rename = "12h")]
    TwelveHours,
    #[serde(rename = "1d")]
    OneDay,
    #[serde(rename = "7d")]
    SevenDays,
    #[serde(rename = "never")]
    Never,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum WorkspaceInviteState {
    Active,
    Expired,
    Exhausted,
    Revoked,
}

/// `POST /api/v1/admin/invites`. `null` maxUses means unlimited.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CreateWorkspaceInvite {
    pub expiry: WorkspaceInviteExpiry,
    pub max_uses: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct WorkspaceInviteCreator {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct WorkspaceInvite {
    pub id: i64,
    pub creator: WorkspaceInviteCreator,
    pub expires_at: Option<Timestamp>,
    pub max_uses: Option<i64>,
    pub uses: i64,
    pub revoked_at: Option<Timestamp>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
    pub state: WorkspaceInviteState,
}

/// Only the creation response carries the bearer token and shareable URL.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct WorkspaceInviteCreated {
    pub invite: WorkspaceInvite,
    pub token: String,
    pub url: String,
}

/// `GET /api/v1/admin/invites`, newest first. No bearer tokens can be recovered.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct WorkspaceInviteList {
    pub invites: Vec<WorkspaceInvite>,
}
