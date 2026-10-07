import { Schema } from "effect";
import type { AgentBadge as GeneratedAgentBadge } from "../../gen/AgentBadge.ts";
import type { AgentKind as GeneratedAgentKind } from "../../gen/AgentKind.ts";
import type { AgentStatus as GeneratedAgentStatus } from "../../gen/AgentStatus.ts";
import { AgentId } from "./ids.ts";
import type { Assert, Pinned } from "./pin.ts";

/** `agents.status`, set by the agent. */
export const AgentStatus = Schema.Literals(["idle", "working", "waiting", "failed"]);

export type AgentStatus = typeof AgentStatus.Type;

export type AgentStatusPin = Assert<Pinned<typeof AgentStatus, GeneratedAgentStatus>>;

/** Owned by one person, or managed for the workspace. */
export const AgentKind = Schema.Literals(["personal", "workspace"]);

export type AgentKind = typeof AgentKind.Type;

export type AgentKindPin = Assert<Pinned<typeof AgentKind, GeneratedAgentKind>>;

/**
 * The agent facts every human sees beside an agent's name (`User.agent`). `suspended` reads
 * "Suspended" whatever `status` says. Kept current by `agent.status`.
 */
export const AgentBadge = Schema.Struct({
  agentId: AgentId,
  kind: AgentKind,
  status: AgentStatus,
  suspended: Schema.Boolean,
});

export type AgentBadge = typeof AgentBadge.Type;

export type AgentBadgePin = Assert<Pinned<typeof AgentBadge, GeneratedAgentBadge>>;
