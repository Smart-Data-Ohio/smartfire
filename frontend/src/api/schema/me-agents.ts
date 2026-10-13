import { Schema } from "effect";
import type { ConnectionRevokedReason as GeneratedConnectionRevokedReason } from "../../gen/ConnectionRevokedReason.ts";
import type { CreatePersonalAgent as GeneratedCreatePersonalAgent } from "../../gen/CreatePersonalAgent.ts";
import type { IssuePersonalCredential as GeneratedIssuePersonalCredential } from "../../gen/IssuePersonalCredential.ts";
import type { PersonalAgent as GeneratedPersonalAgent } from "../../gen/PersonalAgent.ts";
import type { PersonalAgentAccess as GeneratedPersonalAgentAccess } from "../../gen/PersonalAgentAccess.ts";
import type { PersonalAgentConnection as GeneratedPersonalAgentConnection } from "../../gen/PersonalAgentConnection.ts";
import type { PersonalAgentCreated as GeneratedPersonalAgentCreated } from "../../gen/PersonalAgentCreated.ts";
import type { PersonalAgentCredential as GeneratedPersonalAgentCredential } from "../../gen/PersonalAgentCredential.ts";
import type { PersonalAgentList as GeneratedPersonalAgentList } from "../../gen/PersonalAgentList.ts";
import type { PersonalAgentRoom as GeneratedPersonalAgentRoom } from "../../gen/PersonalAgentRoom.ts";
import type { PersonalCredentialCreated as GeneratedPersonalCredentialCreated } from "../../gen/PersonalCredentialCreated.ts";
import type { PersonalCredentialList as GeneratedPersonalCredentialList } from "../../gen/PersonalCredentialList.ts";
import type { RotatePersonalCredential as GeneratedRotatePersonalCredential } from "../../gen/RotatePersonalCredential.ts";
import type { UpdatePersonalAgent as GeneratedUpdatePersonalAgent } from "../../gen/UpdatePersonalAgent.ts";
import { CredentialState } from "./bots.ts";
import { AgentId, CredentialId, RoomId, UserId } from "./ids.ts";
import type { Assert, Pinned } from "./pin.ts";
import { Timestamp } from "./time.ts";

/** Why the server revoked a connection: a used sign-in token was presented again. */
export const ConnectionRevokedReason = Schema.Literals(["tokenReused"]);

export type ConnectionRevokedReasonPin = Assert<
  Pinned<typeof ConnectionRevokedReason, GeneratedConnectionRevokedReason>
>;

/** An OAuth connection's client, and the server's reason when it revoked one. */
export const PersonalAgentConnection = Schema.Struct({
  clientName: Schema.String,
  clientHost: Schema.String,
  clientBlocked: Schema.Boolean,
  revokedReason: Schema.NullOr(ConnectionRevokedReason),
});

export type PersonalAgentConnectionPin = Assert<
  Pinned<typeof PersonalAgentConnection, GeneratedPersonalAgentConnection>
>;

/**
 * A way into one of your agents: a token (its secret shows only when issued) or an OAuth
 * connection a client signed in with (`connection` names the client).
 */
export const PersonalAgentCredential = Schema.Struct({
  id: CredentialId,
  kind: Schema.Literals(["token", "oauth"]),
  name: Schema.String,
  lastFour: Schema.String,
  createdAt: Timestamp,
  expiresAt: Schema.NullOr(Timestamp),
  lastUsedAt: Schema.NullOr(Timestamp),
  lastUsedIp: Schema.NullOr(Schema.String),
  state: CredentialState,
  connection: Schema.NullOr(PersonalAgentConnection),
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
  /** The bot's icon image (`User.avatarIcon.imageUrl`); null for an emoji or unknown icon. */
  iconUrl: Schema.NullOr(Schema.String),
  provider: Schema.NullOr(Schema.String),
  runtime: Schema.NullOr(Schema.String),
  description: Schema.NullOr(Schema.String),
  dailyMessageCap: Schema.NullOr(Schema.Number),
  suspended: Schema.Boolean,
  roomIds: Schema.Array(RoomId),
  capabilities: Schema.Array(Schema.String),
  lastUsedAt: Schema.NullOr(Timestamp),
  createdAt: Timestamp,
  credentials: Schema.Array(PersonalAgentCredential),
});

export type PersonalAgentPin = Assert<Pinned<typeof PersonalAgent, GeneratedPersonalAgent>>;

/** A room you can give your agent; a private one shows the agent in its member list. */
export const PersonalAgentRoom = Schema.Struct({
  id: RoomId,
  name: Schema.String,
  private: Schema.Boolean,
});

export type PersonalAgentRoomPin = Assert<
  Pinned<typeof PersonalAgentRoom, GeneratedPersonalAgentRoom>
>;

/**
 * `GET /me/agents`: your agents, and what the workspace lets you give one. `mcpUrl` is null until
 * an administrator sets a valid APP_URL.
 */
export const PersonalAgentList = Schema.Struct({
  agents: Schema.Array(PersonalAgent),
  enabled: Schema.Boolean,
  limit: Schema.NullOr(Schema.Number),
  capabilities: Schema.Array(Schema.String),
  rooms: Schema.Array(PersonalAgentRoom),
  mcpUrl: Schema.NullOr(Schema.String),
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
