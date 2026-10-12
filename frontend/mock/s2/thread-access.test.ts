import { describe, expect, it } from "vitest";
import type { MessagePage } from "../../src/gen/MessagePage.ts";
import type { ScheduledMessage } from "../../src/gen/ScheduledMessage.ts";
import type { ScheduledMessageList } from "../../src/gen/ScheduledMessageList.ts";
import type { SyncEvent } from "../../src/gen/SyncEvent.ts";
import type { ThreadAccessMembers } from "../../src/gen/ThreadAccessMembers.ts";
import type { ThreadCreated } from "../../src/gen/ThreadCreated.ts";
import type { ThreadList } from "../../src/gen/ThreadList.ts";
import { field, stringField } from "../json.ts";
import { SEED_IDS } from "../server.ts";
import { collect, expectStatus, get, harness, messageBody, NOW, send } from "./testing.ts";

const { rooms, messages, users, viewer } = SEED_IDS;

async function privateThread(roomId: number = rooms.general) {
  const { server } = harness();
  await send(server, "POST", "/__mock/viewer-role", { role: "member" });
  const page = await get<MessagePage>(server, `/api/v1/rooms/${roomId}/messages`);

  const parentId =
    roomId === rooms.general
      ? messages.generalChart
      : page.messages.find((message) => message.thread === null)?.id;

  if (parentId === undefined) throw new Error("A private thread needs a root message");

  const created = await expectStatus<ThreadCreated>(
    server,
    "POST",
    `/api/v1/rooms/${roomId}/threads`,
    {
      parentMessageId: parentId,
      name: "Private plans",
      visibility: "private",
      message: messageBody("private-start", "Private discussion"),
    },
    201,
  );

  return { server, created, path: `/api/v1/threads/${created.detail.thread.id}` };
}

function reason(json: Parameters<typeof field>[0]): string | null {
  return stringField(field(json, "error"), "reason");
}

describe("private-thread access", () => {
  it("grants streamed mentions at finalization before notifying the newly invited viewer", async () => {
    const { server, created, path } = await privateThread(rooms.engineering);
    await expectStatus(server, "POST", `${path}/members`, { userId: users.ember }, 200);
    await expectStatus(server, "DELETE", `${path}/members/${viewer}`, null, 204);
    const events = collect(server);
    const stream = server.threadPost(created.detail.thread.id, users.ember, "Draft", true);
    server.updateMessage(stream.id, `Draft <@${viewer}>`, true);
    expect((await server.handle({ method: "GET", path })).status).toBe(404);
    expect(events.some((event) => event.type === "thread.unread")).toBe(false);
    server.updateMessage(stream.id, `Draft <@${viewer}>`, false);
    const detail = await get<ThreadCreated["detail"]>(server, path);
    expect(detail.membership?.unreadAt).not.toBeNull();
    expect(
      events.some(
        (event) =>
          event.type === "thread.unread" && event.data.threadId === created.detail.thread.id,
      ),
    ).toBe(true);
    const members = await get<ThreadAccessMembers>(server, `${path}/members`);
    expect(members.members.find((member) => member.userId === viewer)?.addedById).toBe(users.ember);
    const count = events.filter((event) => event.type === "thread.unread").length;
    server.updateMessage(stream.id, `Draft <@${viewer}>`, false);
    expect(events.filter((event) => event.type === "thread.unread")).toHaveLength(count);
  });

  it("re-renders scheduled replay against revoked access and preserves the author's draft", async () => {
    const { server, created, path } = await privateThread();
    const live = collect(server, [`room:${rooms.general}`]);
    const resume = server.syncState();
    server.pin(created.message.id, viewer);

    const draft = await expectStatus<ScheduledMessage>(
      server,
      "POST",
      `/api/v1/rooms/${rooms.general}/scheduled_messages`,
      {
        markdownSource: "My retained draft",
        sendAt: new Date(NOW + 3_600_000).toISOString(),
        threadId: created.detail.thread.id,
        replyToMessageId: created.message.id,
      },
      201,
    );

    expect(draft.replyTarget?.excerpt).toBe("Private discussion");
    expect(
      live.some((event) => event.type === "scheduled.changed" && event.data.replyTarget !== null),
    ).toBe(true);
    expect((await send(server, "DELETE", `${path}/members/${viewer}`)).status).toBe(204);
    const replayed: SyncEvent[] = [];

    const connection = server.connect((frame) => {
      if (frame.t === "batch") replayed.push(...frame.events);
    });

    connection.receive({ t: "hello", v: 1, topics: [`room:${rooms.general}`], resume });

    const event = replayed.find(
      (event) => event.type === "scheduled.changed" && event.data.id === draft.id,
    );

    if (event?.type !== "scheduled.changed") throw new Error("Expected retained scheduled draft");
    expect(event.data.replyTarget).toBeNull();
    expect(event.data.sendable).toBe(false);
    expect(event.data.markdownSource).toBe("My retained draft");
    expect(replayed.some((event) => event.type === "message.pinned")).toBe(false);
    const list = await get<ScheduledMessageList>(server, "/api/v1/scheduled_messages");

    expect(list.scheduledMessages.find((message) => message.id === draft.id)).toEqual(event.data);
    connection.close();
  });

  it("defaults old create requests to public and refuses visibility edits atomically", async () => {
    const { server } = harness();

    const created = await expectStatus<ThreadCreated>(
      server,
      "POST",
      `/api/v1/rooms/${rooms.general}/threads`,
      {
        parentMessageId: messages.generalChart,
        name: null,
        message: messageBody("public-start", "Discussion"),
      },
      201,
    );

    expect(created.detail.thread.visibility).toBe("public");
    const path = `/api/v1/threads/${created.detail.thread.id}`;
    const response = await send(server, "PATCH", path, { name: "Changed", visibility: "private" });
    expect(response.status).toBe(422);
    expect(reason(response.json)).toBe("visibilityImmutable");
    expect((await get<ThreadCreated["detail"]>(server, path)).thread.name).toBe(
      created.detail.thread.name,
    );
    expect(
      reason((await send(server, "POST", `${path}/members`, { userId: users.maya })).json),
    ).toBe("notPrivate");
  });

  it("grants creator access, keeps follows separate and makes invites idempotent", async () => {
    const { server, created, path } = await privateThread();
    expect(created.detail.thread.visibility).toBe("private");
    expect(
      (await get<ThreadAccessMembers>(server, `${path}/members`)).members.map(
        (member) => member.userId,
      ),
    ).toEqual([viewer]);
    await send(server, "DELETE", `${path}/join`);
    expect((await server.handle({ method: "GET", path })).status).toBe(200);

    const first = await expectStatus<ThreadAccessMembers>(
      server,
      "POST",
      `${path}/members`,
      { userId: users.maya },
      200,
    );

    const repeated = await expectStatus<ThreadAccessMembers>(
      server,
      "POST",
      `${path}/members`,
      { userId: users.maya },
      200,
    );

    expect(repeated).toEqual(first);
    expect(repeated.members.find((member) => member.userId === users.maya)?.addedById).toBe(viewer);
    const response = await send(server, "POST", `${path}/members`, { userId: 999999 });
    expect(reason(response.json)).toBe("parentRoomRequired");

    const mention = await send(
      server,
      "POST",
      `${path}/messages`,
      messageBody("mention", `<@${users.priya}> can you look?`),
    );

    expect(mention.status).toBe(201);
    const granted = await get<ThreadAccessMembers>(server, `${path}/members`);
    expect(granted.members.map((member) => member.userId)).toContain(users.priya);
    expect(granted.members.find((member) => member.userId === users.priya)?.addedById).toBe(viewer);
    expect(granted.members.filter((member) => member.userId !== users.priya)).toEqual(
      first.members,
    );
  });

  it("invites active parent-room mentionees, including agents, on posts and edits", async () => {
    const { server, created, path } = await privateThread(rooms.engineering);

    const posted = await send(
      server,
      "POST",
      `${path}/messages`,
      messageBody(
        "parent-mentions",
        `<@${users.priya}> <@${users.ember}> <@${users.maya}> <@${users.dana}>`,
      ),
    );

    expect(posted.status).toBe(201);
    const grants = await get<ThreadAccessMembers>(server, `${path}/members`);
    expect(grants.members.map((member) => member.userId)).toEqual([
      viewer,
      users.priya,
      users.ember,
    ]);
    expect(
      (
        await send(server, "PATCH", `/api/v1/messages/${created.message.id}`, {
          markdownSource: `<@${users.jonah}> <@${users.maya}> edited mention`,
        })
      ).status,
    ).toBe(200);
    const edited = await get<ThreadAccessMembers>(server, `${path}/members`);
    expect(edited.members.map((member) => member.userId)).toEqual([
      viewer,
      users.jonah,
      users.priya,
      users.ember,
    ]);
    expect(edited.members.find((member) => member.userId === users.priya)).toEqual(
      grants.members.find((member) => member.userId === users.priya),
    );
    expect(() =>
      server.threadPost(created.detail.thread.id, users.priya, "Invited before delivery"),
    ).not.toThrow();
  });

  it("invites forward-note mentionees while leaving snapshot mentions without grants", async () => {
    const { server, created, path } = await privateThread(rooms.engineering);
    const original = server.post(rooms.general, viewer, `<@${users.jonah}> source mention`);
    const destinations = [{ roomId: rooms.engineering, threadId: created.detail.thread.id }];
    expect(
      (
        await send(server, "POST", `/api/v1/messages/${original.id}/forwards`, {
          destinations,
          note: null,
        })
      ).status,
    ).toBe(201);
    expect(
      (await get<ThreadAccessMembers>(server, `${path}/members`)).members.map(
        (member) => member.userId,
      ),
    ).toEqual([viewer]);
    expect(
      (
        await send(server, "POST", `/api/v1/messages/${original.id}/forwards`, {
          destinations,
          note: "@[Priya Raman] @[Maya Okafor] please look",
        })
      ).status,
    ).toBe(201);
    const grants = await get<ThreadAccessMembers>(server, `${path}/members`);
    expect(grants.members.map((member) => member.userId)).toEqual([viewer, users.priya]);
  });

  it("revokes creator reads, message lookup, lists, previews and live or replayed events after leaving", async () => {
    const { server, created, path } = await privateThread();
    const threadId = created.detail.thread.id;
    const events = collect(server, [`room:${rooms.general}`, `thread:${threadId}`]);
    await send(server, "POST", `${path}/members`, { userId: users.maya });
    const resume = server.syncState();
    expect((await send(server, "DELETE", `${path}/members/${viewer}`)).status).toBe(204);
    expect(events.some((event) => event.type === "thread.removed")).toBe(true);
    const eventCount = events.length;
    server.threadPost(threadId, users.maya, "Still private");
    expect(events).toHaveLength(eventCount);

    for (const suffix of ["", "/messages", "/members"]) {
      expect((await server.handle({ method: "GET", path: `${path}${suffix}` })).status).toBe(404);
    }

    expect((await send(server, "POST", `${path}/join`, { involvement: "everything" })).status).toBe(
      404,
    );
    expect(
      (await server.handle({ method: "GET", path: `/api/v1/messages/${created.message.id}` }))
        .status,
    ).toBe(404);

    const listed = await get<ThreadList>(
      server,
      `/api/v1/rooms/${rooms.general}/threads?state=all`,
    );

    expect(listed.threads.some((row) => row.thread.id === threadId)).toBe(false);

    const page = await get<MessagePage>(
      server,
      `/api/v1/rooms/${rooms.general}/messages?around=${messages.generalChart}`,
    );

    expect(
      page.messages.find((message) => message.id === messages.generalChart)?.thread,
    ).toBeNull();
    const replayed: string[] = [];

    const connection = server.connect((frame) => {
      if (frame.t === "batch") replayed.push(...frame.events.map((event) => event.type));
    });

    connection.receive({
      t: "hello",
      v: 1,
      topics: [`thread:${threadId}`],
      resume: { epoch: resume.epoch, seq: resume.seq },
    });
    expect(replayed).not.toContain("message.created");
    await send(server, "POST", "/__mock/viewer-role", { role: "administrator" });
    expect((await server.handle({ method: "GET", path })).status).toBe(200);
  });
});

it("hides a private child when creating another thread from its public parent", async () => {
  const { server, created, path } = await privateThread();
  await expectStatus(server, "DELETE", `${path}/members/${viewer}`, null, 204);

  const body = {
    parentMessageId: created.detail.thread.parentMessageId,
    message: messageBody("hidden-child-collision", "Fresh discussion"),
  };

  const hidden = await send(server, "POST", `/api/v1/rooms/${rooms.general}/threads`, body);
  expect(hidden.status).toBe(404);
  await send(server, "POST", "/__mock/viewer-role", { role: "administrator" });
  const visible = await send(server, "POST", `/api/v1/rooms/${rooms.general}/threads`, body);
  expect(visible.status).toBe(409);
});

it("keeps creator invitations after leaving without revealing the roster or content", async () => {
  const { server, path } = await privateThread();
  await expectStatus(server, "DELETE", `${path}/members/${viewer}`, null, 204);

  const added = await expectStatus<ThreadAccessMembers>(
    server,
    "POST",
    `${path}/members`,
    { userId: users.maya },
    200,
  );

  expect(added.members.map((member) => member.userId)).toEqual([users.maya]);
  expect((await server.handle({ method: "GET", path: `${path}/members` })).status).toBe(404);
  expect((await server.handle({ method: "GET", path })).status).toBe(404);
  await expectStatus(server, "DELETE", `${path}/members/${users.maya}`, null, 204);
  await expectStatus(server, "POST", `${path}/members`, { userId: viewer }, 200);

  expect((await server.handle({ method: "GET", path })).status).toBe(200);
});

it("lets the controls start a private thread as someone else, share it and take it back", async () => {
  const { server } = harness();
  await send(server, "POST", "/__mock/viewer-role", { role: "member" });
  const events = collect(server, [`room:${rooms.general}`]);

  const hidden = await expectStatus<{ threadId: number }>(
    server,
    "POST",
    "/__mock/private-thread",
    { roomId: rooms.general, creatorId: users.maya, name: "Maya's plans" },
    201,
  );

  const shared = await expectStatus<{ threadId: number }>(
    server,
    "POST",
    "/__mock/private-thread",
    { roomId: rooms.general, creatorId: users.maya, name: "With Riel", withViewer: true },
    201,
  );

  const readable = async (threadId: number, userId: number) =>
    field(
      (await send(server, "POST", "/__mock/thread-readable", { threadId, userId })).json,
      "readable",
    );

  expect(await readable(hidden.threadId, users.maya)).toBe(true);
  expect(await readable(hidden.threadId, viewer)).toBe(false);
  expect(await readable(shared.threadId, viewer)).toBe(true);

  // The viewer only hears about the one shared with them.
  const created = events.filter((event) => event.type === "thread.created");

  expect(created.map((event) => event.type === "thread.created" && event.data.id)).toEqual([
    shared.threadId,
  ]);

  const list = await get<ThreadList>(server, `/api/v1/rooms/${rooms.general}/threads?filter=all`);

  expect(list.threads.map(({ thread }) => thread.id)).toContain(shared.threadId);
  expect(list.threads.map(({ thread }) => thread.id)).not.toContain(hidden.threadId);

  const userEvents = collect(server, ["user"]);

  await expectStatus(
    server,
    "POST",
    "/__mock/thread-access",
    { threadId: shared.threadId, userId: viewer, added: false },
    204,
  );

  expect(userEvents.some((event) => event.type === "thread.removed")).toBe(true);
  expect(await readable(shared.threadId, viewer)).toBe(false);

  await expectStatus(
    server,
    "POST",
    "/__mock/thread-access",
    { threadId: hidden.threadId, userId: viewer, added: true },
    200,
  );

  expect(await readable(hidden.threadId, viewer)).toBe(true);
});

it("tells the viewer of a grant on their own topic, and drops the thread's topic with the access", async () => {
  const { server } = harness();
  await send(server, "POST", "/__mock/viewer-role", { role: "member" });

  const { threadId } = await expectStatus<{ threadId: number }>(
    server,
    "POST",
    "/__mock/private-thread",
    { roomId: rooms.general, creatorId: users.maya, name: "With Riel", withViewer: true },
    201,
  );

  const heard: SyncEvent[] = [];

  const connection = server.connect((frame) => {
    if (frame.t === "batch") heard.push(...frame.events);
  });

  connection.receive({ t: "hello", v: 1, topics: [`thread:${threadId}`], resume: null });

  const access = (added: boolean) =>
    send(server, "POST", "/__mock/thread-access", { threadId, userId: viewer, added });

  await access(false);

  expect(heard.find((event) => event.type === "thread.removed")?.topic).toBe("user");

  heard.length = 0;
  await access(true);

  // Added back: the record arrives on the viewer's own topic, the only one still followed.
  expect(
    heard.some(
      (event) =>
        event.type === "thread.updated" && event.topic === "user" && event.data.id === threadId,
    ),
  ).toBe(true);

  heard.length = 0;
  server.threadPost(threadId, users.maya, "Welcome back");
  expect(heard.some((event) => event.type === "message.created")).toBe(false);

  connection.receive({ t: "sub", topics: [`thread:${threadId}`] });
  server.threadPost(threadId, users.maya, "Now you hear it");
  expect(heard.some((event) => event.type === "message.created")).toBe(true);
  connection.close();
});
