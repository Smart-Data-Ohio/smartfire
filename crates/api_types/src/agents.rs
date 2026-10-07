//! Agents: who they are, what they're doing, and the approvals they wait on (S4, contract A).
//!
//! Ports the agent directory and profile (`agents/directory#index`, the bot branch of
//! `users#show`; `crates/web/src/controllers/presenters/agents.rs`), the status broadcast
//! (`Agent#broadcast_status_change` on `agents:all`), agent steps (`AgentStep#payload`,
//! `Agents::Steps#broadcast_parent`) and human approval decisions (`agent_approvals#update`,
//! `agents/approvals#for_agent`, `crates/web/src/controllers/presenters/agents/history.rs`).
//!
//! An agent is a bot user with an `agents` row. A bot without one (an integration bot) is
//! `role: "bot"` with `agent: null`, which the classic pages label "Bot" rather than "Agent".
//!
//! Bots and agent tokens never use this API: every endpoint here is a 404 for them, as the
//! classic history pages are (the directory is a 403, as `agents/directory#index` is).

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{AgentApprovalStatus, AgentBudgetCap, Timestamp, User};

/// `agents.status` (`Agent::STATUSES`), which the agent sets through `PATCH /agents/me`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum AgentStatus {
    Idle,
    Working,
    Waiting,
    Failed,
}

/// `agents.kind` (`AgentKind`): owned by one person, or managed for the workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum AgentKind {
    Personal,
    Workspace,
}

/// The agent facts every human sees beside an agent's name ([`User::agent`]): the classic
/// "Agent" badge and `agents/_status_badge.html`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AgentBadge {
    pub agent_id: i64,
    pub kind: AgentKind,
    pub status: AgentStatus,
    /// `agents.suspended_at` is set: the badge reads "Suspended" whatever `status` says.
    pub suspended: bool,
}

/// One agent in the directory (`agent_profile::DirectoryRecord`, `views::agents::DirectoryAgent`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AgentDirectoryRow {
    pub agent_id: i64,
    /// The agent's bot user.
    pub user_id: i64,
    pub kind: AgentKind,
    /// `null` when no owner is recorded ("no owner recorded"). The client words the kind as the
    /// classic row does: "Personal agent of {owner}" or "Workspace agent, managed by {owner}".
    pub owner_id: Option<i64>,
    pub status: AgentStatus,
    /// Up to 200 characters; `null` for none.
    pub status_note: Option<String>,
    pub suspended: bool,
    pub created_at: Timestamp,
    /// "since …" beside the status; `null` until the status first changes.
    pub status_changed_at: Option<Timestamp>,
    /// "last seen …"; `null` reads "never".
    pub last_seen_at: Option<Timestamp>,
}

/// `GET /api/v1/agents`: the agent directory (`agents/directory#index`, HTML only in the classic
/// app). Every human may list it; a bot is a 403.
///
/// Rows are in `Agent::directory_rows` order: active agents (not suspended, user active) first,
/// then by lower-cased name. Agents whose user is deactivated are left out. Not paged: the
/// classic page isn't either, and a workspace has few agents.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AgentDirectory {
    pub agents: Vec<AgentDirectoryRow>,
    /// Every agent's bot user and every owner, once each.
    pub users: Vec<User>,
}

/// One of the seven grantable capabilities (`agent_access::CAPABILITIES`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum AgentCapability {
    ReadMessages,
    PostMessages,
    React,
    ManageThreads,
    ExternalAction,
    Fizzy,
    DmAnyone,
}

/// One line of the grants summary (`Agent#grants_summary`): "{capability} workspace-wide" or
/// "{capability} in {n} rooms".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AgentGrant {
    pub capability: AgentCapability,
    /// A grant with no room (`agent_grants.room_id IS NULL`) is active.
    pub workspace_wide: bool,
    /// Rooms with an active grant; 0 when `workspaceWide`.
    pub room_count: i64,
}

/// What the agent may do, in capability name order. `legacy` is "legacy access (no grants
/// recorded)": an agent with no grant rows reads, posts and reacts wherever it's a member. An
/// empty `grants` without `legacy` is "no active grants".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AgentGrants {
    pub legacy: bool,
    pub grants: Vec<AgentGrant>,
}

/// A room on the profile that the viewer is also a member of.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AgentProfileRoom {
    pub room_id: i64,
    /// The viewer-relative name (`room_display_name`).
    pub name: String,
}

/// The agent's last 24 hours of deliveries (`Agent#activity_summary`): "{delivered} delivered,
/// {acknowledged} acknowledged, {posted} posted, {suppressed} suppressed".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AgentActivitySummary {
    pub delivered: i64,
    pub acknowledged: i64,
    pub posted: i64,
    pub suppressed: i64,
}

/// Today's use of one daily cap (`budget_usage_line`), in the workspace's time zone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AgentBudgetUsage {
    pub cap: AgentBudgetCap,
    pub used: i64,
    /// `null` when the cap isn't set ("{used} messages" rather than "{used}/{limit} messages").
    pub limit: Option<i64>,
}

/// The management section of the profile, for administrators and the agent's owner only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AgentManagement {
    pub activity_summary: AgentActivitySummary,
    /// Messages, board posts and external actions, in that order.
    pub budget_usage: Vec<AgentBudgetUsage>,
}

/// `GET /api/v1/agents/:agentId`: the agent's profile (the bot branch of `users#show`,
/// `presenters::agents::profile`). Any human may read it; an unknown agent is a 404.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AgentProfile {
    pub agent: AgentDirectoryRow,
    /// `null` when blank. The classic page joins these with " · ".
    pub provider: Option<String>,
    pub runtime: Option<String>,
    /// Up to 500 characters; `null` when blank.
    pub description: Option<String>,
    /// The rooms the agent is in that the viewer is in too, by lower-cased name.
    pub rooms: Vec<AgentProfileRoom>,
    /// The agent's other rooms, which the viewer isn't in ("and {n} more").
    pub hidden_room_count: i64,
    pub grants: AgentGrants,
    /// `null` unless the viewer is an administrator or the agent's owner.
    pub management: Option<AgentManagement>,
    /// The bot user and the owner.
    pub users: Vec<User>,
}

/// The `agent.status` event, on every active human's `user` topic (the classic `agents:all`
/// stream, which any signed-in person may follow).
///
/// The JSON twin of `Agent#broadcast_status_change`, which fires when `status` or `statusNote`
/// changes. **New** on top of that, and SPA-only: the event is also published when the agent is
/// suspended or resumed and when its working presence is set, replaced or cleared. Those
/// changes send no classic frame, and the classic frames stay byte-identical (the backend's
/// frame-parity test covers it). One event per change, not one per classic target (badge and
/// directory row).
///
/// The client applies it to the agent's [`User::agent`] badge, its directory row and profile,
/// and the members pane.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AgentStatusChanged {
    pub agent_id: i64,
    pub user_id: i64,
    pub status: AgentStatus,
    pub status_note: Option<String>,
    pub status_changed_at: Option<Timestamp>,
    pub suspended: bool,
    /// What the agent says it's doing now (up to 140 characters); `null` when unset or expired.
    ///
    /// Filled only for a recipient who shares a room with the agent, the audience of the
    /// classic members pane, the one place that shows it; `null` for everyone else.
    pub working_presence: Option<String>,
    /// When `workingPresence` lapses (5 minutes after it was set, `assign_working_presence`).
    /// No event marks the lapse: the client hides the text at this time. `null` when
    /// `workingPresence` is.
    pub working_presence_expires_at: Option<Timestamp>,
}

/// An agent step's state (`AgentStep::STATUSES`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum AgentStepStatus {
    Pending,
    Running,
    Done,
    Failed,
}

/// One step an agent reports while it works (`AgentStep#payload`): shown under the agent's
/// message as "Steps (n)" (`agent_steps/_steps.html`), or on a work thread it owns.
///
/// Steps belong to exactly one parent, a message or a work thread, and are never deleted
/// except with it. So the client merges steps by `id`, keeping the copy with the later
/// `updatedAt` (on a tie, the later arrival). Changing a step doesn't touch its message's
/// `updatedAt`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AgentStep {
    pub id: i64,
    /// Set for a step on a message; `threadId` is `null` then.
    pub message_id: Option<i64>,
    /// Set for a step on a work thread; `messageId` is `null` then.
    pub thread_id: Option<i64>,
    /// Up to 120 characters.
    pub name: String,
    pub status: AgentStepStatus,
    /// "In": up to 1000 characters of plain text; `null` for none.
    pub input_summary: Option<String>,
    /// "Out": as `inputSummary`.
    pub output_summary: Option<String>,
    /// Shown in ms below a second, else in seconds; `null` while unknown.
    pub duration_ms: Option<i64>,
    /// The step's place among its parent's (at most 50); the list is ordered by
    /// `(position, id)`.
    pub position: i64,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

/// The `agent.steps` event: a parent's steps changed (a step was added or updated). The JSON
/// twin of `Agents::Steps#broadcast_parent`, which re-renders the message, or the thread's
/// `agent_steps_channel_thread_<id>` list.
///
/// For a message, on its conversation topic (`room:<id>`, or `thread:<id>` for a reply); for a
/// work thread, on `thread:<id>`. Carries every step of the parent in `(position, id)` order;
/// the client merges them by `updatedAt` (see [`AgentStep`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AgentStepsChanged {
    pub room_id: i64,
    /// The message parent; `null` for a thread's steps.
    pub message_id: Option<i64>,
    /// The thread parent, or the thread a message parent is a reply in; `null` for a message on
    /// the room's root timeline.
    pub thread_id: Option<i64>,
    pub steps: Vec<AgentStep>,
}

/// An approval request as a decider sees it: the classic approval card
/// (`agent_approvals/_card.html`, `history::present_approval`).
///
/// Built per viewer, so it only reaches people who may decide it: administrators and the
/// agent's owner, while both they and the agent's user are active (`AgentApproval#decidable_by`).
/// The request's raw `payload` and `externalId` stay with the agent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AgentApproval {
    pub id: i64,
    pub agent_id: i64,
    /// The agent's bot user, for the card's avatar and name.
    pub agent_user_id: i64,
    /// `null` for a request outside a room.
    pub room_id: Option<i64>,
    /// The viewer-relative room name ("in {room}"); `null` when `roomId` is.
    pub room_name: Option<String>,
    /// The action asked for, e.g. `github.merge_pull_request` (up to 60 characters of
    /// `[a-z0-9_.-]`).
    pub action: String,
    /// Up to 500 characters of plain text.
    pub summary: String,
    /// The effective state: a pending request past `expiresAt` reads `expired`
    /// (`AgentApproval#effective_status`).
    pub status: AgentApprovalStatus,
    pub expires_at: Timestamp,
    pub created_at: Timestamp,
    /// Who approved or denied it; `null` while undecided ("by someone" when the user is gone).
    pub decided_by_id: Option<i64>,
    pub decided_at: Option<Timestamp>,
    /// Up to 200 characters; `null` for none.
    pub decision_note: Option<String>,
    /// `github.*` actions: "Acts on GitHub as @{login}"; `null` otherwise or when unknown.
    pub github_login: Option<String>,
    /// `fizzy.*` actions: "Acts on Fizzy as {name}"; `null` otherwise or when unknown.
    pub fizzy_user_name: Option<String>,
    /// `github.*` and `fizzy.*` actions, which only an administrator may approve. Anyone who may
    /// decide may deny them.
    pub admin_only: bool,
    /// The viewer may approve it (`AgentApproval#approvable_by`). The card then shows Approve;
    /// otherwise "Only an administrator can approve {GitHub|Fizzy} write actions."
    pub approvable: bool,
    /// The viewer may deny it (`AgentApproval#decidable_by`).
    pub deniable: bool,
}

/// `GET /api/v1/agents/:agentId/approvals?status=&before=`: the agent's approval requests,
/// newest first (`agents/approvals#for_agent`, HTML only in the classic app).
///
/// - Only an administrator or the agent's owner, while both they and the agent's user are
///   active; anyone else gets a 404, as in the classic app.
/// - `status`: an [`AgentApprovalStatus`] filter, matched on the effective state; omitted (or
///   unknown) lists every state.
/// - `before`: the previous page's `nextCursor`. A cursor that doesn't decode is a 422
///   (`ApiError::Validation` on `before`).
///
/// At most 50 a page. Listing first settles the overdue requests on the page, as the classic
/// page does.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AgentApprovalPage {
    pub approvals: Vec<AgentApproval>,
    /// The agent's bot user and every `decidedById`, once each.
    pub users: Vec<User>,
    /// Pass as `before` for the next page; `null` when this is the last. Opaque: it encodes the
    /// last row's id, and the next page holds the rows with smaller ids. **New**: the classic
    /// page uses `?page=` offsets.
    pub next_cursor: Option<String>,
}

/// A decider's answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum ApprovalDecision {
    Approved,
    Denied,
}

/// `PATCH /api/v1/agent_approvals/:id`: approve or deny a request (`agent_approvals#update`).
/// Answers the updated [`AgentApproval`] and publishes `approval.updated`.
///
/// The server checks every request, whatever the client showed. Each failure carries the
/// classic JSON `error` text as its message:
/// - `NotFound` unless the viewer may decide it (`decidable_by`), and for bots and agent tokens;
/// - `Validation` on `decision` for an unknown decision (the body doesn't decode);
/// - `Forbidden` when someone other than an administrator approves a `github.*` or `fizzy.*`
///   action;
/// - `Validation` on `base` when the agent's GitHub account (or the owner's Fizzy account)
///   changed since the request ("deny it and ask the agent to request again");
/// - `Validation` on `base` when it's no longer pending (decided, cancelled or expired).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DecideApproval {
    pub decision: ApprovalDecision,
    /// The decision note, up to 200 characters; a blank note is none. Omit for none.
    #[ts(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// The `approval.updated` event. **New**: the classic app has no approval broadcast, so other
/// deciders' cards go stale until they reload.
///
/// Published when a request is approved, denied, cancelled by the agent or settled as expired,
/// on the `user` topic of everyone who received its `agent_approval_request` activity item and
/// can still see it (`ActivityItem::accessible_to`: an administrator or the owner, while active).
/// That's the same audience as [`AgentApprovalPage`], so the payload shows nothing a recipient
/// couldn't already read there. `approval` is built for each recipient (`approvable`,
/// `deniable`, `roomName`).
///
/// The matching `activity.item` (with the new `approvalStatus`) goes to the same people.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ApprovalUpdated {
    pub approval: AgentApproval,
}
