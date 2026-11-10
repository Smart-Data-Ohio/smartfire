import type { Bot } from "../../src/gen/Bot.ts";
import type { Grant } from "../../src/gen/Grant.ts";
import type { PersonalAgentCredential } from "../../src/gen/PersonalAgentCredential.ts";
import type { WakePolicy } from "../../src/gen/WakePolicy.ts";

export interface AgentRecord {
  id: number;
  provider: string | null;
  runtime: string | null;
  description: string | null;
  caps: [number | null, number | null, number | null];
  suspended: boolean;
  /** Who can wake it; new personal agents start owner-only. */
  wakePolicy: WakePolicy;
  /** When it was paused and by whom; both `null` while it runs. */
  pausedAt: string | null;
  pausedBy: string | null;
  credentials: (PersonalAgentCredential & { createdBy: string })[];
  grants: (Omit<Grant, "roomName" | "status"> & { roomId: number | null })[];
  everGranted: boolean;
}

type BotFields = Pick<
  Bot,
  | "id"
  | "name"
  | "avatarUrl"
  | "avatarAttached"
  | "iconName"
  | "botAvatar"
  | "signingSecret"
  | "github"
> & {
  removed: boolean;
};

export const PERSONAL_CAPABILITIES = ["read_messages", "post_messages", "react", "manage_threads"];

export interface PersonalBotRecord extends BotFields {
  kind: "personal";
  webhookUrl: null;
  key: null;
  agent: AgentRecord;
  roomIds: number[];
  createdAt: string;
  lastUsedAt: string | null;
}

export type BotRecord =
  | PersonalBotRecord
  | (BotFields & {
      kind: "workspace";
      webhookUrl: string | null;
      key: string;
      agent: AgentRecord | null;
    });

export function agentRecord(id: number): AgentRecord {
  return {
    id,
    provider: null,
    runtime: null,
    description: null,
    caps: [null, null, null],
    suspended: false,
    wakePolicy: "owner",
    pausedAt: null,
    pausedBy: null,
    credentials: [],
    grants: [],
    everGranted: false,
  };
}

/**
 * Pauses or resumes `agent` as the server's `set_paused` does: a suspended agent and a repeat are
 * left alone, and resuming clears who paused it.
 */
export function setPaused(agent: AgentRecord, paused: boolean, by: string, at: string): void {
  if (agent.suspended || (agent.pausedAt !== null) === paused) return;

  agent.pausedAt = paused ? at : null;
  agent.pausedBy = paused ? by : null;
}
