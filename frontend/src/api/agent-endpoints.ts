/**
 * The S4 agent endpoints: the directory and an agent's profile. Each validates the reply with its
 * pinned schema (see endpoints.ts).
 */
import { Effect } from "effect";
import type { AgentDirectory } from "../gen/AgentDirectory.ts";
import type { AgentProfile } from "../gen/AgentProfile.ts";
import { call, get } from "./call.ts";
import {
  AgentDirectory as AgentDirectorySchema,
  AgentProfile as AgentProfileSchema,
} from "./schema/agents.ts";
import { wire } from "./wire.ts";

/** `GET /agents`: every agent, active ones first, then by name; not paged. A bot gets a 403. */
export const agentDirectory = Effect.fn("api.agentDirectory")(function* () {
  return yield* call(get("/agents"), wire<AgentDirectory>(AgentDirectorySchema));
});

/**
 * `GET /agents/:agentId`: the profile. `grants` and `management` are `null` unless the viewer is an
 * administrator or the agent's owner; an unknown agent is a 404.
 */
export const agentProfile = Effect.fn("api.agentProfile")(function* (agentId: number) {
  return yield* call(get(`/agents/${agentId}`), wire<AgentProfile>(AgentProfileSchema));
});
