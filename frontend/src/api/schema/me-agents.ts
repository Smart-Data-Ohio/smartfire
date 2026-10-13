import { Schema } from "effect";
import type { CreatePersonalAgent as GeneratedCreatePersonalAgent } from "../../gen/CreatePersonalAgent.ts";
import type { IssuePersonalCredential as GeneratedIssuePersonalCredential } from "../../gen/IssuePersonalCredential.ts";
import type { PersonalAgent as GeneratedPersonalAgent } from "../../gen/PersonalAgent.ts";
import type { PersonalAgentAccess as GeneratedPersonalAgentAccess } from "../../gen/PersonalAgentAccess.ts";
import type { PersonalAgentCreated as GeneratedPersonalAgentCreated } from "../../gen/PersonalAgentCreated.ts";
import type { PersonalAgentCredential as GeneratedPersonalAgentCredential } from "../../gen/PersonalAgentCredential.ts";
import type { PersonalAgentList as GeneratedPersonalAgentList } from "../../gen/PersonalAgentList.ts";
import type { PersonalCredentialCreated as GeneratedPersonalCredentialCreated } from "../../gen/PersonalCredentialCreated.ts";
import type { PersonalCredentialList as GeneratedPersonalCredentialList } from "../../gen/PersonalCredentialList.ts";
import type { RotatePersonalCredential as GeneratedRotatePersonalCredential } from "../../gen/RotatePersonalCredential.ts";
import type { UpdatePersonalAgent as GeneratedUpdatePersonalAgent } from "../../gen/UpdatePersonalAgent.ts";
import { CredentialState, GrantRoom } from "./bots.ts";
import { AgentId, CredentialId, RoomId, UserId } from "./ids.ts";
import type { Assert, Pinned } from "./pin.ts";
import { Timestamp } from "./time.ts";

/** A token one of your agents connects with; its secret shows only when issued. */
export const PersonalAgentCredential = Schema.Struct({
  id: CredentialId,
  kind: Schema.Literal("token"),
  name: Schema.String,
  lastFour: Schema.String,
  createdAt: Timestamp,
  expiresAt: Schema.NullOr(Timestamp),
  lastUsedAt: Schema.NullOr(Timestamp),
  lastUsedIp: Schema.NullOr(Schema.String),
  state: CredentialState,
});

export type PersonalAgentCredentialPin = Assert<
  Pinned<typeof PersonalAgentCredential, GeneratedPersonalAgentCredential>
>;

/** One of your agents: its profile, the rooms and abilities it holds, and its tokens. */
export const PersonalAgent = Schema.Struct({
  id: AgentId,
  userId: UserId,
  name: Schema.String,
  iconName: Schema.NullOr(Schema.String),
  provider: Schema.NullOr(Schema.String),
  runtime: Schema.NullOr(Schema.String),
  description: Schema.NullOr(Schema.String),
  dailyMessageCap: Schema.NullOr(Schema.Number),
  suspended: Schema.Boolean,
  roomIds: Schema.Array(RoomId),
  capabilities: Schema.Array(Schema.String),
  lastUsedAt: Schema.NullOr(Timestamp),
  credentials: Schema.Array(PersonalAgentCredential),
});

export type PersonalAgentPin = Assert<Pinned<typeof PersonalAgent, GeneratedPersonalAgent>>;

/** `GET /me/agents`: your agents, and what the workspace lets you give one. */
export const PersonalAgentList = Schema.Struct({
  agents: Schema.Array(PersonalAgent),
  enabled: Schema.Boolean,
  limit: Schema.NullOr(Schema.Number),
  capabilities: Schema.Array(Schema.String),
  rooms: Schema.Array(GrantRoom),
  mcpUrl: Schema.String,
  oauthAvailable: Schema.Boolean,
});

export type PersonalAgentListPin = Assert<
  Pinned<typeof PersonalAgentList, GeneratedPersonalAgentList>
>;

/** `POST /me/agents`: the new agent and its first token's secret, shown once. */
export const PersonalAgentCreated = Schema.Struct({
  agent: PersonalAgent,
  secret: Schema.String,
});

export type PersonalAgentCreatedPin = Assert<
  Pinned<typeof PersonalAgentCreated, GeneratedPersonalAgentCreated>
>;

/** `GET /me/agents/:id/credentials`, and the answer to a revoke. */
export const PersonalCredentialList = Schema.Struct({
  credentials: Schema.Array(PersonalAgentCredential),
});

export type PersonalCredentialListPin = Assert<
  Pinned<typeof PersonalCredentialList, GeneratedPersonalCredentialList>
>;

/** A token issued or rotated: the new row and its secret, shown once. */
export const PersonalCredentialCreated = Schema.Struct({
  credential: PersonalAgentCredential,
  secret: Schema.String,
});

export type PersonalCredentialCreatedPin = Assert<
  Pinned<typeof PersonalCredentialCreated, GeneratedPersonalCredentialCreated>
>;

/** `POST /me/agents`: `expiresAt` null is 90 days, `"never"` no expiry. */
export const CreatePersonalAgent = Schema.Struct({
  name: Schema.String,
  iconName: Schema.NullOr(Schema.String),
  provider: Schema.NullOr(Schema.String),
  runtime: Schema.NullOr(Schema.String),
  roomIds: Schema.Array(RoomId),
  capabilities: Schema.Array(Schema.String),
  tokenName: Schema.NullOr(Schema.String),
  expiresAt: Schema.NullOr(Schema.String),
});

export type CreatePersonalAgentPin = Assert<
  Pinned<typeof CreatePersonalAgent, GeneratedCreatePersonalAgent>
>;

/** `PATCH /me/agents/:id`: `null` leaves a field alone. */
export const UpdatePersonalAgent = Schema.Struct({
  name: Schema.NullOr(Schema.String),
  iconName: Schema.NullOr(Schema.String),
  provider: Schema.NullOr(Schema.String),
  runtime: Schema.NullOr(Schema.String),
  description: Schema.NullOr(Schema.String),
  dailyMessageCap: Schema.optionalKey(Schema.NullOr(Schema.Number)),
});

export type UpdatePersonalAgentPin = Assert<
  Pinned<typeof UpdatePersonalAgent, GeneratedUpdatePersonalAgent>
>;

/** `PUT /me/agents/:id/access`: replaces the agent's rooms and abilities. */
export const PersonalAgentAccess = Schema.Struct({
  roomIds: Schema.Array(RoomId),
  capabilities: Schema.Array(Schema.String),
});

export type PersonalAgentAccessPin = Assert<
  Pinned<typeof PersonalAgentAccess, GeneratedPersonalAgentAccess>
>;

/** `POST /me/agents/:id/credentials`. */
export const IssuePersonalCredential = Schema.Struct({
  name: Schema.String,
  expiresAt: Schema.NullOr(Schema.String),
});

export type IssuePersonalCredentialPin = Assert<
  Pinned<typeof IssuePersonalCredential, GeneratedIssuePersonalCredential>
>;

/** `POST /me/agents/:id/credentials/:cid/rotate`: `null` keeps the name, or takes 90 days. */
export const RotatePersonalCredential = Schema.Struct({
  name: Schema.NullOr(Schema.String),
  expiresAt: Schema.NullOr(Schema.String),
});

export type RotatePersonalCredentialPin = Assert<
  Pinned<typeof RotatePersonalCredential, GeneratedRotatePersonalCredential>
>;
