//! Chat bots and agents (S7): `/api/v1/admin/bots/*`, the SPA's twin of the classic chat bot
//! pages (`accounts/bots`, the bot key, the webhook signing secret, the GitHub connection, the kill
//! switch, `accounts/bots/credentials` and `accounts/bots/grants`).
//!
//! Administrators see the list and every control. The person who owns a bot's agent may open
//! that bot, edit it (not its webhook URL), suspend it, reset its signing secret, and revoke its
//! credentials and grants, as on the classic page; anything else is `Forbidden`. Opening a legacy
//! bot's credentials or grants gives it an agent, as the classic pages do.
//!
//! The writes the classic pages guard with a password confirmation answer
//! `ApiError::SudoRequired` when it has lapsed: a new bot, a new key, a webhook URL change, the
//! signing secret, the GitHub connection, and issuing or revoking credentials and grants. A
//! rejected change is `ApiError::Validation` with the classic page's message, its `fields` keyed
//! by the wire names; a refusal the classic page shows as an alert has no `fields` (or names the
//! one input it is about).

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::Timestamp;

/// A bot's icon from the icon set, shown instead of the default avatar when no picture is
/// uploaded.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(export)]
pub enum BotIcon {
    Emoji { title: String, character: String },
    Image { title: String, url: String },
}

/// A room a bot is in, with the commands that post to it as that bot (`BOT_KEY` stands for the
/// key, which is shown only once).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct BotRoom {
    pub id: i64,
    pub name: String,
    /// `curl -d 'Hello!' <url>`.
    pub message_command: String,
    /// `curl -F "attachment=@/path/to/file" <url>`.
    pub attachment_command: String,
}

/// One bot on the list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct BotSummary {
    pub id: i64,
    pub name: String,
    pub avatar_url: String,
    /// Shown instead of `avatar_url` when set (no picture is uploaded).
    pub icon: Option<BotIcon>,
    /// "Workspace agent · Owned by Grace", "no owner recorded", as the classic list reads.
    pub ownership: String,
    pub rooms: Vec<BotRoom>,
}

/// `GET /api/v1/admin/bots`: the active bots, by name. Administrators only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct BotList {
    pub bots: Vec<BotSummary>,
}

/// The agent behind a bot: what it says it is, its daily budgets and today's use of them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct BotAgent {
    pub id: i64,
    pub provider: Option<String>,
    pub runtime: Option<String>,
    pub description: Option<String>,
    /// `null` is unlimited.
    pub daily_message_cap: Option<i64>,
    pub daily_board_post_cap: Option<i64>,
    pub daily_external_action_cap: Option<i64>,
    /// "2/50 messages · 0 board posts · 0 external actions", in the viewer's day.
    pub usage: String,
    /// The kill switch was used: it can't be undone from here.
    pub suspended: bool,
    /// The classic activity ledger and approval requests pages.
    pub ledger_url: String,
    pub approvals_url: String,
}

/// The GitHub account an agent's approved write actions post as.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct BotGithub {
    pub login: String,
    /// GitHub accepts the token. When it doesn't, the account stays linked until replaced.
    pub usable: bool,
    /// Why GitHub stopped accepting it, when known.
    pub disconnected_reason: Option<String>,
}

/// `GET /api/v1/admin/bots/:id`: one bot, as its classic edit page shows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Bot {
    pub id: i64,
    pub name: String,
    pub avatar_url: String,
    /// A picture is uploaded (it wins over the icon).
    pub avatar_attached: bool,
    /// The icon's shortcode, without colons.
    pub icon_name: Option<String>,
    /// `icon_name` resolved, when it names an icon in the set.
    pub icon: Option<BotIcon>,
    pub webhook_url: Option<String>,
    /// The viewer is an administrator: the webhook URL, the key, the GitHub connection,
    /// removal, and issuing credentials and grants are theirs.
    pub can_administer: bool,
    /// `null` for a legacy bot that has never been given an agent.
    pub agent: Option<BotAgent>,
    /// Deliveries carry `X-Smartfire-Signature`; `null` until a secret is generated.
    pub signing_secret: Option<String>,
    /// `null` when no GitHub account is linked.
    pub github: Option<BotGithub>,
}

/// `POST /api/v1/admin/bots` (`accounts/bots#create`): a workspace agent owned by the
/// administrator. `avatar` is a blob uploaded with `POST /api/v1/uploads`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CreateBot {
    pub name: String,
    pub icon_name: Option<String>,
    pub webhook_url: Option<String>,
    pub avatar: Option<String>,
}

/// A bot's key, shown once: the answer to a new bot and to `PUT .../key` (a new key; the
/// old one stops working).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct BotKey {
    pub id: i64,
    pub name: String,
    pub key: String,
    /// `curl -d 'Hello!' <url>`, `ROOM_ID` standing for the room.
    pub example_command: String,
}

/// `PATCH /api/v1/admin/bots/:id` (`accounts/bots#update`). `null` leaves a key as it is; an
/// empty `iconName` or `webhookUrl` clears it. Only administrators may change the webhook URL.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct UpdateBot {
    pub name: Option<String>,
    pub icon_name: Option<String>,
    pub webhook_url: Option<String>,
    /// A blob uploaded with `POST /api/v1/uploads` becomes the picture.
    pub avatar: Option<String>,
    /// Ignored for a legacy bot.
    pub agent: Option<UpdateBotAgent>,
}

/// The agent fields of [`UpdateBot`], as typed: a budget is the text of its input (blank is
/// unlimited), so a bad one is refused with the classic message. `null` leaves a field as it is.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct UpdateBotAgent {
    pub provider: Option<String>,
    pub runtime: Option<String>,
    pub description: Option<String>,
    pub daily_message_cap: Option<String>,
    pub daily_board_post_cap: Option<String>,
    pub daily_external_action_cap: Option<String>,
}

/// The answer to a bot write: the bot afterwards and the classic page's notice, if any. The kill
/// switch (`POST .../kill_switch`), the signing secret (`POST .../webhook_secret`) and the GitHub
/// connection (`PUT` / `DELETE .../github_connection`) answer this too.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct BotChange {
    pub bot: Bot,
    pub notice: Option<String>,
}

/// `DELETE /api/v1/admin/bots/:id`: the bot is deactivated and its agent suspended.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct BotRemoved {
    pub id: i64,
}

/// `PUT /api/v1/admin/bots/:id/github_connection`: a personal access token for the agent's
/// GitHub user, checked with GitHub before it is stored. It is never shown again.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ConnectGithub {
    pub access_token: String,
}

/// Where a credential stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum CredentialState {
    Active,
    Expired,
    Revoked,
}

/// A bearer token for the agent API. Its secret is shown only when it is issued.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Credential {
    pub id: i64,
    pub name: String,
    pub last_four: String,
    pub created_by: String,
    pub created_at: Timestamp,
    pub last_used_at: Option<Timestamp>,
    /// ISO 8601 with the viewer's offset. It can lie far outside the usual years, as typed.
    pub expires_at: Option<String>,
    pub state: CredentialState,
}

/// `GET /api/v1/admin/bots/:id/credentials`, newest first, and the answer to a revocation
/// (`DELETE .../credentials/:credential_id`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CredentialList {
    pub bot_id: i64,
    pub bot_name: String,
    /// Only administrators issue credentials; owners may revoke them.
    pub can_issue: bool,
    pub credentials: Vec<Credential>,
}

/// `POST /api/v1/admin/bots/:id/credentials`. `expiresAt` is a local date and time
/// (`2026-10-31T17:00`) in the viewer's time zone, as the classic input sends it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CreateCredential {
    pub name: String,
    pub expires_at: Option<String>,
}

/// A new credential: its secret, shown once, and the list with it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CredentialCreated {
    pub secret: String,
    pub credentials: CredentialList,
}

/// A capability the agent holds, workspace-wide or in one room.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Grant {
    pub id: i64,
    pub capability: String,
    /// "Workspace-wide", the room's name, or "Deleted room".
    pub room_name: String,
    pub granted_by: String,
    pub created_at: Timestamp,
    pub revoked: bool,
}

/// A room the bot is in, to scope a grant to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GrantRoom {
    pub id: i64,
    pub name: String,
}

/// `GET /api/v1/admin/bots/:id/grants` (active first), and the answer to a new grant and a
/// revocation (`DELETE .../grants/:grant_id`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GrantList {
    pub bot_id: i64,
    pub bot_name: String,
    /// Only administrators grant capabilities; owners may revoke them.
    pub can_grant: bool,
    /// No grant was ever created: the agent can read and post in the rooms it belongs to.
    pub legacy: bool,
    pub grants: Vec<Grant>,
    /// The capabilities a grant can give, in the classic order.
    pub capabilities: Vec<String>,
    pub rooms: Vec<GrantRoom>,
}

/// `POST /api/v1/admin/bots/:id/grants`: `roomId` `null` is workspace-wide (room membership
/// still applies). Granting what the agent already holds changes nothing.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CreateGrant {
    pub capability: String,
    pub room_id: Option<i64>,
}
