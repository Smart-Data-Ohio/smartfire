import type { BotSummary } from "../../src/gen/BotSummary.ts";
import type { OAuthAgentRoomAccess } from "../../src/gen/OAuthAgentRoomAccess.ts";
import type { PersonalAgent } from "../../src/gen/PersonalAgent.ts";
import type { PersonalAgentConnection } from "../../src/gen/PersonalAgentConnection.ts";
import type { PersonalAgentCredential } from "../../src/gen/PersonalAgentCredential.ts";
import type { PersonalAgentList } from "../../src/gen/PersonalAgentList.ts";
import { forbidden, notFound, ok, validation } from "../http.ts";
import { field, isNumber, type Json, stringArrayField, stringField } from "../json.ts";
import { timestamp, VIEWER_ID, type World } from "../seed.ts";
import type { AdminModule } from "./admin.ts";
import { route, type S2Context } from "./context.ts";
import { lookupIcon } from "./emoji.ts";

const CAPABILITIES = ["read_messages", "post_messages", "react", "manage_threads"];

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
}

interface AgentRecord {
  profile: Omit<PersonalAgent, "roomIds" | "capabilities" | "iconUrl">;
  roomIds: number[];
  grants: Map<number, string[]>;
}

export function createMeAgents(ctx: S2Context, admin: AdminModule) {
  let world: World | null = null;
  const agents = new Map<number, AgentRecord>();
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

    const grantable = new Set(rooms().map((room) => room.id));

    for (const agent of agents.values()) {
      for (const roomId of agent.grants.keys()) {
        if (!grantable.has(roomId)) agent.grants.delete(roomId);
      }

      for (const credential of agent.profile.credentials) {
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

    if (agent === undefined) throw notFound();

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

  const replaceAccess = (record: AgentRecord, next: ReturnType<typeof access>) => {
    record.roomIds = next.roomIds;
    record.grants.clear();

    for (const roomId of next.roomIds) record.grants.set(roomId, [...next.capabilities]);
  };

  const present = (record: AgentRecord): PersonalAgent => {
    const grantable = new Set(rooms().map((room) => room.id));
    const roomIds = record.roomIds.filter((id) => grantable.has(id));

    const capabilities =
      record.profile.suspended || !admin.memberAgentPolicy().enabled
        ? []
        : [...new Set(roomIds.flatMap((id) => record.grants.get(id) ?? []))].sort();

    const iconUrl =
      lookupIcon(record.profile.iconName ?? "", admin.uploadedIcons())?.imageUrl ?? null;

    return { ...record.profile, iconUrl, roomIds, capabilities };
  };

  const enabled = () => {
    if (!admin.memberAgentPolicy().enabled) throw forbidden(DISABLED);
  };

  const issue = (
    agent: AgentRecord,
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
    agent.profile.credentials.unshift(credential);

    return { status: 201, json: { credential, secret } };
  };

  let oauthAvailable = false;
  let oauthWorld: World | null = null;

  const connections = (): PersonalAgentCredential[] =>
    [...current().values()].flatMap((agent) =>
      agent.profile.credentials.filter((credential) => credential.connection !== null),
    );

  const viewerLimit = () =>
    ctx.world().users.get(VIEWER_ID)?.role === "administrator"
      ? null
      : admin.memberAgentPolicy().limit;

  const create = (name: string, iconName: string | null): AgentRecord => {
    const limit = viewerLimit();
    const existing = current();

    if (limit !== null && existing.size >= limit)
      throw validation("base", "personal agent limit reached", "Personal agent limit reached");

    if (name.trim() === "") throw validation("name", "can't be blank");

    const profile: AgentRecord["profile"] = {
      id: nextId++,
      userId: nextUserId++,
      name,
      iconName,
      provider: null,
      runtime: null,
      description: null,
      dailyMessageCap: 200,
      suspended: false,
      lastUsedAt: null,
      createdAt: timestamp(ctx.now()),
      credentials: [],
    };

    const agent: AgentRecord = { profile, roomIds: [], grants: new Map() };

    agents.set(profile.id, agent);

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
          ? create(stringField(body, "name") ?? "", stringField(body, "iconName"))
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

      agent.profile.credentials.unshift(credential);

      return { agentId: agent.profile.id, credentialId: credential.id };
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
    roomAccess: () =>
      [...current().values()].flatMap((record) => {
        const agent = present(record);
        const offered = rooms();
        // Suspended, or agents turned off: it can do nothing anywhere, as `present` says.
        const off = record.profile.suspended || !admin.memberAgentPolicy().enabled;

        return agent.roomIds.map((roomId) => ({
          agentId: agent.id,
          roomId,
          roomName: offered.find((room) => room.id === roomId)?.name ?? "",
          capabilities: off ? [] : [...(record.grants.get(roomId) ?? [])].sort(),
        }));
      }),
  };

  const summaries = (): BotSummary[] =>
    [...current().values()].map(({ profile }) => ({
      id: profile.userId,
      name: profile.name,
      avatarUrl: "/avatar.svg",
      icon: null,
      rooms: [],
      ownership: `Personal agent · Owned by ${ctx.world().users.get(VIEWER_ID)?.name ?? "You"}`,
    }));

  return {
    summaries,
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
        const policy = admin.memberAgentPolicy();

        if (
          ctx.world().users.get(VIEWER_ID)?.role !== "administrator" &&
          agents.size >= policy.limit
        )
          throw validation("base", "personal agent limit reached", "Personal agent limit reached");
        const name = stringField(body, "name");

        if (name === null || name.trim() === "") throw validation("name", "can't be blank");
        const granted = access(body);
        const id = nextId;

        const profile: AgentRecord["profile"] = {
          id,
          userId: nextUserId,
          name,
          iconName: stringField(body, "iconName"),
          provider: stringField(body, "provider"),
          runtime: stringField(body, "runtime"),
          description: null,
          dailyMessageCap: 200,
          suspended: false,
          lastUsedAt: null,
          createdAt: timestamp(ctx.now()),
          credentials: [],
        };

        const agent: AgentRecord = { profile, roomIds: [], grants: new Map() };

        replaceAccess(agent, granted);

        const issued = issue(agent, {
          name: stringField(body, "tokenName") ?? "Primary token",
          expiresAt: field(body, "expiresAt") ?? null,
        });

        agents.set(id, agent);
        nextId++;
        nextUserId++;

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

        for (const key of ["name", "iconName", "provider", "runtime", "description"] as const) {
          const value = stringField(body, key);

          if (value !== null) {
            if (key === "name" && value.trim() === "") throw validation("name", "can't be blank");
            agent.profile[key] = value;
          }
        }

        if (cap !== undefined) {
          if (cap !== null && (!isNumber(cap) || !Number.isSafeInteger(cap) || cap <= 0))
            throw validation("dailyMessageCap", "must be a positive integer or null");
          agent.profile.dailyMessageCap = cap;
        }

        return ok(present(agent));
      }),
      route("DELETE", /^\/me\/agents\/(\d+)$/, ({ ids }) => {
        const agent = find(ids[0]);
        current().delete(agent.profile.id);

        return ok({ id: agent.profile.userId });
      }),
      route("PUT", /^\/me\/agents\/(\d+)\/access$/, ({ ids, body }) => {
        const agent = find(ids[0]);
        enabled();
        replaceAccess(agent, access(body));

        return ok(present(agent));
      }),
      route("GET", /^\/me\/agents\/(\d+)\/credentials$/, ({ ids }) =>
        ok({ credentials: find(ids[0]).profile.credentials }),
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
        const credential = agent.profile.credentials.find((c) => c.id === ids[1]);

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
        const credential = agent.profile.credentials.find((c) => c.id === ids[1]);

        if (credential === undefined) throw notFound();
        credential.state = "revoked";

        return ok({ credentials: agent.profile.credentials });
      }),
    ],
  };
}
