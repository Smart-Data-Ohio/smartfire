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
});
