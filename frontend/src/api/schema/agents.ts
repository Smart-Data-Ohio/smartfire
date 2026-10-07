import { Schema } from "effect";
import type { AgentActivitySummary as GeneratedAgentActivitySummary } from "../../gen/AgentActivitySummary.ts";
import type { AgentApproval as GeneratedAgentApproval } from "../../gen/AgentApproval.ts";
import type { AgentApprovalPage as GeneratedAgentApprovalPage } from "../../gen/AgentApprovalPage.ts";
import type { AgentBudgetUsage as GeneratedAgentBudgetUsage } from "../../gen/AgentBudgetUsage.ts";
import type { AgentCapability as GeneratedAgentCapability } from "../../gen/AgentCapability.ts";
import type { AgentDirectory as GeneratedAgentDirectory } from "../../gen/AgentDirectory.ts";
import type { AgentDirectoryRow as GeneratedAgentDirectoryRow } from "../../gen/AgentDirectoryRow.ts";
import type { AgentGrant as GeneratedAgentGrant } from "../../gen/AgentGrant.ts";
import type { AgentGrants as GeneratedAgentGrants } from "../../gen/AgentGrants.ts";
import type { AgentManagement as GeneratedAgentManagement } from "../../gen/AgentManagement.ts";
import type { AgentProfile as GeneratedAgentProfile } from "../../gen/AgentProfile.ts";
import type { AgentProfileRoom as GeneratedAgentProfileRoom } from "../../gen/AgentProfileRoom.ts";
import type { AgentStatusChanged as GeneratedAgentStatusChanged } from "../../gen/AgentStatusChanged.ts";
import type { AgentStep as GeneratedAgentStep } from "../../gen/AgentStep.ts";
import type { AgentStepStatus as GeneratedAgentStepStatus } from "../../gen/AgentStepStatus.ts";
import type { AgentStepsChanged as GeneratedAgentStepsChanged } from "../../gen/AgentStepsChanged.ts";
import type { ApprovalDecision as GeneratedApprovalDecision } from "../../gen/ApprovalDecision.ts";
import type { ApprovalUpdated as GeneratedApprovalUpdated } from "../../gen/ApprovalUpdated.ts";
import type { DecideApproval as GeneratedDecideApproval } from "../../gen/DecideApproval.ts";
import { AgentApprovalStatus, AgentBudgetCap } from "./activity.ts";
import { AgentKind, AgentStatus } from "./agent-identity.ts";
import {
  AgentApprovalId,
  AgentId,
  AgentStepId,
  MessageId,
  RoomId,
  ThreadId,
  UserId,
} from "./ids.ts";
import type { Assert, Pinned } from "./pin.ts";
import { Timestamp } from "./time.ts";
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

/** `GET /api/v1/agents/:agentId`. `management` is for administrators and the owner only. */
export const AgentProfile = Schema.Struct({
  agent: AgentDirectoryRow,
  provider: Schema.NullOr(Schema.String),
  runtime: Schema.NullOr(Schema.String),
  description: Schema.NullOr(Schema.String),
  rooms: Schema.Array(AgentProfileRoom),
  hiddenRoomCount: Schema.Int,
  grants: AgentGrants,
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
});

export type AgentStatusChanged = typeof AgentStatusChanged.Type;

export type AgentStatusChangedPin = Assert<
  Pinned<typeof AgentStatusChanged, GeneratedAgentStatusChanged>
>;

export const AgentStepStatus = Schema.Literals(["pending", "running", "done", "failed"]);

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
 * rights; the server checks them again on every decision.
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
});

export type AgentApproval = typeof AgentApproval.Type;

export type AgentApprovalPin = Assert<Pinned<typeof AgentApproval, GeneratedAgentApproval>>;

/** `GET /api/v1/agents/:agentId/approvals?status=&before=`: newest first, 50 a page. */
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

/** `PATCH /api/v1/agent_approvals/:id`. Leave `note` out for none. */
export const DecideApproval = Schema.Struct({
  decision: ApprovalDecision,
  note: Schema.optionalKey(Schema.String),
});

export type DecideApproval = typeof DecideApproval.Type;

export type DecideApprovalPin = Assert<Pinned<typeof DecideApproval, GeneratedDecideApproval>>;

/** The `approval.updated` event, on the `user` topic of everyone who got the request's item. */
export const ApprovalUpdated = Schema.Struct({ approval: AgentApproval });

export type ApprovalUpdated = typeof ApprovalUpdated.Type;

export type ApprovalUpdatedPin = Assert<Pinned<typeof ApprovalUpdated, GeneratedApprovalUpdated>>;
