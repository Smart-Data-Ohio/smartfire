/**
 * Connecting an app as a personal agent (OAuth): the approval page's API under
 * `/oauth/authorizations/*` and the administrators' client list under `/admin/oauth_clients`.
 * There is no authorize endpoint: `/__mock/oauth-authorization` makes the pending request an app's
 * authorize call would, and the approve and deny answers name the app's callback.
 */
import type { AdminOAuthClient } from "../../src/gen/AdminOAuthClient.ts";
import type { OAuthClientBadge } from "../../src/gen/OAuthClientBadge.ts";
import type { OAuthConsent } from "../../src/gen/OAuthConsent.ts";
import { forbidden, type MockResponse, notFound, ok } from "../http.ts";
import { field, intField, isNumber, type Json, stringField } from "../json.ts";
import { timestamp, VIEWER_ID, type World } from "../seed.ts";
import type { AdminModule } from "./admin.ts";
import { route, type S2Context } from "./context.ts";
import type { AgentConnections } from "./me-agents.ts";

/** How long a pending request lasts (the server's 10 minutes). */
const PENDING_MS = 10 * 60_000;

/** The issuer the callbacks name, as the server's `iss`. */
const ISSUER = "http://127.0.0.1";

interface ClientRecord {
  readonly id: number;
  readonly clientName: string;
  readonly clientId: string;
  readonly clientHost: string;
  readonly redirectUri: string;
  readonly badge: OAuthClientBadge;
  lastConnectedAt: number | null;
  blockedAt: number | null;
}

interface AuthorizationRecord {
  readonly clientId: number;
  readonly state: string | null;
  readonly expiresAt: number;
  status: "pending" | "approved" | "denied";
}

/** A client's id is its metadata document's URL; its host is that URL's (or the whole id). */
function hostOf(clientId: string): string {
  try {
    return new URL(clientId).hostname || clientId;
  } catch {
    return clientId;
  }
}

/** The callback with its query, as the server's `oauth::callback`. */
function callback(redirectUri: string, params: Record<string, string | null>): string {
  const url = new URL(redirectUri);

  for (const [key, value] of Object.entries(params)) {
    if (value !== null) url.searchParams.set(key, value);
  }

  return url.toString();
}

export function createOAuth(ctx: S2Context, admin: AdminModule, agents: AgentConnections) {
  let world: World | null = null;
  const clients = new Map<number, ClientRecord>();
  const authorizations = new Map<string, AuthorizationRecord>();
  let nextClientId = 1;

  const current = () => {
    if (world !== ctx.world()) {
      world = ctx.world();
      clients.clear();
      authorizations.clear();
      nextClientId = 1;
    }
  };

  const administrator = () => {
    if (ctx.world().users.get(VIEWER_ID)?.role !== "administrator") throw forbidden();
  };

  /** The client named so (made on first use), at `redirectUri`. */
  const clientFor = (clientName: string, redirectUri: string): ClientRecord => {
    current();

    const existing = [...clients.values()].find(
      (client) => client.clientName === clientName && client.redirectUri === redirectUri,
    );

    if (existing !== undefined) return existing;
    const loopback = /^http:\/\/(127\.0\.0\.1|localhost|\[::1\])(:\d+)?\//.test(redirectUri);

    const clientId = loopback
      ? `https://apps.example/${clientName.toLowerCase().replaceAll(/[^a-z0-9]+/g, "-")}.json`
      : `${new URL(redirectUri).origin}/oauth/client.json`;

    const client: ClientRecord = {
      id: nextClientId++,
      clientName,
      clientId,
      clientHost: hostOf(clientId),
      redirectUri,
      badge: loopback ? "appOnThisComputer" : "unverifiedApp",
      lastConnectedAt: null,
      blockedAt: null,
    };

    clients.set(client.id, client);

    return client;
  };

  /** The viewer's pending request with this id; 404 when it expired, was answered or is unknown. */
  const pending = (path: string) => {
    current();
    const publicId = /\/oauth\/authorizations\/([^/]+)/.exec(path)?.[1] ?? "";
    const authorization = authorizations.get(publicId);

    if (
      authorization === undefined ||
      authorization.status !== "pending" ||
      authorization.expiresAt <= ctx.now()
    )
      throw notFound("Not found");
    const client = clients.get(authorization.clientId);

    if (client === undefined || client.blockedAt !== null) throw notFound("Not found");

    return { authorization, client };
  };

  const consent = (path: string): OAuthConsent => {
    const { client } = pending(path);
    const listed = agents.agents();

    const last = listed.find((agent) =>
      agent.credentials.some(
        (credential) => credential.connection?.clientHost === client.clientHost,
      ),
    );

    return {
      clientName: client.clientName,
      redirectHostname: new URL(client.redirectUri).hostname,
      badge: client.badge,
      agents: listed,
      selectedAgentId: last?.id ?? null,
      rooms: agents.rooms(),
      capabilities: [...agents.capabilities],
      enabled: agents.enabled(),
      limit: agents.limit(),
      accessAppliesToAllConnections: true,
    };
  };

  const present = (client: ClientRecord): AdminOAuthClient => ({
    id: client.id,
    clientName: client.clientName,
    clientId: client.clientId,
    clientHost: client.clientHost,
    liveConnections: agents.liveConnections(client.clientHost),
    lastConnectedAt: client.lastConnectedAt === null ? null : timestamp(client.lastConnectedAt),
    blockedAt: client.blockedAt === null ? null : timestamp(client.blockedAt),
  });

  const setBlocked = (id: number | undefined, blocked: boolean): MockResponse => {
    administrator();
    current();
    const client = id === undefined ? undefined : clients.get(id);

    if (client === undefined) throw notFound();
    client.blockedAt = blocked ? (client.blockedAt ?? ctx.now()) : null;
    agents.setBlocked(client.clientHost, blocked);

    return ok(present(client));
  };

  /**
   * The controls: `oauth-available` ({on}); `oauth-authorization` ({clientName, redirectUri,
   * state}) makes a pending request and answers its `publicId` and approval `path`;
   * `oauth-connection` ({clientName, redirectUri, agentId | name, roomIds, capabilities})
   * connects an agent as approving would; `oauth-reuse` ({credentialId}) ends a connection for a
   * reused sign-in token. `null` for any other control.
   */
  const control = (action: string, body: Json | undefined): MockResponse | null => {
    switch (action) {
      case "oauth-available": {
        const on = field(body, "on");

        agents.setAvailable(on !== false);

        return ok({ ok: true });
      }

      case "oauth-authorization": {
        const client = clientFor(
          stringField(body, "clientName") ?? "Example Assistant",
          stringField(body, "redirectUri") ?? "https://client.example/callback",
        );

        const publicId = ctx.hex(32);

        authorizations.set(publicId, {
          clientId: client.id,
          state: stringField(body, "state"),
          expiresAt: ctx.now() + PENDING_MS,
          status: "pending",
        });

        return ok({ publicId, path: `/app/oauth/consent/${publicId}`, clientId: client.id });
      }

      case "oauth-connection": {
        const client = clientFor(
          stringField(body, "clientName") ?? "Example Assistant",
          stringField(body, "redirectUri") ?? "https://client.example/callback",
        );

        const agentId = intField(body, "agentId");

        const connected = agents.connect(client, agentId, {
          name: stringField(body, "name") ?? "Connected agent",
          iconName: stringField(body, "iconName"),
          roomIds: field(body, "roomIds") ?? [],
          capabilities: field(body, "capabilities") ?? [...agents.capabilities],
        });

        client.lastConnectedAt = ctx.now();

        return ok({ ...connected, clientId: client.id });
      }

      case "oauth-reuse": {
        const id = field(body, "credentialId");

        if (!isNumber(id) || !agents.revokeForReuse(id)) throw notFound("No such connection");

        return ok({ ok: true });
      }

      default:
        return null;
    }
  };

  return {
    control,
    routes: [
      route("GET", /^\/oauth\/authorizations\/[^/]+$/, ({ path }) => ok(consent(path))),
      route("POST", /^\/oauth\/authorizations\/[^/]+\/approve$/, ({ path, body }) => {
        const { authorization, client } = pending(path);

        admin.requireSudo();
        const agentId = field(body, "agentId");

        agents.connect(client, isNumber(agentId) ? agentId : null, body);
        authorization.status = "approved";
        client.lastConnectedAt = ctx.now();

        return ok({
          redirectTo: callback(client.redirectUri, {
            code: `mock_code_${ctx.hex(16)}`,
            state: authorization.state,
            iss: ISSUER,
          }),
        });
      }),
      route("POST", /^\/oauth\/authorizations\/[^/]+\/deny$/, ({ path }) => {
        const { authorization, client } = pending(path);

        authorization.status = "denied";

        return ok({
          redirectTo: callback(client.redirectUri, {
            error: "access_denied",
            state: authorization.state,
            iss: ISSUER,
          }),
        });
      }),
      route("GET", /^\/admin\/oauth_clients$/, () => {
        administrator();
        current();

        const listed = [...clients.values()]
          .filter((client) => client.blockedAt !== null || client.lastConnectedAt !== null)
          .sort(
            (a, b) =>
              Number(b.blockedAt !== null) - Number(a.blockedAt !== null) ||
              (b.lastConnectedAt ?? 0) - (a.lastConnectedAt ?? 0),
          )
          .map(present);

        return ok({ clients: listed, oauthAvailable: agents.available() });
      }),
      route("PUT", /^\/admin\/oauth_clients\/(\d+)\/block$/, ({ ids }) => setBlocked(ids[0], true)),
      route("DELETE", /^\/admin\/oauth_clients\/(\d+)\/block$/, ({ ids }) =>
        setBlocked(ids[0], false),
      ),
    ],
  };
}
