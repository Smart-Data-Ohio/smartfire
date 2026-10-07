import { Schema } from "effect";
import type { Bot as GeneratedBot } from "../../gen/Bot.ts";
import type { BotAgent as GeneratedBotAgent } from "../../gen/BotAgent.ts";
import type { BotChange as GeneratedBotChange } from "../../gen/BotChange.ts";
import type { BotGithub as GeneratedBotGithub } from "../../gen/BotGithub.ts";
import type { BotIcon as GeneratedBotIcon } from "../../gen/BotIcon.ts";
import type { BotKey as GeneratedBotKey } from "../../gen/BotKey.ts";
import type { BotList as GeneratedBotList } from "../../gen/BotList.ts";
import type { BotRemoved as GeneratedBotRemoved } from "../../gen/BotRemoved.ts";
import type { BotRoom as GeneratedBotRoom } from "../../gen/BotRoom.ts";
import type { BotSummary as GeneratedBotSummary } from "../../gen/BotSummary.ts";
import type { ConnectGithub as GeneratedConnectGithub } from "../../gen/ConnectGithub.ts";
import type { CreateBot as GeneratedCreateBot } from "../../gen/CreateBot.ts";
import type { CreateCredential as GeneratedCreateCredential } from "../../gen/CreateCredential.ts";
import type { CreateGrant as GeneratedCreateGrant } from "../../gen/CreateGrant.ts";
import type { Credential as GeneratedCredential } from "../../gen/Credential.ts";
import type { CredentialCreated as GeneratedCredentialCreated } from "../../gen/CredentialCreated.ts";
import type { CredentialList as GeneratedCredentialList } from "../../gen/CredentialList.ts";
import type { CredentialState as GeneratedCredentialState } from "../../gen/CredentialState.ts";
import type { Grant as GeneratedGrant } from "../../gen/Grant.ts";
import type { GrantList as GeneratedGrantList } from "../../gen/GrantList.ts";
import type { GrantRoom as GeneratedGrantRoom } from "../../gen/GrantRoom.ts";
import type { UpdateBot as GeneratedUpdateBot } from "../../gen/UpdateBot.ts";
import type { UpdateBotAgent as GeneratedUpdateBotAgent } from "../../gen/UpdateBotAgent.ts";
import { RoomId, UserId } from "./ids.ts";
import type { Assert, Pinned } from "./pin.ts";
import { Timestamp } from "./time.ts";

/** A bot's icon from the icon set, shown when no picture is uploaded. */
export const BotIcon = Schema.Union([
  Schema.Struct({
    kind: Schema.Literal("emoji"),
    title: Schema.String,
    character: Schema.String,
  }),
  Schema.Struct({ kind: Schema.Literal("image"), title: Schema.String, url: Schema.String }),
]);

export type BotIcon = typeof BotIcon.Type;

export type BotIconPin = Assert<Pinned<typeof BotIcon, GeneratedBotIcon>>;

/** A room a bot is in, with the commands that post there as the bot. */
export const BotRoom = Schema.Struct({
  id: RoomId,
  name: Schema.String,
  messageCommand: Schema.String,
  attachmentCommand: Schema.String,
});

export type BotRoomPin = Assert<Pinned<typeof BotRoom, GeneratedBotRoom>>;

/** One bot on the list. */
export const BotSummary = Schema.Struct({
  id: UserId,
  name: Schema.String,
  avatarUrl: Schema.String,
  icon: Schema.NullOr(BotIcon),
  ownership: Schema.String,
  rooms: Schema.Array(BotRoom),
});

export type BotSummaryPin = Assert<Pinned<typeof BotSummary, GeneratedBotSummary>>;

/** `GET /admin/bots`: the active bots, by name. */
export const BotList = Schema.Struct({ bots: Schema.Array(BotSummary) });

export type BotListPin = Assert<Pinned<typeof BotList, GeneratedBotList>>;

/** The agent behind a bot. */
export const BotAgent = Schema.Struct({
  id: Schema.Int,
  provider: Schema.NullOr(Schema.String),
  runtime: Schema.NullOr(Schema.String),
  description: Schema.NullOr(Schema.String),
  dailyMessageCap: Schema.NullOr(Schema.Int),
  dailyBoardPostCap: Schema.NullOr(Schema.Int),
  dailyExternalActionCap: Schema.NullOr(Schema.Int),
  usage: Schema.String,
  suspended: Schema.Boolean,
  ledgerUrl: Schema.String,
  approvalsUrl: Schema.String,
});

export type BotAgentPin = Assert<Pinned<typeof BotAgent, GeneratedBotAgent>>;

/** The GitHub account an agent's approved actions post as. */
export const BotGithub = Schema.Struct({
  login: Schema.String,
  usable: Schema.Boolean,
  disconnectedReason: Schema.NullOr(Schema.String),
});

export type BotGithubPin = Assert<Pinned<typeof BotGithub, GeneratedBotGithub>>;

/** `GET /admin/bots/:id`: one bot, as its classic edit page shows it. */
export const Bot = Schema.Struct({
  id: UserId,
  name: Schema.String,
  avatarUrl: Schema.String,
  avatarAttached: Schema.Boolean,
  iconName: Schema.NullOr(Schema.String),
  icon: Schema.NullOr(BotIcon),
  webhookUrl: Schema.NullOr(Schema.String),
  canAdminister: Schema.Boolean,
  agent: Schema.NullOr(BotAgent),
  signingSecret: Schema.NullOr(Schema.String),
  github: Schema.NullOr(BotGithub),
});

export type BotPin = Assert<Pinned<typeof Bot, GeneratedBot>>;

/** `POST /admin/bots`. */
export const CreateBot = Schema.Struct({
  name: Schema.String,
  iconName: Schema.NullOr(Schema.String),
  webhookUrl: Schema.NullOr(Schema.String),
  avatar: Schema.NullOr(Schema.String),
});

export type CreateBotPin = Assert<Pinned<typeof CreateBot, GeneratedCreateBot>>;

/** A bot's key, shown once. */
export const BotKey = Schema.Struct({
  id: UserId,
  name: Schema.String,
  key: Schema.String,
  exampleCommand: Schema.String,
});

export type BotKeyPin = Assert<Pinned<typeof BotKey, GeneratedBotKey>>;

/** The agent fields of an edit, as typed (`null` leaves one alone). */
export const UpdateBotAgent = Schema.Struct({
  provider: Schema.NullOr(Schema.String),
  runtime: Schema.NullOr(Schema.String),
  description: Schema.NullOr(Schema.String),
  dailyMessageCap: Schema.NullOr(Schema.String),
  dailyBoardPostCap: Schema.NullOr(Schema.String),
  dailyExternalActionCap: Schema.NullOr(Schema.String),
});

export type UpdateBotAgentPin = Assert<Pinned<typeof UpdateBotAgent, GeneratedUpdateBotAgent>>;

/** `PATCH /admin/bots/:id`: `null` leaves a key alone, blank clears it. */
export const UpdateBot = Schema.Struct({
  name: Schema.NullOr(Schema.String),
  iconName: Schema.NullOr(Schema.String),
  webhookUrl: Schema.NullOr(Schema.String),
  avatar: Schema.NullOr(Schema.String),
  agent: Schema.NullOr(UpdateBotAgent),
});

export type UpdateBotPin = Assert<Pinned<typeof UpdateBot, GeneratedUpdateBot>>;

/** A bot write's answer: the bot afterwards and the classic notice. */
export const BotChange = Schema.Struct({ bot: Bot, notice: Schema.NullOr(Schema.String) });

export type BotChangePin = Assert<Pinned<typeof BotChange, GeneratedBotChange>>;

/** `DELETE /admin/bots/:id`. */
export const BotRemoved = Schema.Struct({ id: UserId });

export type BotRemovedPin = Assert<Pinned<typeof BotRemoved, GeneratedBotRemoved>>;

/** `PUT /admin/bots/:id/github_connection`. */
export const ConnectGithub = Schema.Struct({ accessToken: Schema.String });

export type ConnectGithubPin = Assert<Pinned<typeof ConnectGithub, GeneratedConnectGithub>>;

/** Where a credential stands. */
export const CredentialState = Schema.Literals(["active", "expired", "revoked"]);

export type CredentialStatePin = Assert<Pinned<typeof CredentialState, GeneratedCredentialState>>;

/** A bearer token for the agent API; its secret shows only when issued. */
export const Credential = Schema.Struct({
  id: Schema.Int,
  name: Schema.String,
  lastFour: Schema.String,
  createdBy: Schema.String,
  createdAt: Timestamp,
  lastUsedAt: Schema.NullOr(Timestamp),
  expiresAt: Schema.NullOr(Schema.String),
  state: CredentialState,
});

export type CredentialPin = Assert<Pinned<typeof Credential, GeneratedCredential>>;

/** `GET /admin/bots/:id/credentials`, newest first. */
export const CredentialList = Schema.Struct({
  botId: UserId,
  botName: Schema.String,
  canIssue: Schema.Boolean,
  credentials: Schema.Array(Credential),
});

export type CredentialListPin = Assert<Pinned<typeof CredentialList, GeneratedCredentialList>>;

/** `POST /admin/bots/:id/credentials`: `expiresAt` is a local date and time. */
export const CreateCredential = Schema.Struct({
  name: Schema.String,
  expiresAt: Schema.NullOr(Schema.String),
});

export type CreateCredentialPin = Assert<
  Pinned<typeof CreateCredential, GeneratedCreateCredential>
>;

/** A new credential: its secret, shown once, and the list with it. */
export const CredentialCreated = Schema.Struct({
  secret: Schema.String,
  credentials: CredentialList,
});

export type CredentialCreatedPin = Assert<
  Pinned<typeof CredentialCreated, GeneratedCredentialCreated>
>;

/** A capability the agent holds, workspace-wide or in one room. */
export const Grant = Schema.Struct({
  id: Schema.Int,
  capability: Schema.String,
  roomName: Schema.String,
  grantedBy: Schema.String,
  createdAt: Timestamp,
  revoked: Schema.Boolean,
});

export type GrantPin = Assert<Pinned<typeof Grant, GeneratedGrant>>;

/** A room a grant can be scoped to. */
export const GrantRoom = Schema.Struct({ id: RoomId, name: Schema.String });

export type GrantRoomPin = Assert<Pinned<typeof GrantRoom, GeneratedGrantRoom>>;

/** `GET /admin/bots/:id/grants`, active first, with what the form offers. */
export const GrantList = Schema.Struct({
  botId: UserId,
  botName: Schema.String,
  canGrant: Schema.Boolean,
  legacy: Schema.Boolean,
  grants: Schema.Array(Grant),
  capabilities: Schema.Array(Schema.String),
  rooms: Schema.Array(GrantRoom),
});

export type GrantListPin = Assert<Pinned<typeof GrantList, GeneratedGrantList>>;

/** `POST /admin/bots/:id/grants`: `roomId` `null` is workspace-wide. */
export const CreateGrant = Schema.Struct({
  capability: Schema.String,
  roomId: Schema.NullOr(RoomId),
});

export type CreateGrantPin = Assert<Pinned<typeof CreateGrant, GeneratedCreateGrant>>;
