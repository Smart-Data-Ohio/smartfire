import { Schema } from "effect";
import type { AdminOAuthClient as GeneratedAdminOAuthClient } from "../../gen/AdminOAuthClient.ts";
import type { AdminOAuthClientList as GeneratedAdminOAuthClientList } from "../../gen/AdminOAuthClientList.ts";
import type { ApproveOAuthAuthorization as GeneratedApproveOAuthAuthorization } from "../../gen/ApproveOAuthAuthorization.ts";
import type { OAuthAgentHiddenRooms as GeneratedOAuthAgentHiddenRooms } from "../../gen/OAuthAgentHiddenRooms.ts";
import type { OAuthAgentRoomAccess as GeneratedOAuthAgentRoomAccess } from "../../gen/OAuthAgentRoomAccess.ts";
import type { OAuthClientBadge as GeneratedOAuthClientBadge } from "../../gen/OAuthClientBadge.ts";
import type { OAuthConsent as GeneratedOAuthConsent } from "../../gen/OAuthConsent.ts";
import type { OAuthRedirect as GeneratedOAuthRedirect } from "../../gen/OAuthRedirect.ts";
import { BotAvatarSpec } from "./bot-avatar.ts";
import { AgentId, RoomId } from "./ids.ts";
import { PersonalAgent, PersonalAgentRoom } from "./me-agents.ts";
import type { Assert, Pinned } from "./pin.ts";
import { Timestamp } from "./time.ts";

/** Who the server says is asking: an app it hasn't verified, or one on this computer. */
export const OAuthClientBadge = Schema.Literals(["unverifiedApp", "appOnThisComputer"]);

export type OAuthClientBadgePin = Assert<
  Pinned<typeof OAuthClientBadge, GeneratedOAuthClientBadge>
>;

/** What an existing agent can do in one room now (no abilities: it's only a member there). */
export const OAuthAgentRoomAccess = Schema.Struct({
  agentId: AgentId,
  roomId: RoomId,
  roomName: Schema.String,
  capabilities: Schema.Array(Schema.String),
});

export type OAuthAgentRoomAccessPin = Assert<
  Pinned<typeof OAuthAgentRoomAccess, GeneratedOAuthAgentRoomAccess>
>;

/** How many rooms an agent is in that the person can no longer see: counted, never named. */
export const OAuthAgentHiddenRooms = Schema.Struct({ agentId: AgentId, count: Schema.Int });

export type OAuthAgentHiddenRoomsPin = Assert<
  Pinned<typeof OAuthAgentHiddenRooms, GeneratedOAuthAgentHiddenRooms>
>;

/**
 * `GET /oauth/authorizations/:publicId`: the client asking, where it sends you back, your agents
 * (with the one last linked to it), and what a personal agent may be given.
 */
export const OAuthConsent = Schema.Struct({
  clientName: Schema.String,
  redirectHostname: Schema.String,
  badge: OAuthClientBadge,
  agents: Schema.Array(PersonalAgent),
  selectedAgentId: Schema.NullOr(AgentId),
  rooms: Schema.Array(PersonalAgentRoom),
  capabilities: Schema.Array(Schema.String),
  enabled: Schema.Boolean,
  limit: Schema.NullOr(Schema.Number),
  accessAppliesToAllConnections: Schema.Boolean,
  roomAccess: Schema.Array(OAuthAgentRoomAccess),
  hiddenRooms: Schema.Array(OAuthAgentHiddenRooms),
});

export type OAuthConsentPin = Assert<Pinned<typeof OAuthConsent, GeneratedOAuthConsent>>;

/** `POST …/approve`: `agentId` null makes a new agent named `name`. */
export const ApproveOAuthAuthorization = Schema.Struct({
  agentId: Schema.NullOr(AgentId),
  name: Schema.NullOr(Schema.String),
  iconName: Schema.NullOr(Schema.String),
  /** A new agent's picture and picked avatar; ignored for an existing agent. */
  avatar: Schema.NullOr(Schema.String),
  botAvatar: Schema.NullOr(BotAvatarSpec),
  roomIds: Schema.Array(RoomId),
  capabilities: Schema.Array(Schema.String),
});

export type ApproveOAuthAuthorizationPin = Assert<
  Pinned<typeof ApproveOAuthAuthorization, GeneratedApproveOAuthAuthorization>
>;

/** The answer to approve and deny: where the browser goes next (the client's callback). */
export const OAuthRedirect = Schema.Struct({ redirectTo: Schema.String });

export type OAuthRedirectPin = Assert<Pinned<typeof OAuthRedirect, GeneratedOAuthRedirect>>;

/** A client that has connected, as administrators see it. */
export const AdminOAuthClient = Schema.Struct({
  id: Schema.Int,
  clientName: Schema.String,
  clientId: Schema.String,
  clientHost: Schema.String,
  liveConnections: Schema.Number,
  lastConnectedAt: Schema.NullOr(Timestamp),
  blockedAt: Schema.NullOr(Timestamp),
});

export type AdminOAuthClientPin = Assert<
  Pinned<typeof AdminOAuthClient, GeneratedAdminOAuthClient>
>;

/** `GET /admin/oauth_clients`. */
export const AdminOAuthClientList = Schema.Struct({
  clients: Schema.Array(AdminOAuthClient),
  oauthAvailable: Schema.Boolean,
});

export type AdminOAuthClientListPin = Assert<
  Pinned<typeof AdminOAuthClientList, GeneratedAdminOAuthClientList>
>;
