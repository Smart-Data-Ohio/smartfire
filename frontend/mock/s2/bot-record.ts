import type { Bot } from "../../src/gen/Bot.ts";
import type { Grant } from "../../src/gen/Grant.ts";
import type { PersonalAgentCredential } from "../../src/gen/PersonalAgentCredential.ts";

export interface AgentRecord {
  id: number;
  provider: string | null;
  runtime: string | null;
  description: string | null;
  caps: [number | null, number | null, number | null];
  suspended: boolean;
  credentials: (PersonalAgentCredential & { createdBy: string })[];
  grants: (Omit<Grant, "roomName" | "status"> & { roomId: number | null })[];
  everGranted: boolean;
}

type BotFields = Pick<
  Bot,
  "id" | "name" | "avatarUrl" | "avatarAttached" | "iconName" | "signingSecret" | "github"
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
    credentials: [],
    grants: [],
    everGranted: false,
  };
}
