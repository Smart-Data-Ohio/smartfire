//! The Slack importer (S7): `/api/v1/admin/slack/*` and `/api/v1/slack/*`, the SPA's twin of the
//! classic importer pages (`accounts/slack_imports`, `accounts/slack_import_runs` and
//! `slack/imports`).
//!
//! Administrators set up the workspace's Slack app, dry-run the workspace import, review its plan
//! and start a test, full or catch-up import. Everyone with a connected Slack account previews and
//! imports their own direct messages, group DMs and private channels. A run's progress is read
//! again from its `status` endpoint while it is active, as the classic page polls it.
//!
//! Connecting a Slack account is an OAuth round trip on the classic routes (`connectPath`); it
//! comes back to the classic page, which opens the SPA's again for someone who uses it.
//!
//! Saving or removing the app credentials and disconnecting a Slack account answer
//! `ApiError::SudoRequired` when the password confirmation has lapsed, as the classic pages ask
//! for it. A refusal the classic page shows as an alert (another import is running, nothing is
//! checked, the run can't be undone) is `ApiError::Validation` with that alert and no `fields`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::Timestamp;

/// A Slack account's connection, as the setup and personal pages describe it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "state", rename_all = "camelCase")]
#[ts(export)]
pub enum SlackConnectionState {
    /// Never connected.
    None,
    Connected,
    /// Slack rejected the grant (revoked or expired, say): reconnecting fixes it.
    Rejected {
        reason: Option<String>,
    },
}

/// Who a run imports for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum SlackRunKind {
    /// The administrator's import of the workspace's channels.
    Workspace,
    /// A person's own direct messages, group DMs and private channels.
    Personal,
}

/// Whether a run writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum SlackRunMode {
    /// A dry run (a preview, for a personal run): reads Slack, writes nothing, and plans.
    DryRun,
    Import,
}

/// Where a run stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum SlackRunStatus {
    Queued,
    Running,
    Undoing,
    Completed,
    Failed,
    Cancelled,
    Undone,
}

/// The run the setup page names while one is active.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SlackRunSummary {
    pub id: i64,
    pub kind: SlackRunKind,
    pub mode: SlackRunMode,
    pub status: SlackRunStatus,
}

/// `GET /api/v1/admin/slack` (`accounts/slack_imports#show`): the setup page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SlackSetup {
    pub client_id: Option<String>,
    /// Both the Client ID and the (write-only) Client Secret are saved.
    pub configured: bool,
    pub configured_by: Option<String>,
    /// The Slack workspace the administrator's connection named.
    pub team_name: Option<String>,
    /// A connection has named the Slack workspace, so runs can start.
    pub team_known: bool,
    /// The administrator's own Slack account.
    pub connection: SlackConnectionState,
    pub active_run: Option<SlackRunSummary>,
    /// The Slack app manifest to create the app from.
    pub manifest: String,
    /// Where "Connect Slack" starts the OAuth round trip (a classic route, a full page load).
    pub connect_path: String,
}

/// `PUT /api/v1/admin/slack` (`accounts/slack_imports#update`). A blank `clientSecret` keeps
/// the saved one.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SaveSlackCredentials {
    pub client_id: String,
    pub client_secret: Option<String>,
}

/// The answer to a setup write: the page afresh and the classic page's notice.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SlackSetupChange {
    pub setup: SlackSetup,
    pub notice: String,
}

/// `DELETE /api/v1/slack/connection` (`slack/connections#destroy`): the classic notice.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SlackDisconnected {
    pub notice: String,
}

/// One run on a list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SlackRunRow {
    pub id: i64,
    pub kind: SlackRunKind,
    pub mode: SlackRunMode,
    pub status: SlackRunStatus,
    pub started_by: String,
    pub created_at: Timestamp,
}

/// `GET /api/v1/admin/slack/runs` (`accounts/slack_import_runs#index`): every run, newest first.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SlackRunList {
    pub runs: Vec<SlackRunRow>,
}

/// `GET /api/v1/slack/imports` (`slack/imports#index`): a person's connection and their runs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SlackPersonal {
    /// An administrator has set up the import (a connection named the Slack workspace).
    pub team_known: bool,
    pub connection: SlackConnectionState,
    pub connect_path: String,
    pub runs: Vec<SlackRunRow>,
}

/// The people a run found in Slack.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SlackPeople {
    pub total: i64,
    /// Matched to an account here by email.
    pub matched: i64,
    /// Given a claimable placeholder account.
    pub placeholders: i64,
    pub deactivated: i64,
    pub bots: i64,
}

/// What a run has brought over so far.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SlackCounts {
    pub rooms_created: i64,
    pub rooms_merged: i64,
    pub messages: i64,
    pub replies: i64,
    pub threads: i64,
    pub reactions: i64,
    pub pins: i64,
    pub files_linked: i64,
    pub skipped: i64,
}

/// A Slack conversation a dry run found.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SlackConversation {
    /// Slack's id (`C024BE91L`), what an import checks.
    pub id: String,
    pub name: String,
    /// "Public channel", "Private channel", "Direct message", "Group DM".
    pub kind: String,
    pub archived: bool,
    pub members: i64,
    pub messages: i64,
    pub threads: i64,
}

/// `GET .../runs/:id/status`: one run's progress, polled while it is active.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SlackRun {
    pub id: i64,
    pub kind: SlackRunKind,
    pub mode: SlackRunMode,
    pub status: SlackRunStatus,
    /// "Workspace dry run #12", as the classic page heads it.
    pub title: String,
    pub started_by: String,
    pub created_at: Timestamp,
    pub started_at: Option<Timestamp>,
    pub finished_at: Option<Timestamp>,
    /// The step it is on ("users", "conversations", "undo").
    pub phase: Option<String>,
    /// What it is working on within the step.
    pub current: Option<String>,
    /// Queued behind another import, which has to finish first.
    pub queued_behind: bool,
    pub people: Option<SlackPeople>,
    pub counts: Option<SlackCounts>,
    pub api_calls: Option<i64>,
    pub issues_count: i64,
    pub error: Option<String>,
    /// Queued, running or undoing: its status is worth reading again.
    pub active: bool,
    pub cancellable: bool,
    pub undoable: bool,
    /// Why undo is blocked (a later import touched the same conversations, another run is
    /// active), shown beside the disabled button.
    pub undo_blocked_reason: Option<String>,
    /// A completed workspace dry run: its plan can be reviewed and imported.
    pub plan_ready: bool,
    /// A completed full workspace import: a catch-up import can run.
    pub catch_up: bool,
    /// A completed personal preview's conversations, to check for the import (else empty).
    pub conversations: Vec<SlackConversation>,
}

/// A problem a run recorded.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SlackIssue {
    /// "warning" or "error".
    pub level: String,
    /// The Slack object it is about (a channel or message id), if any.
    pub slack_ref: Option<String>,
    pub message: String,
}

/// `GET /api/v1/admin/slack/runs/:id?page=` (`accounts/slack_import_runs#show`): the run and a
/// page of its issues, 50 at a time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SlackRunPage {
    pub run: SlackRun,
    pub issues: Vec<SlackIssue>,
    /// The next page of older issues, if any.
    pub next_page: Option<i64>,
}

/// The answer to starting, cancelling or undoing a run: the run it is about now and the classic
/// page's notice.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SlackRunChange {
    pub run: SlackRun,
    pub notice: String,
}

/// One conversation in a workspace plan, with where it goes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SlackPlanConversation {
    pub conversation: SlackConversation,
    /// `"new"` (a room named after it), `"skip"`, or the id (as text) of a room to merge into.
    pub target: String,
}

/// A room a conversation can merge into.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SlackRoomTarget {
    pub id: i64,
    pub name: String,
}

/// A message converted from Slack's markup, to check the conversion before importing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SlackSample {
    pub conversation: String,
    pub slack_text: String,
    /// The message as Smartfire will show it (sanitized HTML).
    pub html: String,
}

/// `GET /api/v1/admin/slack/runs/:id/plan` (`accounts/slack_import_runs#plan`): a completed
/// dry run's plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SlackPlan {
    pub run_id: i64,
    pub conversations: Vec<SlackPlanConversation>,
    /// The open and closed rooms, by name.
    pub rooms: Vec<SlackRoomTarget>,
    pub samples: Vec<SlackSample>,
    /// A test import's default oldest day (`YYYY-MM-DD`, two weeks back).
    pub default_oldest: String,
}

/// `POST /api/v1/admin/slack/runs` (`accounts/slack_import_runs#create`): a workspace dry run.
/// The dates are `YYYY-MM-DD` days in the administrator's time zone.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct StartSlackDryRun {
    pub include_private: bool,
    pub oldest: Option<String>,
    pub latest: Option<String>,
}

/// A workspace import's reach.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum SlackPreset {
    /// The checked conversations' recent messages (between `oldest`, two weeks back by default,
    /// and `latest`).
    Test,
    /// Everything in the checked conversations.
    Full,
}

/// `POST /api/v1/admin/slack/runs/:id/import` (`accounts/slack_import_runs#start_import`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct StartSlackImport {
    pub conversation_ids: Vec<String>,
    /// Each checked conversation's target, as [`SlackPlanConversation::target`] spells it.
    pub room_targets: BTreeMap<String, String>,
    pub preset: SlackPreset,
    pub oldest: Option<String>,
    pub latest: Option<String>,
}

/// `POST /api/v1/slack/imports` (`slack/imports#create`): a preview, or an import of a
/// completed preview's checked conversations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct StartPersonalSlackImport {
    pub mode: SlackRunMode,
    /// The preview an import comes from.
    pub dry_run_id: Option<i64>,
    pub conversation_ids: Vec<String>,
}
