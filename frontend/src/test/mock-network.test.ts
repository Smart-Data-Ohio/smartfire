import { afterEach, describe, expect, it } from "vitest";
import { createMockServer, SEED_IDS } from "../../mock/server.ts";
import type { Me } from "../gen/Me.ts";
import type { ServerFrame } from "../gen/ServerFrame.ts";
import { installMockNetwork, type MockNetwork } from "./mock-network.ts";

let network: MockNetwork | null = null;

afterEach(() => {
  network?.restore();
  network = null;
});

function nextMessage(socket: WebSocket): Promise<ServerFrame> {
  return new Promise((resolve) => {
    socket.addEventListener(
      "message",
      (event) => {
        // SAFETY: the mock socket only ever sends serialized ServerFrames as text.
        resolve(JSON.parse(String(event.data)) as ServerFrame);
      },
      { once: true },
    );
  });
}

function closed(socket: WebSocket): Promise<{ code: number; wasClean: boolean }> {
  return new Promise((resolve) => {
    socket.onclose = (event) => resolve({ code: event.code, wasClean: event.wasClean });
  });
}

describe("installMockNetwork", () => {
  it("routes fetch for /api/v1 to the mock and rejects other URLs", async () => {
    network = installMockNetwork();

    const response = await fetch("/api/v1/me");
    const me: Me = await response.json();

    expect(response.status).toBe(200);
    expect(me.user.id).toBe(SEED_IDS.viewer);
    await expect(fetch("https://example.com/")).rejects.toThrow(TypeError);
  });

  it("sends JSON bodies and headers through, CSRF included", async () => {
    network = installMockNetwork();

    const path = `/api/v1/rooms/${SEED_IDS.rooms.quiet}/messages`;

    const body = JSON.stringify({
      clientMessageId: "net-1",
      markdownSource: "over the stub",
      replyToMessageId: null,
      replyNotifyAuthor: null,
    });

    const rejected = await fetch(path, { method: "POST", body });

    const created = await fetch(path, {
      method: "POST",
      body,
      headers: { "Content-Type": "application/json", "X-CSRF-Token": network.server.csrfToken() },
    });

    expect(rejected.status).toBe(422);
    expect(created.status).toBe(201);
  });

  it("opens a WebSocket asynchronously and bridges it to the sync hub", async () => {
    network = installMockNetwork();

    const socket = new WebSocket("/api/v1/sync");

    expect(socket.readyState).toBe(WebSocket.CONNECTING);
    await new Promise((resolve) => {
      socket.onopen = resolve;
    });

    const welcome = nextMessage(socket);

    socket.send(JSON.stringify({ t: "hello", v: 1, resume: null, topics: ["room:1"] }));
    expect(await welcome).toMatchObject({ t: "welcome", resumed: false });

    const typing = nextMessage(socket);

    network.server.typing(1, SEED_IDS.users.maya, true);
    expect(await typing).toMatchObject({
      t: "batch",
      events: [
        { topic: "room:1", type: "typing", data: { userId: SEED_IDS.users.maya, on: true } },
      ],
    });

    const done = closed(socket);

    socket.close(1000);
    expect(await done).toEqual({ code: 1000, wasClean: true });
    expect(network.server.syncState().connections).toBe(0);
  });

  it("reports a server-side drop as an unclean 1006 close", async () => {
    const server = createMockServer();

    network = installMockNetwork(server);

    const socket = new WebSocket("ws://localhost/api/v1/sync");

    await new Promise((resolve) => {
      socket.onopen = resolve;
    });

    const done = closed(socket);

    server.dropConnections();
    expect(await done).toEqual({ code: 1006, wasClean: false });
    expect(network.sockets).toHaveLength(1);
    server.dispose();
  });
});
