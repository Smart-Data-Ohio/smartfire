import type { OAuthAgentHiddenRooms } from "../../src/gen/OAuthAgentHiddenRooms.ts";
import type { OAuthAgentRoomAccess } from "../../src/gen/OAuthAgentRoomAccess.ts";
import type { PersonalAgent } from "../../src/gen/PersonalAgent.ts";
import type { PersonalAgentConnection } from "../../src/gen/PersonalAgentConnection.ts";
import type { PersonalAgentCredential } from "../../src/gen/PersonalAgentCredential.ts";
import type { PersonalAgentList } from "../../src/gen/PersonalAgentList.ts";
import { forbidden, notFound, ok, validation } from "../http.ts";
import { field, isNumber, type Json, stringArrayField, stringField } from "../json.ts";
import { timestamp, VIEWER_ID, type World } from "../seed.ts";
import type { AdminModule } from "./admin.ts";
import { applied, type BotAvatarChange, botAvatarChange } from "./bot-avatar.ts";
import { agentRecord, PERSONAL_CAPABILITIES, type PersonalBotRecord } from "./bot-record.ts";
import { route, type S2Context } from "./context.ts";
import { lookupIcon } from "./emoji.ts";
import type { Uploads } from "./uploads.ts";

const CAPABILITIES = PERSONAL_CAPABILITIES;

const DAY = 86_400_000;

const DISABLED = "Personal agents are turned off for this workspace.";

/** How long an OAuth connection lasts (the server's `CONNECTION_LIFETIME`). */
const CONNECTION_DAYS = 180;

/** The client a connection goes through, as the OAuth mock hands it over. */
export interface ConnectingClient {
  readonly clientName: string;
  readonly clientHost: string;
  /** Its identity (the URL of its metadata document): clients on one host can differ. */
  readonly clientId: string;
}

/** What the OAuth mock (mock/s2/oauth.ts) needs from the personal agents it connects. */
export interface AgentConnections {
  /** Whether the server has OAuth (what `oauthAvailable` says); off by default. */
  readonly available: () => boolean;
  readonly setAvailable: (on: boolean) => void;
  /** The viewer's agents, as the API presents them. */
  readonly agents: () => PersonalAgent[];
  readonly rooms: () => { id: number; name: string; private: boolean }[];
  readonly capabilities: readonly string[];
  /** The viewer's agent limit; `null` for an administrator. */
  readonly limit: () => number | null;
  readonly enabled: () => boolean;
  /**
   * Approves: connects `agentId` (or a new agent named `name`) through `client`, replacing its
   * access with the body's `roomIds` and `capabilities`, as the server's approve does.
   */
  readonly connect: (
    client: ConnectingClient,
    agentId: number | null,
    body: Json | undefined,
  ) => { agentId: number; credentialId: number };
  /** Ends a connection as the server does when a used sign-in token comes back. */
  readonly revokeForReuse: (credentialId: number) => boolean;
  /** Marks every connection through the client `clientId` blocked or not. */
  readonly setBlocked: (clientId: string, blocked: boolean) => void;
  /** The live connections through the client `clientId`. */
  readonly liveConnections: (clientId: string) => number;
  /** What each agent can do in each of its rooms, as the consent page compares it. */
  readonly roomAccess: () => OAuthAgentRoomAccess[];
  /** Per agent, how many rooms it's in that the viewer can't see any more (only counted). */
  readonly hiddenRooms: () => OAuthAgentHiddenRooms[];
}

export function createMeAgents(ctx: S2Context, admin: AdminModule, uploads: Uploads) {
  let world: World | null = null;
  const agents = new Map<number, PersonalBotRecord>();
  let nextId = 10000;
  let nextUserId = 20000;
  let nextCredentialId = 10000;

  const current = () => {
    if (world !== ctx.world()) {
      world = ctx.world();
      agents.clear();
      nextId = 10000;
      nextUserId = 20000;
      nextCredentialId = 10000;
    }

    for (const agent of agents.values()) {
      if (agent.removed) {
        agents.delete(agent.agent.id);
        continue;
      }

      for (const credential of agent.agent.credentials) {
        if (
          credential.state === "active" &&
          credential.expiresAt !== null &&
          Date.parse(credential.expiresAt) <= ctx.now()
        )
          credential.state = "expired";
      }
    }

    return agents;
  };

  const find = (id: number | undefined) => {
    const agent = id === undefined ? undefined : current().get(id);

    if (agent === undefined || agent.removed) throw notFound();

    return agent;
  };

  const rooms = () =>
    [...ctx.world().rooms.values()].flatMap((room) =>
      room.room.kind !== "direct" && room.memberIds.includes(VIEWER_ID)
        ? [{ id: room.room.id, name: ctx.displayName(room), private: room.room.kind !== "open" }]
        : [],
    );

  const access = (body: Json | undefined) => {
    const capabilities = stringArrayField(body, "capabilities");
    const rawRooms = field(body, "roomIds");

    if (capabilities === null || capabilities.some((c) => !CAPABILITIES.includes(c)))
      throw validation("capabilities", "must be everyday chat capabilities");

    if (!Array.isArray(rawRooms)) throw validation("roomIds", "must be an array");
    const roomIds: number[] = [];

    for (const id of rawRooms) {
      if (!isNumber(id) || !rooms().some((r) => r.id === id))
        throw validation("roomIds", "must be a non-direct room you belong to");
      roomIds.push(id);
    }

    return {
      roomIds: [...new Set(roomIds)].sort((a, b) => a - b),
      capabilities: [...new Set(capabilities)].sort(),
    };
  };

  const replaceAccess = (record: PersonalBotRecord, next: ReturnType<typeof access>) => {
    record.roomIds = next.roomIds;

    for (const grant of record.agent.grants) grant.revoked = true;

    for (const roomId of next.roomIds) {
      for (const capability of next.capabilities) {
        record.agent.grants.push({
          id: record.agent.grants.length + 1,
          roomId,
          capability,
          grantedBy: ctx.world().users.get(VIEWER_ID)?.name ?? "You",
          createdAt: timestamp(ctx.now()),
          revoked: false,
        });
        record.agent.everGranted = true;
      }
    }
  };

  const roomCapabilities = (record: PersonalBotRecord, roomId: number) =>
    record.agent.grants
      .filter((grant) => !grant.revoked && grant.roomId === roomId)
      .map((grant) => grant.capability);

  const present = (record: PersonalBotRecord): PersonalAgent => {
    const grantable = new Set(rooms().map((room) => room.id));
    const roomIds = record.roomIds.filter((id) => grantable.has(id));
    const agent = record.agent;

    const capabilities =
      agent.suspended || !admin.memberAgentPolicy().enabled
        ? []
        : [...new Set(roomIds.flatMap((id) => roomCapabilities(record, id)))].sort();

    const iconUrl = lookupIcon(record.iconName ?? "", admin.uploadedIcons())?.imageUrl ?? null;

    return {
      id: agent.id,
      userId: record.id,
      name: record.name,
      iconName: record.iconName,
      iconUrl,
      botAvatar: record.botAvatar,
      avatarUrl: record.avatarAttached ? record.avatarUrl : null,
      provider: agent.provider,
      runtime: agent.runtime,
      description: agent.description,
      dailyMessageCap: agent.caps[0],
      suspended: agent.suspended,
      lastUsedAt: record.lastUsedAt,
      createdAt: record.createdAt,
      credentials: agent.credentials.map(({ createdBy: _createdBy, ...credential }) => credential),
      roomIds,
      capabilities,
    };
  };

  const enabled = () => {
    if (!admin.memberAgentPolicy().enabled) throw forbidden(DISABLED);
  };

  const issue = (
    agent: PersonalBotRecord,
    body: Json | undefined,
    previous?: PersonalAgentCredential,
  ) => {
    const name = stringField(body, "name") ?? previous?.name ?? "Primary token";

    if (name.trim() === "") throw validation("name", "can't be blank");
    const expiry = stringField(body, "expiresAt");
    let expires: number | null = ctx.now() + 90 * DAY;

    if (expiry === "never") expires = null;
    else if (expiry !== null) expires = Date.parse(expiry);

    if (expires !== null && (!Number.isFinite(expires) || expires <= ctx.now()))
      throw validation("expiresAt", "must be a future timestamp or never");
    const secret = `sfa_${ctx.hex(64)}`;

    const credential: PersonalAgentCredential = {
      id: nextCredentialId++,
      kind: "token",
      name,
      lastFour: secret.slice(-4),
      createdAt: timestamp(ctx.now()),
      expiresAt: expires === null ? null : timestamp(expires),
      lastUsedAt: null,
      lastUsedIp: null,
      state: "active",
      connection: null,
    };

    if (previous !== undefined) previous.state = "revoked";
    agent.agent.credentials.unshift({
      ...credential,
      createdBy: ctx.world().users.get(VIEWER_ID)?.name ?? "You",
    });

    return { status: 201, json: { credential, secret } };
  };

  let oauthAvailable = false;
  let oauthWorld: World | null = null;

  const connections = (): PersonalAgentCredential[] =>
    [...current().values()].flatMap((agent) =>
      agent.agent.credentials.filter((credential) => credential.connection !== null),
    );

  const viewerLimit = () =>
    ctx.world().users.get(VIEWER_ID)?.role === "administrator"
      ? null
      : admin.memberAgentPolicy().limit;

  /** The write's picture and picked avatar, checked before anything is written. */
  const pictureOf = (body: Json | undefined) => {
    const avatar = stringField(body, "avatar");
    const change = botAvatarChange(body, (message) => validation("botAvatar", message));

    return {
      change,
      avatarUrl: avatar === null ? null : uploads.attachment(avatar).url,
    };
  };

  /** The picture after a write: a new upload, and the spec it's the render of (or none). */
  const repicture = (
    record: PersonalBotRecord,
    picture: { change: BotAvatarChange; avatarUrl: string | null },
  ) => {
    if (picture.avatarUrl !== null) {
      record.avatarUrl = picture.avatarUrl;
      record.avatarAttached = true;
    }

    record.botAvatar = applied(record.botAvatar, picture.change);
  };

  const create = (name: string, iconName: string | null, body?: Json): PersonalBotRecord => {
    const limit = viewerLimit();
    const existing = current();

    if (limit !== null && existing.size >= limit)
      throw validation("base", "personal agent limit reached", "Personal agent limit reached");

    if (name.trim() === "") throw validation("name", "can't be blank");

    const picture = pictureOf(body);

    const agent: PersonalBotRecord = {
      kind: "personal",
      id: nextUserId++,
      name,
      avatarUrl: "/avatar.svg",
      avatarAttached: false,
      iconName,
      botAvatar: null,
      webhookUrl: null,
      signingSecret: null,
      key: null,
      github: null,
      removed: false,
      agent: { ...agentRecord(nextId++), caps: [200, null, null] },
      roomIds: [],
      createdAt: timestamp(ctx.now()),
      lastUsedAt: null,
    };

    repicture(agent, picture);

    return agent;
  };

  const connectionsApi: AgentConnections = {
    available: () => {
      if (oauthWorld !== ctx.world()) {
        oauthWorld = ctx.world();
        oauthAvailable = false;
      }

      return oauthAvailable;
    },
    setAvailable: (on) => {
      oauthWorld = ctx.world();
      oauthAvailable = on;
    },
    agents: () => [...current().values()].map(present),
    rooms: () => rooms(),
    capabilities: CAPABILITIES,
    limit: viewerLimit,
    enabled: () => admin.memberAgentPolicy().enabled,
    connect: (client, agentId, body) => {
      enabled();
      const granted = access(body);

      const agent =
        agentId === null
          ? create(stringField(body, "name") ?? "", stringField(body, "iconName"), body)
          : find(agentId);

      replaceAccess(agent, granted);

      const connection: PersonalAgentConnection = {
        clientName: client.clientName,
        clientHost: client.clientHost,
        clientId: client.clientId,
        clientBlocked: false,
        revokedReason: null,
      };

      const credential: PersonalAgentCredential = {
        id: nextCredentialId++,
        kind: "oauth",
        name: client.clientName,
        lastFour: "oauth",
        createdAt: timestamp(ctx.now()),
        expiresAt: timestamp(ctx.now() + CONNECTION_DAYS * DAY),
        lastUsedAt: null,
        lastUsedIp: null,
        state: "active",
        connection,
      };

      agent.agent.credentials.unshift({
        ...credential,
        createdBy: ctx.world().users.get(VIEWER_ID)?.name ?? "You",
      });

      agents.set(agent.agent.id, agent);

      return { agentId: agent.agent.id, credentialId: credential.id };
    },
    revokeForReuse: (credentialId) => {
      const credential = connections().find((c) => c.id === credentialId);

      if (credential === undefined || credential.connection === null) return false;
      credential.state = "revoked";
      credential.connection = { ...credential.connection, revokedReason: "tokenReused" };

      return true;
    },
    setBlocked: (clientId, blocked) => {
      for (const credential of connections()) {
        if (credential.connection?.clientId === clientId)
          credential.connection = { ...credential.connection, clientBlocked: blocked };
      }
    },
    liveConnections: (clientId) =>
      connections().filter((c) => c.state === "active" && c.connection?.clientId === clientId)
        .length,
    hiddenRooms: () => {
      const offered = new Set(rooms().map((room) => room.id));

      // Rooms that still exist (not direct) that the viewer isn't in any more.
      const hidden = (id: number) => {
        const room = ctx.world().rooms.get(id);

        return room !== undefined && room.room.kind !== "direct" && !offered.has(id);
      };

      return [...current().values()].flatMap((record) => {
        const count = record.roomIds.filter(hidden).length;

        return count === 0 ? [] : [{ agentId: record.agent.id, count }];
      });
    },
    roomAccess: () =>
      [...current().values()].flatMap((record) => {
        const agent = present(record);
        const offered = rooms();
        // Suspended, or agents turned off: it can do nothing anywhere, as `present` says.
        const off = record.agent.suspended || !admin.memberAgentPolicy().enabled;

        return agent.roomIds.map((roomId) => ({
          agentId: agent.id,
          roomId,
          roomName: offered.find((room) => room.id === roomId)?.name ?? "",
          capabilities: off ? [] : roomCapabilities(record, roomId).sort(),
        }));
      }),
  };

  return {
    records: () => [...current().values()],
    revokeForMembership: (roomId: number, userId: number) => {
      if (userId !== VIEWER_ID) return;

      for (const record of current().values()) {
        for (const grant of record.agent.grants) {
          if (grant.roomId === roomId) grant.revoked = true;
        }
      }
    },
    credentialId: () => {
      current();

      return nextCredentialId++;
    },
    connections: connectionsApi,
    routes: [
      route("GET", /^\/me\/agents$/, () => {
        const policy = admin.memberAgentPolicy();

        const list: PersonalAgentList = {
          agents: [...current().values()].map(present),
          enabled: policy.enabled,
          limit: ctx.world().users.get(VIEWER_ID)?.role === "administrator" ? null : policy.limit,
          capabilities: CAPABILITIES,
          rooms: rooms(),
          mcpUrl: "http://127.0.0.1/agents/mcp",
          oauthAvailable: connectionsApi.available(),
        };

        return ok(list);
      }),
      route("POST", /^\/me\/agents$/, ({ body }) => {
        enabled();
        admin.requireSudo();
        current();
        const name = stringField(body, "name");

        if (name === null || name.trim() === "") throw validation("name", "can't be blank");
        const granted = access(body);
        const agent = create(name, stringField(body, "iconName"), body);
        agent.agent.provider = stringField(body, "provider");
        agent.agent.runtime = stringField(body, "runtime");
        replaceAccess(agent, granted);

        const issued = issue(agent, {
          name: stringField(body, "tokenName") ?? "Primary token",
          expiresAt: field(body, "expiresAt") ?? null,
        });

        agents.set(agent.agent.id, agent);

        return {
          status: 201,
          json: { agent: present(agent), secret: stringField(issued.json, "secret") },
        };
      }),
      route("GET", /^\/me\/agents\/(\d+)$/, ({ ids }) => ok(present(find(ids[0])))),
      route("PATCH", /^\/me\/agents\/(\d+)$/, ({ ids, body }) => {
        const agent = find(ids[0]);
        const cap = field(body, "dailyMessageCap");

        if (
          cap !== undefined &&
          cap !== null &&
          (!isNumber(cap) || !Number.isSafeInteger(cap) || cap <= 0)
        )
          throw validation("dailyMessageCap", "must be a positive integer or null");
        const name = stringField(body, "name");

        if (name !== null && name.trim() === "") throw validation("name", "can't be blank");
        const description = stringField(body, "description");

        if (description !== null && description.length > 500)
          throw validation("description", "must be at most 500 characters");

        repicture(agent, pictureOf(body));

        for (const key of ["name", "iconName", "provider", "runtime", "description"] as const) {
          const value = stringField(body, key);

          if (value !== null) {
            if (key === "name" && value.trim() === "") throw validation("name", "can't be blank");

            if (key === "name" || key === "iconName") agent[key] = value;
            else agent.agent[key] = value;
          }
        }

        if (cap !== undefined) {
          if (cap !== null && (!isNumber(cap) || !Number.isSafeInteger(cap) || cap <= 0))
            throw validation("dailyMessageCap", "must be a positive integer or null");
          agent.agent.caps[0] = cap;
        }

        return ok(present(agent));
      }),
      route("DELETE", /^\/me\/agents\/(\d+)$/, ({ ids }) => {
        const agent = find(ids[0]);
        current().delete(agent.agent.id);

        return ok({ id: agent.id });
      }),
      route("PUT", /^\/me\/agents\/(\d+)\/access$/, ({ ids, body }) => {
        const agent = find(ids[0]);
        enabled();
        replaceAccess(agent, access(body));

        return ok(present(agent));
      }),
      route("GET", /^\/me\/agents\/(\d+)\/credentials$/, ({ ids }) =>
        ok({ credentials: present(find(ids[0])).credentials }),
      ),
      route("POST", /^\/me\/agents\/(\d+)\/credentials$/, ({ ids, body }) => {
        const agent = find(ids[0]);
        enabled();
        admin.requireSudo();

        if (stringField(body, "name") === null) throw validation("name", "can't be blank");

        return issue(agent, body);
      }),
      route("POST", /^\/me\/agents\/(\d+)\/credentials\/(\d+)\/rotate$/, ({ ids, body }) => {
        const agent = find(ids[0]);
        const credential = agent.agent.credentials.find((c) => c.id === ids[1]);

        if (
          credential === undefined ||
          credential.state === "revoked" ||
          credential.connection !== null
        )
          throw notFound();
        enabled();
        admin.requireSudo();

        return issue(agent, body, credential);
      }),
      route("DELETE", /^\/me\/agents\/(\d+)\/credentials\/(\d+)$/, ({ ids }) => {
        const agent = find(ids[0]);
        const credential = agent.agent.credentials.find((c) => c.id === ids[1]);

        if (credential === undefined) throw notFound();
        credential.state = "revoked";

        return ok({ credentials: present(agent).credentials });
      }),
    ],
  };
}
