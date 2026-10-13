import { Schema } from "effect";
import { describe, expect, it } from "vitest";
import { PersonalAgentList } from "../../src/api/schema/me-agents.ts";
import { AdminOAuthClientList, OAuthConsent, OAuthRedirect } from "../../src/api/schema/oauth.ts";
import type { AdminOAuthClientList as ClientList } from "../../src/gen/AdminOAuthClientList.ts";
import type { OAuthConsent as Consent } from "../../src/gen/OAuthConsent.ts";
import type { PersonalAgentList as AgentList } from "../../src/gen/PersonalAgentList.ts";
import { field, intField, stringField } from "../json.ts";
import { ROOM_IDS } from "../seed.ts";
import { errorOf, get, harness, NOW, send } from "./testing.ts";

const AGENTS = "/api/v1/me/agents";

const CLIENTS = "/api/v1/admin/oauth_clients";

/** Makes a pending request as an app's authorize call would; its API path. */
async function authorize(
  server: ReturnType<typeof harness>["server"],
  body: Record<string, string> = {},
) {
  const made = await send(server, "POST", "/__mock/oauth-authorization", {
    state: "xyz",
    ...body,
  });

  expect(made.status).toBe(200);

  return `/api/v1/oauth/authorizations/${stringField(made.json, "publicId")}`;
}

describe("connecting an app (OAuth)", () => {
  it("shows the request, approves as a new agent and answers the app's callback", async () => {
    const { server } = harness();

    await send(server, "POST", "/__mock/oauth-available", { on: true });
    expect((await get<AgentList>(server, AGENTS)).oauthAvailable).toBe(true);

    const path = await authorize(server);
    const consent = await get<Consent>(server, path);

    expect(() => Schema.decodeUnknownSync(OAuthConsent)(consent)).not.toThrow();
    expect(consent.clientName).toBe("Example Assistant");
    expect(consent.redirectHostname).toBe("client.example");
    expect(consent.badge).toBe("unverifiedApp");
    expect(consent.selectedAgentId).toBeNull();

    const approved = await send(server, "POST", `${path}/approve`, {
      agentId: null,
      name: "Assistant",
      iconName: null,
      roomIds: [ROOM_IDS.general],
      capabilities: ["read_messages"],
    });

    expect(approved.status).toBe(200);
    expect(() => Schema.decodeUnknownSync(OAuthRedirect)(approved.json)).not.toThrow();

    const to = new URL(stringField(approved.json, "redirectTo") ?? "");

    expect(to.origin + to.pathname).toBe("https://client.example/callback");
    expect(to.searchParams.get("state")).toBe("xyz");
    expect(to.searchParams.get("code")).toMatch(/^mock_code_/);

    // Answered: the request is gone.
    expect((await server.handle({ method: "GET", path })).status).toBe(404);

    const list = await get<AgentList>(server, AGENTS);

    expect(() => Schema.decodeUnknownSync(PersonalAgentList)(list)).not.toThrow();

    const [agent] = list.agents;

    expect(agent?.name).toBe("Assistant");
    expect(agent?.roomIds).toEqual([ROOM_IDS.general]);
    expect(agent?.credentials[0]).toMatchObject({
      kind: "oauth",
      name: "Example Assistant",
      state: "active",
      expiresAt: new Date(NOW + 180 * 86_400_000).toISOString(),
      connection: { clientName: "Example Assistant", clientHost: "client.example" },
    });

    // The next request from the same app picks the agent it connected as.
    const again = await get<Consent>(server, await authorize(server));

    expect(again.selectedAgentId).toBe(agent?.id);
  });

  it("denies with access_denied, and refuses an expired request", async () => {
    const { server, clock } = harness();

    const path = await authorize(server, {
      clientName: "Desk app",
      redirectUri: "http://127.0.0.1:33418/callback",
    });

    expect((await get<Consent>(server, path)).badge).toBe("appOnThisComputer");

    const denied = await send(server, "POST", `${path}/deny`, {});
    const to = new URL(stringField(denied.json, "redirectTo") ?? "");

    expect(to.searchParams.get("error")).toBe("access_denied");
    expect(to.searchParams.get("state")).toBe("xyz");

    const late = await authorize(server);

    clock.advance(11 * 60_000);
    expect(errorOf((await server.handle({ method: "GET", path: late })).json).tag).toBe("NotFound");
  });

  it("asks for the confirmation before approving", async () => {
    const { server } = harness();
    const path = await authorize(server);

    await send(server, "POST", "/__mock/lapse-sudo", { on: true });

    const held = await send(server, "POST", `${path}/approve`, {
      agentId: null,
      name: "Assistant",
      iconName: null,
      roomIds: [],
      capabilities: [],
    });

    expect(errorOf(held.json).tag).toBe("SudoRequired");
    expect((await get<AgentList>(server, AGENTS)).agents).toEqual([]);
  });

  it("lists connected apps for administrators, and blocking one marks its connections", async () => {
    const { server } = harness();

    expect(await get<ClientList>(server, CLIENTS)).toEqual({ clients: [], oauthAvailable: false });

    const connected = await send(server, "POST", "/__mock/oauth-connection", {
      clientName: "Claude",
      redirectUri: "https://claude.ai/api/mcp/auth_callback",
      name: "My Claude",
    });

    const clientId = intField(connected.json, "clientId");
    const listed = await get<ClientList>(server, CLIENTS);

    expect(() => Schema.decodeUnknownSync(AdminOAuthClientList)(listed)).not.toThrow();
    expect(listed.clients).toMatchObject([
      { id: clientId, clientName: "Claude", clientHost: "claude.ai", liveConnections: 1 },
    ]);

    const blocked = await send(server, "PUT", `${CLIENTS}/${clientId}/block`);

    expect(stringField(blocked.json, "blockedAt")).not.toBeNull();

    const credential = (await get<AgentList>(server, AGENTS)).agents[0]?.credentials[0];

    expect(credential?.connection?.clientBlocked).toBe(true);

    const unblocked = await send(server, "DELETE", `${CLIENTS}/${clientId}/block`);

    expect(field(unblocked.json, "blockedAt")).toBeNull();
    expect((await send(server, "PUT", `${CLIENTS}/999/block`)).status).toBe(404);
  });

  it("ends a connection for a reused sign-in token, with the reason", async () => {
    const { server } = harness();
    const connected = await send(server, "POST", "/__mock/oauth-connection", { name: "Helper" });
    const agentId = intField(connected.json, "agentId");
    const credentialId = intField(connected.json, "credentialId");

    // A live connection can't be rotated into a token.
    const rotate = `${AGENTS}/${agentId}/credentials/${credentialId}/rotate`;

    expect((await send(server, "POST", rotate, {})).status).toBe(404);

    await send(server, "POST", "/__mock/oauth-reuse", { credentialId });

    const credential = (await get<AgentList>(server, AGENTS)).agents[0]?.credentials[0];

    expect(credential?.state).toBe("revoked");
    expect(credential?.connection?.revokedReason).toBe("tokenReused");
  });
});
