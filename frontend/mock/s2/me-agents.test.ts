import { describe, expect, it } from "vitest";
import { field, intField, stringField } from "../json.ts";
import { errorOf, harness, send } from "./testing.ts";

const ROOT = "/api/v1/me/agents";

describe("personal agents", () => {
  it("creates, reads once, rotates, revokes and removes", async () => {
    const { server } = harness();
    const listed = await server.handle({ method: "GET", path: ROOT });
    expect(listed.status).toBe(200);
    expect(field(listed.json, "oauthAvailable")).toBe(false);

    const created = await send(server, "POST", ROOT, {
      name: "My helper",
      roomIds: [],
      capabilities: [],
    });

    expect(created.status).toBe(201);
    const secret = stringField(created.json, "secret");
    expect(secret).toMatch(/^sfa_[a-f0-9]{64}$/);
    const agent = field(created.json, "agent");
    const id = intField(agent, "id");
    const path = `${ROOT}/${id}`;
    const credentials = await server.handle({ method: "GET", path: `${path}/credentials` });
    expect(credentials.status).toBe(200);
    expect(JSON.stringify(credentials.json)).not.toContain(secret);

    const issued = await send(server, "POST", `${path}/credentials`, {
      name: "Second",
      expiresAt: "never",
    });

    expect(issued.status).toBe(201);
    const cid = intField(field(issued.json, "credential"), "id");
    const rotated = await send(server, "POST", `${path}/credentials/${cid}/rotate`, {});
    expect(rotated.status).toBe(201);
    expect(stringField(rotated.json, "secret")).not.toBe(stringField(issued.json, "secret"));
    expect((await send(server, "DELETE", `${path}/credentials/${cid}`)).status).toBe(200);
    expect((await send(server, "DELETE", path)).status).toBe(200);
    expect((await server.handle({ method: "GET", path })).status).toBe(404);
    server.reset();
    expect((await server.handle({ method: "GET", path })).status).toBe(404);
  });
  it("keeps administrators unlimited and expires credentials", async () => {
    const { server, clock } = harness();
    await send(server, "PATCH", "/api/v1/admin/workspace", { memberAgentLimit: 1 });
    const body = { name: "helper", roomIds: [], capabilities: [] };
    const created = await send(server, "POST", ROOT, body);
    expect(created.status).toBe(201);
    expect((await send(server, "POST", ROOT, body)).status).toBe(201);
    const id = intField(field(created.json, "agent"), "id");
    const path = `${ROOT}/${id}`;
    expect(
      (await send(server, "PATCH", path, { name: "changed", dailyMessageCap: -1 })).status,
    ).toBe(422);
    const profile = await server.handle({ method: "GET", path });
    expect(stringField(profile.json, "name")).toBe("helper");
    clock.advance(91 * 86_400_000);
    const list = await server.handle({ method: "GET", path: `${path}/credentials` });
    const credentials = field(list.json, "credentials");
    expect(Array.isArray(credentials) && stringField(credentials[0], "state")).toBe("expired");
    expect((await send(server, "GET", `${ROOT}/999999`)).status).toBe(404);
  });
  it("guards CSRF, sudo, access and workspace settings", async () => {
    const { server } = harness();
    const body = { name: "helper", roomIds: [], capabilities: [] };
    expect((await server.handle({ method: "POST", path: ROOT, body })).status).toBe(422);
    await send(server, "POST", "/__mock/lapse-sudo", { on: true });
    expect(errorOf((await send(server, "POST", ROOT, body)).json).tag).toBe("SudoRequired");
  });
  it("rejects forbidden capabilities and rooms without creating", async () => {
    const { server } = harness();

    for (const capability of ["dm_anyone", "external_action", "fizzy"]) {
      expect(
        (await send(server, "POST", ROOT, { name: "bad", roomIds: [], capabilities: [capability] }))
          .status,
      ).toBe(422);
    }

    expect(
      (
        await send(server, "POST", ROOT, {
          name: "bad",
          roomIds: [999999],
          capabilities: ["read_messages"],
        })
      ).status,
    ).toBe(422);
    const listed = await server.handle({ method: "GET", path: ROOT });
    expect(field(listed.json, "agents")).toEqual([]);

    const disabled = await send(server, "PATCH", "/api/v1/admin/workspace", {
      memberAgentsEnabled: false,
      memberAgentLimit: 1,
    });

    expect(disabled.status).toBe(200);
    expect(
      (await send(server, "POST", ROOT, { name: "disabled", roomIds: [], capabilities: [] }))
        .status,
    ).toBe(403);
  });
});
