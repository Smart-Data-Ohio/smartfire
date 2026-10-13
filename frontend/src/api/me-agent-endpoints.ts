/**
 * The member's own agents (`/api/v1/me/agents/*`): Settings → Agents. Creating an agent, issuing a
 * token and rotating one wait for a fresh confirmation once the last has lapsed; their replies
 * carry a secret shown once, so they are `secret` and never kept across a Google confirmation.
 */
import { Effect } from "effect";
import type { BotRemoved } from "../gen/BotRemoved.ts";
import type { CreatePersonalAgent } from "../gen/CreatePersonalAgent.ts";
import type { IssuePersonalCredential } from "../gen/IssuePersonalCredential.ts";
import type { PersonalAgent } from "../gen/PersonalAgent.ts";
import type { PersonalAgentAccess } from "../gen/PersonalAgentAccess.ts";
import type { PersonalAgentCreated } from "../gen/PersonalAgentCreated.ts";
import type { PersonalAgentList } from "../gen/PersonalAgentList.ts";
import type { PersonalCredentialCreated } from "../gen/PersonalCredentialCreated.ts";
import type { PersonalCredentialList } from "../gen/PersonalCredentialList.ts";
import type { RotatePersonalCredential } from "../gen/RotatePersonalCredential.ts";
import type { UpdatePersonalAgent } from "../gen/UpdatePersonalAgent.ts";
import { call, get } from "./call.ts";
import { BotRemoved as BotRemovedSchema } from "./schema/bots.ts";
import {
  PersonalAgentCreated as PersonalAgentCreatedSchema,
  PersonalAgentList as PersonalAgentListSchema,
  PersonalAgent as PersonalAgentSchema,
  PersonalCredentialCreated as PersonalCredentialCreatedSchema,
  PersonalCredentialList as PersonalCredentialListSchema,
} from "./schema/me-agents.ts";
import { wire } from "./wire.ts";

const ROOT = "/me/agents";

const agentReply = wire<PersonalAgent>(PersonalAgentSchema);

const issuedReply = wire<PersonalCredentialCreated>(PersonalCredentialCreatedSchema);

/** `GET /me/agents`: your agents, and the workspace's policy for them. */
export const meAgents = Effect.fn("api.meAgents")(function* () {
  return yield* call(get(ROOT), wire<PersonalAgentList>(PersonalAgentListSchema));
});

/** `POST /me/agents`: the agent, its rooms and abilities and its first token, in one write. */
export const createMeAgent = Effect.fn("api.createMeAgent")(function* (body: CreatePersonalAgent) {
  return yield* call(
    { method: "POST", path: ROOT, body, secret: true },
    wire<PersonalAgentCreated>(PersonalAgentCreatedSchema),
  );
});

/** `PATCH /me/agents/:id`. */
export const updateMeAgent = Effect.fn("api.updateMeAgent")(function* (
  agentId: number,
  body: UpdatePersonalAgent,
) {
  return yield* call({ method: "PATCH", path: `${ROOT}/${agentId}`, body }, agentReply);
});

/** `DELETE /me/agents/:id`: deactivates the agent's bot; its tokens stop working. */
export const removeMeAgent = Effect.fn("api.removeMeAgent")(function* (agentId: number) {
  return yield* call(
    { method: "DELETE", path: `${ROOT}/${agentId}` },
    wire<BotRemoved>(BotRemovedSchema),
  );
});

/** `PUT /me/agents/:id/access`: replaces its rooms and abilities. */
export const updateMeAgentAccess = Effect.fn("api.updateMeAgentAccess")(function* (
  agentId: number,
  body: PersonalAgentAccess,
) {
  return yield* call({ method: "PUT", path: `${ROOT}/${agentId}/access`, body }, agentReply);
});

/** `POST /me/agents/:id/credentials`: a new token, its secret shown once. */
export const issueMeAgentToken = Effect.fn("api.issueMeAgentToken")(function* (
  agentId: number,
  body: IssuePersonalCredential,
) {
  return yield* call(
    { method: "POST", path: `${ROOT}/${agentId}/credentials`, body, secret: true },
    issuedReply,
  );
});

/** `POST /me/agents/:id/credentials/:cid/rotate`: a replacement; the old token stops at once. */
export const rotateMeAgentToken = Effect.fn("api.rotateMeAgentToken")(function* (
  agentId: number,
  credentialId: number,
  body: RotatePersonalCredential,
) {
  return yield* call(
    {
      method: "POST",
      path: `${ROOT}/${agentId}/credentials/${credentialId}/rotate`,
      body,
      secret: true,
    },
    issuedReply,
  );
});

/** `DELETE /me/agents/:id/credentials/:cid`: answers the agent's tokens. */
export const revokeMeAgentToken = Effect.fn("api.revokeMeAgentToken")(function* (
  agentId: number,
  credentialId: number,
) {
  return yield* call(
    { method: "DELETE", path: `${ROOT}/${agentId}/credentials/${credentialId}` },
    wire<PersonalCredentialList>(PersonalCredentialListSchema),
  );
});
