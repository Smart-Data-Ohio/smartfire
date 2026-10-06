import { describe, expect, it } from "vitest";
import type { MessageDTO } from "../../src/gen/MessageDTO.ts";
import type { MessagePage } from "../../src/gen/MessagePage.ts";
import type { ThreadCreated } from "../../src/gen/ThreadCreated.ts";
import type { ThreadDetail } from "../../src/gen/ThreadDetail.ts";
import type { ThreadList } from "../../src/gen/ThreadList.ts";
import type { ThreadMembershipState } from "../../src/gen/ThreadMembershipState.ts";
import { SEED_IDS } from "../server.ts";
import { collect, errorOf, expectStatus, get, harness, messageBody, send } from "./testing.ts";

const { rooms, users, threads, messages, viewer } = SEED_IDS;

describe("listing and opening threads", () => {
  it("filters a room's threads by state, most recent first", async () => {
    const { server } = harness();
    const path = `/api/v1/rooms/${rooms.general}/threads`;
    const active = await get<ThreadList>(server, path);
    const closed = await get<ThreadList>(server, `${path}?state=closed`);
    const locked = await get<ThreadList>(server, `${path}?state=locked`);

    expect(active.threads.map((row) => row.thread.id)).toEqual([threads.generalActive]);
    expect(active.threads[0]?.membership?.involvement).toBe("everything");
    expect(closed.threads.map((row) => row.thread.id)).toEqual([threads.generalClosed]);
    expect(locked.threads.map((row) => row.thread.id)).toEqual([threads.generalLocked]);
    expect(active.users.map((user) => user.id)).toContain(users.jonah);

    const direct = await get<ThreadList>(server, `/api/v1/rooms/${rooms.dmMaya}/threads`);

    expect(direct.threads).toEqual([]);
  });

  it("opens a thread with its parent and the viewer's permissions", async () => {
    const { server } = harness();
    const detail = await get<ThreadDetail>(server, `/api/v1/threads/${threads.generalLocked}`);

    expect(detail.parentMessage?.id).toBe(messages.generalLockedThreadRoot);
    expect(detail.permissions).toEqual({
      canRename: true,
      canClose: false,
      canReopen: false,
      canLock: false,
      canUnlock: true,
      canDelete: true,
    });
    expect((await server.handle({ method: "GET", path: "/api/v1/threads/99" })).status).toBe(404);
  });

  it("pages a thread's replies", async () => {
    const { server } = harness();

    const page = await get<MessagePage>(
      server,
      `/api/v1/threads/${threads.generalActive}/messages`,
    );

    expect(page.messages).toHaveLength(6);
    expect(page.before).toBeNull();
    expect(page.after).toBeNull();
    expect(page.messages.every((message) => message.threadId === threads.generalActive)).toBe(true);
  });
});

describe("replying", () => {
  it("posts a reply and keeps the indicator current", async () => {
    const { server } = harness();
    const events = collect(server, [`room:${rooms.general}`, `thread:${threads.generalActive}`]);

    const reply = await expectStatus<MessageDTO>(
      server,
      "POST",
      `/api/v1/threads/${threads.generalActive}/messages`,
      messageBody("reply-1", "On it"),
      201,
    );

    expect(reply.threadId).toBe(threads.generalActive);
    expect(events.map((event) => `${event.type}`)).toEqual([
      "message.created",
      "thread.indicator",
      "thread.updated",
      "thread.updated",
    ]);
    expect(events[1]?.data).toMatchObject({
      parentMessageId: messages.generalThreadRoot,
      thread: { replyCount: 7, replierIds: [viewer, users.jonah, users.priya] },
    });

    const again = await send(
      server,
      "POST",
      `/api/v1/threads/${threads.generalActive}/messages`,
      messageBody("reply-1", "On it"),
    );

    expect(again).toEqual({ status: 200, json: reply });

    const root = await get<MessagePage>(
      server,
      `/api/v1/rooms/${rooms.general}/messages?around=${messages.generalThreadRoot}`,
    );

    expect(
      root.messages.find((message) => message.id === messages.generalThreadRoot)?.thread,
    ).toMatchObject({ replyCount: 7, lastReplyAt: reply.createdAt });
  });

  it("refuses a reply in a locked thread", async () => {
    const { server } = harness();

    const response = await send(
      server,
      "POST",
      `/api/v1/threads/${threads.generalLocked}/messages`,
      messageBody("locked-1", "Let me in"),
    );

    expect(response.status).toBe(403);
    expect(errorOf(response.json).tag).toBe("Forbidden");
  });

  it("tells the viewer about someone else's reply in a thread they follow", async () => {
    const { server } = harness();
    const events = collect(server);

    await send(server, "POST", `/api/v1/threads/${threads.generalActive}/read`);
    server.threadPost(threads.generalActive, users.priya, "Numbers are back up");

    expect(events.map((event) => event.type)).toEqual(["thread.read", "thread.unread"]);
    expect(events[1]?.data).toEqual({
      threadId: threads.generalActive,
      roomId: rooms.general,
      refreshOnly: false,
    });

    const detail = await get<ThreadDetail>(server, `/api/v1/threads/${threads.generalActive}`);

    expect(detail.membership?.unreadAt).not.toBeNull();
  });

  it("fans typing out on the thread topic", async () => {
    const { server } = harness();
    const events = collect(server, [`thread:${threads.design}`]);

    await send(server, "POST", "/__mock/thread-typing", {
      threadId: threads.design,
      userId: users.lucia,
    });

    expect(events).toEqual([
      expect.objectContaining({
        topic: `thread:${threads.design}`,
        type: "typing",
        data: { userId: users.lucia, on: true },
      }),
    ]);
  });
});

describe("starting a thread", () => {
  it("creates it with the first reply and publishes thread.created", async () => {
    const { server } = harness();
    const parentId = messages.generalChart;
    const events = collect(server, [`room:${rooms.general}`]);

    const created = await expectStatus<ThreadCreated>(
      server,
      "POST",
      `/api/v1/rooms/${rooms.general}/threads`,
      { parentMessageId: parentId, name: null, message: messageBody("start-1", "Thoughts?") },
      201,
    );

    expect(created.detail.thread).toMatchObject({
      parentMessageId: parentId,
      creatorId: viewer,
      status: "active",
      replyCount: 1,
    });
    expect(created.detail.thread.name).toMatch(/^Weekly signups/);
    expect(created.detail.membership?.involvement).toBe("mentions");
    expect(created.message.threadId).toBe(created.detail.thread.id);
    expect(events.map((event) => event.type)).toEqual(["thread.created", "thread.indicator"]);

    const replay = await send(server, "POST", `/api/v1/rooms/${rooms.general}/threads`, {
      parentMessageId: parentId,
      name: null,
      message: messageBody("start-1", "Thoughts?"),
    });

    expect(replay.status).toBe(200);

    const twice = await send(server, "POST", `/api/v1/rooms/${rooms.general}/threads`, {
      parentMessageId: parentId,
      name: null,
      message: messageBody("start-2", "Again?"),
    });

    expect(twice.status).toBe(409);
  });

  it("refuses a direct room and an unknown parent", async () => {
    const { server } = harness();

    const direct = await send(server, "POST", `/api/v1/rooms/${rooms.dmMaya}/threads`, {
      parentMessageId: 90_000,
      name: null,
      message: messageBody("dm-1", "x"),
    });

    const missing = await send(server, "POST", `/api/v1/rooms/${rooms.general}/threads`, {
      parentMessageId: 1,
      name: null,
      message: messageBody("missing-1", "x"),
    });

    expect(direct.status).toBe(403);
    expect(missing.status).toBe(404);
  });
});

describe("managing a thread", () => {
  it("renames, closes, reopens, locks and unlocks", async () => {
    const { server } = harness();
    const path = `/api/v1/threads/${threads.design}`;
    const events = collect(server, [`room:${rooms.design}`]);

    const renamed = await expectStatus<ThreadDetail>(
      server,
      "PATCH",
      path,
      { name: " Empty states " },
      200,
    );

    expect(renamed.thread.name).toBe("Empty states");
    expect(events.map((event) => event.type)).toEqual(["thread.updated"]);

    const statuses = [];

    for (const status of ["closed", "active", "locked", "active"]) {
      const detail = await expectStatus<ThreadDetail>(server, "PATCH", path, { status }, 200);

      statuses.push(detail.thread.status);
    }

    expect(statuses).toEqual(["closed", "active", "locked", "active"]);
    expect((await send(server, "PATCH", path, { name: "" })).status).toBe(422);
    expect((await send(server, "PATCH", path, { status: "archived" })).status).toBe(422);
  });

  it("joins, leaves and marks read", async () => {
    const { server } = harness();
    const path = `/api/v1/threads/${threads.design}`;

    expect((await send(server, "POST", `${path}/read`)).status).toBe(404);

    const joined = await expectStatus<ThreadMembershipState>(
      server,
      "POST",
      `${path}/join`,
      { involvement: "everything" },
      200,
    );

    expect(joined.membership).toMatchObject({
      threadId: threads.design,
      involvement: "everything",
    });
    expect((await send(server, "POST", `${path}/read`)).status).toBe(200);
    expect((await send(server, "DELETE", `${path}/join`)).status).toBe(204);

    const detail = await get<ThreadDetail>(server, path);

    expect(detail.membership).toBeNull();
  });

  it("deletes a thread for a moderator", async () => {
    const { server } = harness();
    const events = collect(server, [`room:${rooms.design}`]);

    expect((await send(server, "DELETE", `/api/v1/threads/${threads.design}`)).status).toBe(204);
    expect(events.map((event) => event.type)).toEqual(["thread.indicator", "thread.removed"]);
    expect(
      (await server.handle({ method: "GET", path: `/api/v1/threads/${threads.design}` })).status,
    ).toBe(404);
  });
});
