import { Schema } from "effect";
import { describe, expect, it } from "vitest";
import { field, intField } from "../../../mock/json.ts";
import { harness, send } from "../../../mock/s2/testing.ts";
import { ROOM_IDS } from "../../../mock/seed.ts";
import {
  PersonalAgent,
  PersonalAgentCreated,
  PersonalAgentList,
  PersonalCredentialCreated,
  PersonalCredentialList,
} from "./me-agents.ts";

const ROOT = "/api/v1/me/agents";

describe("the personal agents mock against the pinned wire schemas", () => {
  it("decodes the list, a new agent, its tokens and their changes", async () => {
    const { server } = harness();
    const before = await server.handle({ method: "GET", path: ROOT });

    expect(() => Schema.decodeUnknownSync(PersonalAgentList)(before.json)).not.toThrow();

    const created = await send(server, "POST", ROOT, {
      name: "Helper",
      iconName: "owl",
      provider: "openai",
      runtime: "codex",
      roomIds: [ROOM_IDS.general],
      capabilities: ["read_messages", "post_messages"],
      tokenName: null,
      expiresAt: null,
    });

    expect(created.status).toBe(201);
    expect(() => Schema.decodeUnknownSync(PersonalAgentCreated)(created.json)).not.toThrow();

    const id = intField(field(created.json, "agent"), "id");
    const path = `${ROOT}/${id}`;
    const after = await server.handle({ method: "GET", path: ROOT });
    const one = await server.handle({ method: "GET", path });

    expect(() => Schema.decodeUnknownSync(PersonalAgentList)(after.json)).not.toThrow();
    expect(() => Schema.decodeUnknownSync(PersonalAgent)(one.json)).not.toThrow();

    const issued = await send(server, "POST", `${path}/credentials`, {
      name: "Laptop",
      expiresAt: "never",
    });

    expect(() => Schema.decodeUnknownSync(PersonalCredentialCreated)(issued.json)).not.toThrow();

    const cid = intField(field(issued.json, "credential"), "id");

    const rotated = await send(server, "POST", `${path}/credentials/${cid}/rotate`, {
      expiresAt: null,
    });

    const revoked = await send(server, "DELETE", `${path}/credentials/${cid}`);
    const access = await send(server, "PUT", `${path}/access`, { roomIds: [], capabilities: [] });

    expect(() => Schema.decodeUnknownSync(PersonalCredentialCreated)(rotated.json)).not.toThrow();
    expect(() => Schema.decodeUnknownSync(PersonalCredentialList)(revoked.json)).not.toThrow();
    expect(() => Schema.decodeUnknownSync(PersonalAgent)(access.json)).not.toThrow();
  });

  it("decodes a list with no public address, private rooms and an agent's icon and age", () => {
    const decoded = Schema.decodeUnknownSync(PersonalAgentList)({
      agents: [
        {
          id: 1,
          userId: 50,
          name: "Helper",
          iconName: "claude",
          iconUrl: "/icons/claude",
          provider: null,
          runtime: "codex",
          description: null,
          dailyMessageCap: null,
          suspended: false,
          roomIds: [],
          capabilities: [],
          lastUsedAt: null,
          createdAt: "2026-10-13T12:00:00Z",
          credentials: [],
        },
      ],
      enabled: true,
      limit: 3,
      capabilities: [],
      rooms: [{ id: 2, name: "launch-planning", private: true }],
      mcpUrl: null,
      oauthAvailable: false,
    });

    expect(decoded.mcpUrl).toBeNull();
    expect(decoded.rooms[0]?.private).toBe(true);
    expect(decoded.agents[0]?.iconUrl).toBe("/icons/claude");
  });

  it("answers a removal with the bot user's id, not the agent's", async () => {
    const { server } = harness();

    const body = {
      name: "Helper",
      iconName: null,
      provider: null,
      runtime: "codex",
      roomIds: [],
      capabilities: [],
      tokenName: null,
      expiresAt: null,
    };

    const created = await send(server, "POST", ROOT, body);
    const agent = field(created.json, "agent");
    const removed = await send(server, "DELETE", `${ROOT}/${intField(agent, "id")}`);

    expect(intField(removed.json, "id")).toBe(intField(agent, "userId"));
    expect(intField(agent, "userId")).not.toBe(intField(agent, "id"));
  });
});
