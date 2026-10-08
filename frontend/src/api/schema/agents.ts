import { Schema } from "effect";
import type { AgentActivitySummary as GeneratedAgentActivitySummary } from "../../gen/AgentActivitySummary.ts";
import type { AgentApproval as GeneratedAgentApproval } from "../../gen/AgentApproval.ts";
import type { AgentApprovalPage as GeneratedAgentApprovalPage } from "../../gen/AgentApprovalPage.ts";
import type { AgentBudgetUsage as GeneratedAgentBudgetUsage } from "../../gen/AgentBudgetUsage.ts";
import type { AgentCapability as GeneratedAgentCapability } from "../../gen/AgentCapability.ts";
import type { AgentDeliveryOutcome as GeneratedAgentDeliveryOutcome } from "../../gen/AgentDeliveryOutcome.ts";
import type { AgentDirectory as GeneratedAgentDirectory } from "../../gen/AgentDirectory.ts";
import type { AgentDirectoryRow as GeneratedAgentDirectoryRow } from "../../gen/AgentDirectoryRow.ts";
import type { AgentExternalResult as GeneratedAgentExternalResult } from "../../gen/AgentExternalResult.ts";
import type { AgentGrant as GeneratedAgentGrant } from "../../gen/AgentGrant.ts";
import type { AgentGrants as GeneratedAgentGrants } from "../../gen/AgentGrants.ts";
import type { AgentLedgerEvent as GeneratedAgentLedgerEvent } from "../../gen/AgentLedgerEvent.ts";
import type { AgentLedgerEventType as GeneratedAgentLedgerEventType } from "../../gen/AgentLedgerEventType.ts";
import type { AgentLedgerPage as GeneratedAgentLedgerPage } from "../../gen/AgentLedgerPage.ts";
import type { AgentManagement as GeneratedAgentManagement } from "../../gen/AgentManagement.ts";
import type { AgentProfile as GeneratedAgentProfile } from "../../gen/AgentProfile.ts";
import type { AgentProfileRoom as GeneratedAgentProfileRoom } from "../../gen/AgentProfileRoom.ts";
import type { AgentStatusChanged as GeneratedAgentStatusChanged } from "../../gen/AgentStatusChanged.ts";
import type { AgentStep as GeneratedAgentStep } from "../../gen/AgentStep.ts";
import type { AgentStepStatus as GeneratedAgentStepStatus } from "../../gen/AgentStepStatus.ts";
import type { AgentStepsChanged as GeneratedAgentStepsChanged } from "../../gen/AgentStepsChanged.ts";
import type { AgentWebhookStatus as GeneratedAgentWebhookStatus } from "../../gen/AgentWebhookStatus.ts";
import type { ApprovalDecision as GeneratedApprovalDecision } from "../../gen/ApprovalDecision.ts";
import type { ApprovalUpdated as GeneratedApprovalUpdated } from "../../gen/ApprovalUpdated.ts";
import type { DecideApproval as GeneratedDecideApproval } from "../../gen/DecideApproval.ts";
import { AgentApprovalStatus, AgentBudgetCap } from "./activity.ts";
import { AgentKind, AgentStatus } from "./agent-identity.ts";
import {
  AgentApprovalId,
  AgentEventId,
  AgentId,
  AgentStepId,
  MessageId,
  RoomId,
  ThreadId,
  UserId,
} from "./ids.ts";
import type { Assert, Pinned } from "./pin.ts";
import { RowTimestamp, Timestamp } from "./time.ts";
import { tolerantLiterals } from "./tolerant.ts";
import { User } from "./user.ts";

export { AgentBadge, AgentKind, AgentStatus } from "./agent-identity.ts";

/** One agent in the directory. `ownerId` is `null` when no owner is recorded. */
export const AgentDirectoryRow = Schema.Struct({
  agentId: AgentId,
  userId: UserId,
  kind: AgentKind,
  ownerId: Schema.NullOr(UserId),
  status: AgentStatus,
  statusNote: Schema.NullOr(Schema.String),
  suspended: Schema.Boolean,
  createdAt: Timestamp,
  statusChangedAt: Schema.NullOr(Timestamp),
  lastSeenAt: Schema.NullOr(Timestamp),
  updatedAt: RowTimestamp,
});

export type AgentDirectoryRow = typeof AgentDirectoryRow.Type;

export type AgentDirectoryRowPin = Assert<
  Pinned<typeof AgentDirectoryRow, GeneratedAgentDirectoryRow>
>;

/** `GET /api/v1/agents`: active agents first, then by name; every bot user and owner once. */
export const AgentDirectory = Schema.Struct({
  agents: Schema.Array(AgentDirectoryRow),
  users: Schema.Array(User),
});

export type AgentDirectory = typeof AgentDirectory.Type;

export type AgentDirectoryPin = Assert<Pinned<typeof AgentDirectory, GeneratedAgentDirectory>>;

export const AgentCapability = Schema.Literals([
  "read_messages",
  "post_messages",
  "react",
  "manage_threads",
  "external_action",
  "fizzy",
  "dm_anyone",
]);

export type AgentCapability = typeof AgentCapability.Type;

export type AgentCapabilityPin = Assert<Pinned<typeof AgentCapability, GeneratedAgentCapability>>;

export const AgentGrant = Schema.Struct({
  capability: AgentCapability,
  workspaceWide: Schema.Boolean,
  roomCount: Schema.Int,
});

export type AgentGrant = typeof AgentGrant.Type;

export type AgentGrantPin = Assert<Pinned<typeof AgentGrant, GeneratedAgentGrant>>;

/** `legacy`: no grant rows, so it reads, posts and reacts wherever it's a member. */
export const AgentGrants = Schema.Struct({
  legacy: Schema.Boolean,
  grants: Schema.Array(AgentGrant),
});

export type AgentGrants = typeof AgentGrants.Type;

export type AgentGrantsPin = Assert<Pinned<typeof AgentGrants, GeneratedAgentGrants>>;

export const AgentProfileRoom = Schema.Struct({ roomId: RoomId, name: Schema.String });

export type AgentProfileRoom = typeof AgentProfileRoom.Type;

export type AgentProfileRoomPin = Assert<
  Pinned<typeof AgentProfileRoom, GeneratedAgentProfileRoom>
>;

export const AgentActivitySummary = Schema.Struct({
  delivered: Schema.Int,
  acknowledged: Schema.Int,
  posted: Schema.Int,
  suppressed: Schema.Int,
});

export type AgentActivitySummary = typeof AgentActivitySummary.Type;

export type AgentActivitySummaryPin = Assert<
  Pinned<typeof AgentActivitySummary, GeneratedAgentActivitySummary>
>;

/** Today's use of one daily cap; `limit` is `null` when the cap isn't set. */
export const AgentBudgetUsage = Schema.Struct({
  cap: AgentBudgetCap,
  used: Schema.Int,
  limit: Schema.NullOr(Schema.Int),
});

export type AgentBudgetUsage = typeof AgentBudgetUsage.Type;

export type AgentBudgetUsagePin = Assert<
  Pinned<typeof AgentBudgetUsage, GeneratedAgentBudgetUsage>
>;

export const AgentManagement = Schema.Struct({
  activitySummary: AgentActivitySummary,
  budgetUsage: Schema.Array(AgentBudgetUsage),
});

export type AgentManagement = typeof AgentManagement.Type;

export type AgentManagementPin = Assert<Pinned<typeof AgentManagement, GeneratedAgentManagement>>;

/**
 * `GET /api/v1/agents/:agentId`. `grants` and `management` are for administrators and the owner
 * only (`grants` departs from classic, which shows it to everyone).
 */
export const AgentProfile = Schema.Struct({
  agent: AgentDirectoryRow,
  provider: Schema.NullOr(Schema.String),
  runtime: Schema.NullOr(Schema.String),
  description: Schema.NullOr(Schema.String),
  rooms: Schema.Array(AgentProfileRoom),
  hiddenRoomCount: Schema.Int,
  grants: Schema.NullOr(AgentGrants),
  management: Schema.NullOr(AgentManagement),
  users: Schema.Array(User),
});

export type AgentProfile = typeof AgentProfile.Type;

export type AgentProfilePin = Assert<Pinned<typeof AgentProfile, GeneratedAgentProfile>>;

/**
 * The `agent.status` event. `workingPresence` is filled only for people who share a room with
 * the agent; the client hides it at `workingPresenceExpiresAt` (no event marks the lapse).
 */
export const AgentStatusChanged = Schema.Struct({
  agentId: AgentId,
  userId: UserId,
  status: AgentStatus,
  statusNote: Schema.NullOr(Schema.String),
  statusChangedAt: Schema.NullOr(Timestamp),
  suspended: Schema.Boolean,
  workingPresence: Schema.NullOr(Schema.String),
  workingPresenceExpiresAt: Schema.NullOr(Timestamp),
  updatedAt: RowTimestamp,
});

export type AgentStatusChanged = typeof AgentStatusChanged.Type;

export type AgentStatusChangedPin = Assert<
  Pinned<typeof AgentStatusChanged, GeneratedAgentStatusChanged>
>;

/** Where a step is. Tolerant: a status added after this build decodes to `"unknown"`. */
export const AgentStepStatus = tolerantLiterals(["pending", "running", "done", "failed"]);

export type AgentStepStatus = typeof AgentStepStatus.Type;

export type AgentStepStatusPin = Assert<Pinned<typeof AgentStepStatus, GeneratedAgentStepStatus>>;

/**
 * One step an agent reports, on a message or a work thread. Steps are never deleted apart from
 * with their parent, so the client merges them by `id`, keeping the later `updatedAt` (on a tie,
 * the later arrival).
 */
export const AgentStep = Schema.Struct({
  id: AgentStepId,
  messageId: Schema.NullOr(MessageId),
  threadId: Schema.NullOr(ThreadId),
  name: Schema.String,
  status: AgentStepStatus,
  inputSummary: Schema.NullOr(Schema.String),
  outputSummary: Schema.NullOr(Schema.String),
  durationMs: Schema.NullOr(Schema.Int),
  position: Schema.Int,
  createdAt: Timestamp,
  updatedAt: Timestamp,
});

export type AgentStep = typeof AgentStep.Type;

export type AgentStepPin = Assert<Pinned<typeof AgentStep, GeneratedAgentStep>>;

/** The `agent.steps` event: every step of one parent, in `(position, id)` order. */
export const AgentStepsChanged = Schema.Struct({
  roomId: RoomId,
  messageId: Schema.NullOr(MessageId),
  threadId: Schema.NullOr(ThreadId),
  steps: Schema.Array(AgentStep),
});

export type AgentStepsChanged = typeof AgentStepsChanged.Type;

export type AgentStepsChangedPin = Assert<
  Pinned<typeof AgentStepsChanged, GeneratedAgentStepsChanged>
>;

/**
 * An approval request as this viewer sees it. `approvable` and `deniable` are the viewer's own
 * rights, meaningful only while `status` is `pending`; the server checks them again on every
 * decision. `decidedById` stays set after the decider's account is deleted ("by someone" when
 * no user matches).
 */
export const AgentApproval = Schema.Struct({
  id: AgentApprovalId,
  agentId: AgentId,
  agentUserId: UserId,
  roomId: Schema.NullOr(RoomId),
  roomName: Schema.NullOr(Schema.String),
  action: Schema.String,
  summary: Schema.String,
  status: AgentApprovalStatus,
  expiresAt: Timestamp,
  createdAt: Timestamp,
  decidedById: Schema.NullOr(UserId),
  decidedAt: Schema.NullOr(Timestamp),
  decisionNote: Schema.NullOr(Schema.String),
  githubLogin: Schema.NullOr(Schema.String),
  fizzyUserName: Schema.NullOr(Schema.String),
  adminOnly: Schema.Boolean,
  approvable: Schema.Boolean,
  deniable: Schema.Boolean,
  updatedAt: RowTimestamp,
});

export type AgentApproval = typeof AgentApproval.Type;

export type AgentApprovalPin = Assert<Pinned<typeof AgentApproval, GeneratedAgentApproval>>;

/**
 * `GET /api/v1/agents/:agentId/approvals?status=&before=`: newest first, 50 a page. A new
 * request publishes no `approval.updated`: refetch the first page on an `agent_approval_request`
 * `activity.item`, and whenever the page is shown again.
 */
export const AgentApprovalPage = Schema.Struct({
  approvals: Schema.Array(AgentApproval),
  users: Schema.Array(User),
  nextCursor: Schema.NullOr(Schema.String),
});

export type AgentApprovalPage = typeof AgentApprovalPage.Type;

export type AgentApprovalPagePin = Assert<
  Pinned<typeof AgentApprovalPage, GeneratedAgentApprovalPage>
>;

export const ApprovalDecision = Schema.Literals(["approved", "denied"]);

export type ApprovalDecision = typeof ApprovalDecision.Type;

export type ApprovalDecisionPin = Assert<
  Pinned<typeof ApprovalDecision, GeneratedApprovalDecision>
>;

/** `PATCH /api/v1/agent_approvals/:id`. `note` is the classic `decision_note`; leave it out for none. */
export const DecideApproval = Schema.Struct({
  decision: ApprovalDecision,
  note: Schema.optionalKey(Schema.String),
});

export type DecideApproval = typeof DecideApproval.Type;

export type DecideApprovalPin = Assert<Pinned<typeof DecideApproval, GeneratedDecideApproval>>;

/**
 * The `approval.updated` event, on the `user` topic of everyone who got the request's item: a
 * subset of the page's audience, so a page refetches when shown again and after `sync.reset`.
 * `users` holds the agent's bot user and the decider.
 */
export const ApprovalUpdated = Schema.Struct({
  approval: AgentApproval,
  users: Schema.Array(User),
});

export type ApprovalUpdated = typeof ApprovalUpdated.Type;

export type ApprovalUpdatedPin = Assert<Pinned<typeof ApprovalUpdated, GeneratedApprovalUpdated>>;

/** What a ledger entry records. Tolerant: a type added after this build decodes to `"unknown"`. */
export const AgentLedgerEventType = tolerantLiterals([
  "mention",
  "direct_message",
  "reply",
  "approval_decided",
  "github_action_completed",
  "fizzy_action_completed",
  "work_assigned",
  "work_unassigned",
  "work_handed_off",
  "slash_command",
  "posted",
  "delivery_suppressed_rate_limit",
  "delivery_suppressed_hop_limit",
  "delivery_suppressed_revoked",
]);

export type AgentLedgerEventType = typeof AgentLedgerEventType.Type;

export type AgentLedgerEventTypePin = Assert<
  Pinned<typeof AgentLedgerEventType, GeneratedAgentLedgerEventType>
>;

/** A delivery's outcome; the ledger's filter. Tolerant, like `AgentLedgerEventType`. */
export const AgentDeliveryOutcome = tolerantLiterals([
  "pending",
  "delivered",
  "acknowledged",
  "suppressed",
]);

export type AgentDeliveryOutcome = typeof AgentDeliveryOutcome.Type;

export type AgentDeliveryOutcomePin = Assert<
  Pinned<typeof AgentDeliveryOutcome, GeneratedAgentDeliveryOutcome>
>;

/** Whether the entry was pushed to the agent's webhook (`none` shows no webhook line). */
export const AgentWebhookStatus = tolerantLiterals(["none", "pending", "delivered", "failed"]);

export type AgentWebhookStatus = typeof AgentWebhookStatus.Type;

export type AgentWebhookStatusPin = Assert<
  Pinned<typeof AgentWebhookStatus, GeneratedAgentWebhookStatus>
>;

/** A GitHub or Fizzy action's result: "GitHub {action}: {status} — {message}". */
export const AgentExternalResult = Schema.Struct({
  action: Schema.NullOr(Schema.String),
  status: Schema.NullOr(Schema.String),
  message: Schema.NullOr(Schema.String),
});

export type AgentExternalResult = typeof AgentExternalResult.Type;

export type AgentExternalResultPin = Assert<
  Pinned<typeof AgentExternalResult, GeneratedAgentExternalResult>
>;

/**
 * One ledger entry. `content` is the message's plain text, cut to 140 characters, only when the
 * agent may read the room and the viewer is an administrator or a member of it; `null` reads
 * "Content unavailable" when there's a `messageId`.
 */
export const AgentLedgerEvent = Schema.Struct({
  id: AgentEventId,
  eventType: AgentLedgerEventType,
  outcome: Schema.NullOr(AgentDeliveryOutcome),
  createdAt: Timestamp,
  roomId: Schema.NullOr(RoomId),
  roomName: Schema.NullOr(Schema.String),
  actorId: Schema.NullOr(UserId),
  messageId: Schema.NullOr(MessageId),
  hop: Schema.Int,
  detail: Schema.NullOr(Schema.String),
  webhookStatus: AgentWebhookStatus,
  webhookAttempts: Schema.Int,
  webhookLastError: Schema.NullOr(Schema.String),
  external: Schema.NullOr(AgentExternalResult),
  handoffSummary: Schema.NullOr(Schema.String),
  content: Schema.NullOr(Schema.String),
});

export type AgentLedgerEvent = typeof AgentLedgerEvent.Type;

export type AgentLedgerEventPin = Assert<
  Pinned<typeof AgentLedgerEvent, GeneratedAgentLedgerEvent>
>;

/**
 * `GET /api/v1/agents/:agentId/events?outcome=&before=`: newest first, 50 a page, for
 * administrators and the agent's owner (403 for anyone else). No live updates: refetch the
 * first page when the ledger is shown again.
 */
export const AgentLedgerPage = Schema.Struct({
  events: Schema.Array(AgentLedgerEvent),
  users: Schema.Array(User),
  nextCursor: Schema.NullOr(Schema.String),
});

export type AgentLedgerPage = typeof AgentLedgerPage.Type;

export type AgentLedgerPagePin = Assert<Pinned<typeof AgentLedgerPage, GeneratedAgentLedgerPage>>;
