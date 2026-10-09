//! GitHub repository subscriptions and inbound email on a room's settings.
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// One pull-request event the classic subscription form offers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GithubEventChoice {
    pub key: String,
    pub label: String,
    /// Checked on the subscribe form until the person changes it.
    pub selected_by_default: bool,
}

/// A repository whose pull-request events post into the room.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GithubSubscription {
    pub id: i64,
    pub full_name: String,
    pub events: Vec<String>,
}

/// `GET /api/v1/rooms/:room_id/github_subscriptions`.
/// The same people who see the classic section: the room's creator and administrators,
/// on every room except a direct message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GithubSubscriptionList {
    pub subscriptions: Vec<GithubSubscription>,
    pub administrator: bool,
    /// Classic GitHub App OAuth start (`GET /github/app/connect`), or null when the app
    /// is not configured. The callback has no return path back to room settings.
    pub connect_path: Option<String>,
    pub events: Vec<GithubEventChoice>,
}

/// `POST /api/v1/rooms/:room_id/github_subscriptions`.
/// An empty `events` list uses the classic defaults. `skipAccessCheck` is honored only
/// for an administrator, as on the classic form.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export)]
pub struct SubscribeGithubRepository {
    pub full_name: String,
    #[serde(default)]
    pub events: Vec<String>,
    #[serde(default)]
    pub skip_access_check: bool,
}

/// `PATCH /api/v1/rooms/:room_id/github_subscriptions/:id`. At least one known event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export)]
pub struct UpdateGithubSubscription {
    pub events: Vec<String>,
}

/// `GET` and `POST /api/v1/rooms/:room_id/inbound_email`.
/// `enabled` is whether `INBOUND_EMAIL_DOMAIN` is set. `address` is present only then,
/// and only after a token exists. POST creates or rotates the token either way.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct InboundEmail {
    pub enabled: bool,
    pub address: Option<String>,
}
