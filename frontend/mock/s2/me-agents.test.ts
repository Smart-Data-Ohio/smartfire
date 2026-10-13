import { describe, expect, it } from "vitest";
import { field, intField, type Json, stringField } from "../json.ts";
import { lookupIcon } from "./emoji.ts";
import { errorOf, harness, NOW, send } from "./testing.ts";

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
    const removed = await send(server, "DELETE", path);
    expect(removed.status).toBe(200);
    expect(intField(agent, "userId")).not.toBe(id);
    expect(intField(removed.json, "id")).toBe(intField(agent, "userId"));
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
  it("returns only effective capabilities when no room was granted", async () => {
    const { server } = harness();

    const created = await send(server, "POST", ROOT, {
      name: "No rooms",
      roomIds: [],
      capabilities: ["read_messages"],
    });

    expect(created.status).toBe(201);
    const agent = field(created.json, "agent");
    expect(field(agent, "capabilities")).toEqual([]);
    const id = intField(agent, "id");
    const shown = await server.handle({ method: "GET", path: `${ROOT}/${id}` });
    expect(field(shown.json, "capabilities")).toEqual([]);

    const updated = await send(server, "PUT", `${ROOT}/${id}/access`, {
      roomIds: [],
      capabilities: ["react"],
    });

    expect(updated.status).toBe(200);
    expect(field(updated.json, "capabilities")).toEqual([]);
  });
  it("filters rooms and effective capabilities after the owner leaves", async () => {
    const { server } = harness();

    const room = await send(server, "POST", "/api/v1/rooms", {
      type: "closed",
      clientRoomId: "d35352aa-5906-4d78-9236-673af121a781",
      name: "Agent room",
      iconName: null,
      userIds: [1, 2],
    });

    expect(room.status).toBe(201);
    const roomId = intField(field(room.json, "room"), "id");

    const created = await send(server, "POST", ROOT, {
      name: "Room helper",
      roomIds: [roomId],
      capabilities: ["read_messages"],
    });

    expect(created.status).toBe(201);
    const agent = field(created.json, "agent");
    expect(field(agent, "roomIds")).toEqual([roomId]);
    expect(field(agent, "capabilities")).toEqual(["read_messages"]);
    const id = intField(agent, "id");
    const path = `${ROOT}/${id}`;

    const left = await send(server, "PATCH", `/api/v1/rooms/${roomId}`, {
      type: "closed",
      userIds: [2],
    });

    expect(left.status).toBe(200);
    const shown = await server.handle({ method: "GET", path });
    expect(field(shown.json, "roomIds")).toEqual([]);
    expect(field(shown.json, "capabilities")).toEqual([]);
    const listed = await server.handle({ method: "GET", path: ROOT });
    const listedAgents = field(listed.json, "agents");
    expect(Array.isArray(listedAgents) && field(listedAgents[0], "roomIds")).toEqual([]);
    expect(Array.isArray(listedAgents) && field(listedAgents[0], "capabilities")).toEqual([]);
    const patched = await send(server, "PATCH", path, { description: "Still mine" });
    expect(patched.status).toBe(200);
    expect(field(patched.json, "roomIds")).toEqual([]);
    expect(field(patched.json, "capabilities")).toEqual([]);
  });
  it("rejects zero message caps without changing the profile", async () => {
    const { server } = harness();

    const created = await send(server, "POST", ROOT, {
      name: "Keep this name",
      roomIds: [],
      capabilities: [],
    });

    const id = intField(field(created.json, "agent"), "id");
    const path = `${ROOT}/${id}`;

    const rejected = await send(server, "PATCH", path, {
      name: "Must not save",
      dailyMessageCap: 0,
    });

    expect(rejected.status).toBe(422);
    const shown = await server.handle({ method: "GET", path });
    expect(stringField(shown.json, "name")).toBe("Keep this name");
    expect(field(shown.json, "dailyMessageCap")).toBe(200);
    expect((await send(server, "PATCH", path, { dailyMessageCap: 1 })).status).toBe(200);
    expect((await send(server, "PATCH", path, { dailyMessageCap: null })).status).toBe(200);
  });
  it("returns the creation time and resolved avatar icon on creates and reads", async () => {
    const { server, clock } = harness();

    const created = await send(server, "POST", ROOT, {
      name: "Branded",
      iconName: "claude",
      roomIds: [],
      capabilities: [],
    });

    expect(created.status).toBe(201);
    const agent = field(created.json, "agent");
    const id = intField(agent, "id");
    expect(stringField(agent, "createdAt")).toBe(new Date(NOW).toISOString());
    expect(stringField(agent, "iconUrl")).toBe(lookupIcon("claude")?.imageUrl);
    clock.advance(10000);

    for (const iconName of ["claude", "partyparrot", "rocket", ""]) {
      const patched = await send(server, "PATCH", `${ROOT}/${id}`, { iconName });
      expect(patched.status).toBe(200);
      expect(field(patched.json, "iconUrl")).toBe(lookupIcon(iconName)?.imageUrl ?? null);
      expect(stringField(patched.json, "createdAt")).toBe(new Date(NOW).toISOString());
      const shown = await send(server, "GET", `${ROOT}/${id}`);
      expect(shown.json).toEqual(patched.json);
      const listed = await send(server, "GET", ROOT);
      const agents = field(listed.json, "agents");
      expect(Array.isArray(agents) && agents[0]).toEqual(patched.json);
    }
  });
  it("marks member-list rooms private and open rooms public", async () => {
    const { server } = harness();

    for (const [index, kind] of ["open", "closed", "voice", "stage", "board"].entries()) {
      const room = {
        type: kind,
        clientRoomId: `d35352aa-5906-4d78-9236-673af121a78${index}`,
        name: `Privacy ${kind}`,
        iconName: null,
      };

      const created = await send(
        server,
        "POST",
        "/api/v1/rooms",
        kind === "open" ? room : { ...room, userIds: [1, 2] },
      );

      expect(created.status, JSON.stringify(created.json)).toBe(201);
      const id = intField(field(created.json, "room"), "id");
      const list = await send(server, "GET", ROOT);
      const rooms = field(list.json, "rooms");
      const choice = Array.isArray(rooms) ? rooms.find((r) => intField(r, "id") === id) : null;
      expect(field(choice, "private")).toBe(kind !== "open");
    }
  });
  it("sends readable disabled errors and restores effective grants when reenabled", async () => {
    const { server } = harness();
    const list = await send(server, "GET", ROOT);
    const rooms = field(list.json, "rooms");
    const roomId = Array.isArray(rooms) ? intField(rooms[0], "id") : null;
    expect(roomId).not.toBeNull();
    const body = { name: "Toggle helper", roomIds: [roomId], capabilities: ["read_messages"] };
    const created = await send(server, "POST", ROOT, body);
    expect(created.status).toBe(201);
    const agent = field(created.json, "agent");
    const id = intField(agent, "id");
    const credentials = field(agent, "credentials");
    const cid = Array.isArray(credentials) ? intField(credentials[0], "id") : null;
    expect(
      (await send(server, "PATCH", "/api/v1/admin/workspace", { memberAgentsEnabled: false }))
        .status,
    ).toBe(200);

    const requests: [string, string, Json][] = [
      ["POST", ROOT, body],
      ["POST", `${ROOT}/${id}/credentials`, { name: "Disabled" }],
      ["POST", `${ROOT}/${id}/credentials/${cid}/rotate`, {}],
      ["PUT", `${ROOT}/${id}/access`, { roomIds: [], capabilities: [] }],
    ];

    for (const [method, path, request] of requests) {
      const denied = await send(server, method, path, request);
      expect(denied.status).toBe(403);
      expect(errorOf(denied.json)).toEqual({
        tag: "Forbidden",
        message: "Personal agents are turned off for this workspace.",
      });
    }

    const paused = await send(server, "GET", `${ROOT}/${id}`);
    expect(field(paused.json, "capabilities")).toEqual([]);
    expect(field(paused.json, "credentials")).toEqual(credentials);
    expect(
      (await send(server, "PATCH", "/api/v1/admin/workspace", { memberAgentsEnabled: true }))
        .status,
    ).toBe(200);
    const resumed = await send(server, "GET", `${ROOT}/${id}`);
    expect(field(resumed.json, "capabilities")).toEqual(["read_messages"]);
    expect(field(resumed.json, "credentials")).toEqual(credentials);
  });
  it("reports the per-member limit as a form error", async () => {
    const { server } = harness();
    expect(
      (await send(server, "PATCH", "/api/v1/admin/workspace", { memberAgentLimit: 1 })).status,
    ).toBe(200);
    await send(server, "POST", "/__mock/viewer-role", { role: "member" });
    const body = { name: "One helper", roomIds: [], capabilities: [] };
    expect((await send(server, "POST", ROOT, body)).status).toBe(201);
    const denied = await send(server, "POST", ROOT, body);
    expect(denied.status).toBe(422);
    expect(field(field(denied.json, "error"), "fields")).toEqual({
      base: ["personal agent limit reached"],
    });
  });
});
