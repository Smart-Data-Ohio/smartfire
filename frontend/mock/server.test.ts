import { describe, expect, it } from "vitest";
import type { MessageDTO } from "../src/gen/MessageDTO.ts";
import type { MessagePage } from "../src/gen/MessagePage.ts";
import type { ReadState } from "../src/gen/ReadState.ts";
import type { RoomDetail } from "../src/gen/RoomDetail.ts";
import type { Sidebar } from "../src/gen/Sidebar.ts";
import type { SyncEvent } from "../src/gen/SyncEvent.ts";
import { field, type Json, stringField } from "./json.ts";
import { manualScheduler } from "./scheduler.ts";
import { createMockServer, type MockServer, PAGE_SIZE, SEED_IDS } from "./server.ts";

const NOW = Date.UTC(2026, 9, 6, 16, 30, 0);

const { rooms } = SEED_IDS;

function quietServer(seed = 1): MockServer {
  return createMockServer({ now: () => NOW, seed, scheduler: manualScheduler(NOW) });
}

async function get<T>(server: MockServer, path: string): Promise<T> {
  const response = await server.handle({ method: "GET", path });

  expect(response.status).toBe(200);

  // SAFETY: the mock builds these bodies from the generated wire types; the tests name which.
  return response.json as T;
}

function send(server: MockServer, method: string, path: string, body: Json, token?: string) {
  return server.handle({
    method,
    path,
    body,
    headers: { "X-CSRF-Token": token ?? server.csrfToken() },
  });
}

function createBody(clientMessageId: string, markdownSource: string): Json {
  return { clientMessageId, markdownSource, replyToMessageId: null, replyNotifyAuthor: null };
}

interface ErrorSummary {
  readonly tag: string | null;
  readonly message: string | null;
}

/** The `_tag` and message of an `{"error": ...}` body. */
function errorOf(json: Json): ErrorSummary {
  const error = field(json, "error");

  return { tag: stringField(error, "_tag"), message: stringField(error, "message") };
}

const page = (server: MockServer, roomId: number, query = "") =>
  get<MessagePage>(server, `/api/v1/rooms/${roomId}/messages${query}`);

function collect(server: MockServer, topics: string[] = []): SyncEvent[] {
  const events: SyncEvent[] = [];

  const connection = server.connect((frame) => {
    if (frame.t === "batch") events.push(...frame.events);
  });

  connection.receive({ t: "hello", v: 1, resume: null, topics });

  return events;
}

describe("seed", () => {
  it("is deterministic for a seed and clock", async () => {
    const [a, b, other] = [quietServer(1), quietServer(1), quietServer(2)];

    expect(await get(a, "/api/v1/sidebar")).toEqual(await get(b, "/api/v1/sidebar"));
    expect(await page(a, rooms.general)).toEqual(await page(b, rooms.general));
    expect(a.csrfToken()).toBe(b.csrfToken());
    expect(await page(a, rooms.general)).not.toEqual(await page(other, rooms.general));
  });

  it("speaks the wire format: RFC 3339 millisecond UTC timestamps, nulls present", async () => {
    const { messages } = await page(quietServer(), rooms.general);
    const message = messages[0];

    expect(message?.createdAt).toMatch(/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{3}Z$/);
    expect(message).toHaveProperty("threadId", null);
    expect(message).toHaveProperty("replyToMessageId", null);
    expect(Date.parse(message?.createdAt ?? "")).toBeLessThan(NOW);
  });

  it("seeds the rooms the screenshots need", async () => {
    const server = quietServer();
    const sidebar = await get<Sidebar>(server, "/api/v1/sidebar");
    const row = (id: number) => sidebar.rows.find((candidate) => candidate.room.id === id);

    expect(row(rooms.general)?.unreadCount).toBeGreaterThan(PAGE_SIZE);
    expect(row(rooms.general)?.mentionCount).toBe(1);
    expect(row(rooms.quiet)?.unreadCount).toBe(0);
    expect(row(rooms.dmEmber)?.displayName).toBe("Ember");
    expect(row(rooms.random)?.membership.involvement).toBe("muted");
    expect(row(rooms.engineering)?.membership.favoritePosition).toBe(1);
    expect(sidebar.categories.map((category) => category.name)).toEqual(["Launch", "Team"]);
    expect(await page(server, rooms.quiet)).toEqual({
      messages: [],
      users: [],
      before: null,
      after: null,
      saved: [],
    });

    const detail = await get<RoomDetail>(server, `/api/v1/rooms/${rooms.general}`);

    expect(detail.unread?.count).toBe(row(rooms.general)?.unreadCount);
  });
});

describe("message paging", () => {
  it("walks from the newest page back to the start with correct cursors", async () => {
    const server = quietServer();
    let current = await page(server, rooms.general);
    const seen: number[] = [];

    expect(current.messages).toHaveLength(PAGE_SIZE);
    expect(current.after).toBeNull();
    expect(current.before).toBe(current.messages[0]?.id);

    for (;;) {
      seen.unshift(...current.messages.map((message) => message.id));

      if (current.before === null) break;

      const older = await page(server, rooms.general, `?before=${current.before}`);

      expect(older.after).toBe(older.messages.at(-1)?.id);
      current = older;
    }

    expect(seen).toHaveLength(400);
    expect([...seen].sort((a, b) => a - b)).toEqual(seen);
    expect(current.messages.length).toBeLessThanOrEqual(PAGE_SIZE);
  });

  it("pages forward with after until the present", async () => {
    const server = quietServer();
    const newest = await page(server, rooms.engineering);
    const start = newest.messages[0]?.id ?? 0;
    const after = await page(server, rooms.engineering, `?after=${start}`);

    expect(after.messages[0]?.id).toBeGreaterThan(start);
    expect(after.after).toBeNull();
    expect(after.before).toBe(after.messages[0]?.id);
  });

  it("returns 40 on each side of an around anchor, clipped at the ends", async () => {
    const server = quietServer();
    const detail = await get<RoomDetail>(server, `/api/v1/rooms/${rooms.general}`);
    const anchor = detail.unread?.firstUnreadMessageId ?? 0;
    const around = await page(server, rooms.general, `?around=${anchor}`);
    const index = around.messages.findIndex((message) => message.id === anchor);

    expect(index).toBe(PAGE_SIZE);
    expect(around.messages.length).toBeLessThanOrEqual(2 * PAGE_SIZE + 1);
    expect(around.before).toBe(around.messages[0]?.id);

    const tail = await page(server, rooms.general);
    const lastId = tail.messages.at(-1)?.id ?? 0;
    const atEnd = await page(server, rooms.general, `?around=${lastId}`);

    expect(atEnd.after).toBeNull();
    expect(atEnd.messages.at(-1)?.id).toBe(lastId);
  });

  it("answers 404 for an unknown room or anchor", async () => {
    const server = quietServer();
    const room = await server.handle({ method: "GET", path: "/api/v1/rooms/999" });

    const anchor = await server.handle({
      method: "GET",
      path: `/api/v1/rooms/${rooms.general}/messages?before=1`,
    });

    expect(room.status).toBe(404);
    expect(errorOf(room.json)).toEqual({ tag: "NotFound", message: "Room not found" });
    expect(anchor.status).toBe(404);
  });
});

describe("creating messages", () => {
  it("is idempotent by clientMessageId: 201, then 200 with the same message", async () => {
    const server = quietServer();
    const path = `/api/v1/rooms/${rooms.quiet}/messages`;
    const first = await send(server, "POST", path, createBody("c-1", "hello"));
    const again = await send(server, "POST", path, createBody("c-1", "hello"));

    expect(first.status).toBe(201);
    expect(again).toEqual({ status: 200, json: first.json });
    expect((await page(server, rooms.quiet)).messages).toHaveLength(1);
  });

  it("renders Markdown to sanitized HTML with mentions as the server marks them up", async () => {
    const server = quietServer();

    const markdown =
      "Hi **team**, see *this* and `code` at https://example.com/a?b=1 <script>x</script> @Maya\n\n```ts\nconst a = 1 < 2;\n```";

    const response = await send(
      server,
      "POST",
      `/api/v1/rooms/${rooms.quiet}/messages`,
      createBody("c-2", markdown),
    );

    const html = stringField(response.json, "bodyHtml") ?? "";

    expect(html).toContain("<strong>team</strong>");
    expect(html).toContain("<em>this</em>");
    expect(html).toContain("<code>code</code>");
    expect(html).toContain('<a href="https://example.com/a?b=1"');
    expect(html).toContain("&lt;script&gt;");
    expect(html).not.toContain("<script>");
    expect(html).toContain(
      '<div class="mention mention--user-2" sgid="mock-2" data-user-id="2"><a title="Maya Okafor" class="btn avatar" href="/users/2">',
    );
    expect(html).toContain('<pre><code class="language-ts">const a = 1 &lt; 2;</code></pre>');
  });

  it("rejects a blank body with a Validation error", async () => {
    const server = quietServer();

    const response = await send(
      server,
      "POST",
      `/api/v1/rooms/${rooms.quiet}/messages`,
      createBody("c-3", "  "),
    );

    expect(response.status).toBe(422);
    expect(errorOf(response.json).tag).toBe("Validation");
    expect(response.json).toMatchObject({ error: { fields: { body: [expect.any(String)] } } });
  });
});

describe("CSRF", () => {
  it("rejects a write without the token, and the old token after a rotation", async () => {
    const server = quietServer();
    const path = `/api/v1/rooms/${rooms.general}/read`;
    const missing = await server.handle({ method: "POST", path });
    const old = server.csrfToken();
    const rotated = await server.handle({ method: "POST", path: "/__mock/rotate-csrf" });
    const stale = await send(server, "POST", path, null, old);
    const boot = await get<Json>(server, "/api/v1/boot");
    const fresh = await send(server, "POST", path, null, stringField(boot, "csrfToken") ?? "");

    expect(missing.status).toBe(422);
    expect(errorOf(missing.json)).toEqual({
      tag: "InvalidAuthenticityToken",
      message: "Can't verify CSRF token authenticity.",
    });
    expect(stringField(rotated.json, "csrfToken")).not.toBe(old);
    expect(stale.status).toBe(422);
    expect(fresh.status).toBe(200);
  });
});

describe("holdSends", () => {
  it("keeps message creation pending until released", async () => {
    const server = quietServer();
    const events = collect(server, [`room:${rooms.quiet}`]);
    let settled = false;

    server.holdSends(true);

    const pending = send(
      server,
      "POST",
      `/api/v1/rooms/${rooms.quiet}/messages`,
      createBody("held-1", "wait for it"),
    ).then((response) => {
      settled = true;

      return response;
    });

    await Promise.resolve();
    await Promise.resolve();
    expect(settled).toBe(false);
    expect(server.pendingSends()).toBe(1);
    expect(events.some((event) => event.type === "message.created")).toBe(false);

    server.releaseSends();

    const response = await pending;

    expect(response.status).toBe(201);
    expect(events.some((event) => event.type === "message.created")).toBe(true);
  });
});

describe("read state", () => {
  it("marks a room read and unread again", async () => {
    const server = quietServer();
    const events = collect(server);
    const path = `/api/v1/rooms/${rooms.design}/read`;
    const read = await send(server, "POST", path, null);

    expect(read.json).toEqual({ roomId: rooms.design, unread: false, firstUnreadMessageId: null });
    expect((await get<RoomDetail>(server, `/api/v1/rooms/${rooms.design}`)).unread).toBeNull();

    const { messages } = await page(server, rooms.design);
    const target = messages.at(-3)?.id ?? 0;
    const unread = await send(server, "DELETE", path, { messageId: target });
    const state: ReadState = { roomId: rooms.design, unread: true, firstUnreadMessageId: target };

    expect(unread.json).toEqual(state);
    expect((await get<RoomDetail>(server, `/api/v1/rooms/${rooms.design}`)).unread).toEqual({
      firstUnreadMessageId: target,
      count: 3,
    });
    expect(events.map((event) => event.type)).toEqual(["room.read", "room.unread"]);
  });
});

describe("users and presence", () => {
  it("lists users in id order and leaves bots out of presence", async () => {
    const server = quietServer();
    const users = await get<{ users: { id: number }[] }>(server, "/api/v1/users?ids=9,2,404,2");

    const presence = await get<{ presences: { userId: number }[] }>(
      server,
      `/api/v1/presence?ids=${SEED_IDS.bot},4,2`,
    );

    expect(users.users.map((user) => user.id)).toEqual([2, 9]);
    expect(presence.presences.map((entry) => entry.userId)).toEqual([2, 4]);
  });
});

describe("live messages from others", () => {
  it("make an unviewed room unread and update its sidebar row", () => {
    const server = quietServer();
    const events = collect(server);
    const message: MessageDTO = server.post(rooms.engineering, SEED_IDS.users.jonah, "ping @Riel");

    expect(events.map((event) => event.type)).toEqual(["room.unread", "sidebar.row.upserted"]);
    expect(events[0]?.data).toEqual({
      roomId: rooms.engineering,
      messageId: message.id,
      mentioned: true,
    });
    expect(events[1]?.data).toMatchObject({ unreadCount: 1, mentionCount: 1 });
  });

  it("leave a room the viewer is looking at read", () => {
    const server = quietServer();
    const events: SyncEvent[] = [];

    const connection = server.connect((frame) => {
      if (frame.t === "batch") events.push(...frame.events);
    });

    connection.receive({ t: "hello", v: 1, resume: null, topics: [`room:${rooms.engineering}`] });
    connection.receive({ t: "present", room: rooms.engineering });
    server.post(rooms.engineering, SEED_IDS.users.jonah, "hello");

    expect(events.map((event) => event.type)).toEqual(["message.created", "sidebar.row.upserted"]);
    expect(events[1]?.data).toMatchObject({ unreadCount: 0 });
  });
});
