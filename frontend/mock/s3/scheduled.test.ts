import { describe, expect, it } from "vitest";
import type { ActivityList } from "../../src/gen/ActivityList.ts";
import type { MessagePage } from "../../src/gen/MessagePage.ts";
import type { ScheduledMessage } from "../../src/gen/ScheduledMessage.ts";
import type { ScheduledMessageList } from "../../src/gen/ScheduledMessageList.ts";
import { SCHEDULED_PAGE_SIZE } from "../s2/composer.ts";
import { collect, expectStatus, get, harness, NOW, send } from "../s2/testing.ts";
import { type MockServer, SEED_IDS } from "../server.ts";

const { rooms, threads, s3 } = SEED_IDS;

const HOUR = 3_600_000;

function create(
  server: MockServer,
  roomId: number,
  markdownSource: string,
  sendAt: number,
  threadId: number | null = null,
): Promise<ScheduledMessage> {
  return expectStatus<ScheduledMessage>(
    server,
    "POST",
    `/api/v1/rooms/${roomId}/scheduled_messages`,
    { markdownSource, sendAt: new Date(sendAt).toISOString(), threadId, replyToMessageId: null },
    201,
  );
}

describe("the scheduled lists", () => {
  it("changes and clears reply targets, rejects another conversation, and sends the edited target", async () => {
    const { server } = harness();
    const page = await get<MessagePage>(server, `/api/v1/rooms/${rooms.general}/messages`);
    const targets = page.messages.filter((message) => !message.systemNote);
    const [first, second] = targets;

    if (first === undefined || second === undefined) throw new Error("Expected two reply targets");

    const created = await expectStatus<ScheduledMessage>(
      server,
      "POST",
      `/api/v1/rooms/${rooms.general}/scheduled_messages`,
      {
        markdownSource: "Later",
        sendAt: new Date(NOW + HOUR).toISOString(),
        threadId: null,
        replyToMessageId: first.id,
      },
      201,
    );

    const path = `/api/v1/scheduled_messages/${created.id}`;
    expect(created.replyTarget?.messageId).toBe(first.id);

    const changed = await expectStatus<ScheduledMessage>(
      server,
      "PATCH",
      path,
      { replyToMessageId: second.id },
      200,
    );

    expect(changed.replyTarget?.messageId).toBe(second.id);
    const other = await get<MessagePage>(server, `/api/v1/rooms/${rooms.design}/messages`);
    const foreign = other.messages.find((message) => !message.systemNote);

    if (foreign === undefined) throw new Error("Expected a foreign reply target");
    await expectStatus(server, "PATCH", path, { replyToMessageId: foreign.id }, 422);

    const cleared = await expectStatus<ScheduledMessage>(
      server,
      "PATCH",
      path,
      { replyToMessageId: null },
      200,
    );

    expect(cleared.replyToMessageId).toBeNull();
    expect(cleared.replyTarget).toBeNull();
    await expectStatus(server, "PATCH", path, { replyToMessageId: second.id }, 200);
    const sent = await expectStatus<ScheduledMessage>(server, "POST", `${path}/send_now`, {}, 200);

    const posted = await get<MessagePage>(
      server,
      `/api/v1/rooms/${rooms.general}/messages?around=${sent.sentMessageId}`,
    );

    expect(
      posted.messages.find((message) => message.id === sent.sentMessageId)?.replyToMessageId,
    ).toBe(second.id);
  });

  it("lists pending ones soonest first, the stranded one not sendable, with names", async () => {
    const { server } = harness();

    const pending = await get<ScheduledMessageList>(
      server,
      "/api/v1/scheduled_messages?status=pending",
    );

    const times = pending.scheduledMessages.map((message) => message.sendAt);

    const stranded = pending.scheduledMessages.find(
      (message) => message.id === s3.scheduled.stranded,
    );

    expect(times).toEqual([...times].sort());
    expect(stranded).toMatchObject({ state: "pending", sendable: false });
    expect(pending.scheduledMessages.filter((message) => message.sendable)).toHaveLength(
      pending.scheduledMessages.length - 1,
    );
    expect(pending.conversations).toContainEqual(
      expect.objectContaining({ roomId: s3.rooms.offsite, roomName: "q3-offsite" }),
    );
    expect(pending.conversations).toContainEqual(
      expect.objectContaining({ threadId: threads.design, threadName: "Onboarding empty states" }),
    );
  });

  it("lists past ones most recent first: sent with their message, dropped with a reason or none", async () => {
    const { server } = harness();
    const past = await get<ScheduledMessageList>(server, "/api/v1/scheduled_messages?status=past");
    const byId = new Map(past.scheduledMessages.map((message) => [message.id, message]));
    const times = past.scheduledMessages.map((message) => message.sendAt);

    expect(times).toEqual([...times].sort().reverse());
    expect(byId.get(s3.scheduled.dmMayaSent)).toMatchObject({
      state: "sent",
      sentMessageId: expect.any(Number),
    });
    expect(byId.get(s3.scheduled.droppedThread)).toMatchObject({
      state: "dropped",
      dropReason: "its thread was deleted",
    });
    expect(byId.get(s3.scheduled.droppedAccess)).toMatchObject({
      state: "dropped",
      dropReason: null,
    });
  });

  it("pages 50 at a time", async () => {
    const { server } = harness();

    for (let index = 0; index < SCHEDULED_PAGE_SIZE + 5; index++) {
      await create(server, rooms.quiet, `Later ${index}`, NOW + (index + 1) * HOUR);
    }

    const path = `/api/v1/scheduled_messages?status=pending&roomId=${rooms.quiet}`;
    const first = await get<ScheduledMessageList>(server, path);
    const second = await get<ScheduledMessageList>(server, `${path}&before=${first.nextCursor}`);

    expect(first.scheduledMessages).toHaveLength(SCHEDULED_PAGE_SIZE);
    expect(second.scheduledMessages.map((message) => message.markdownSource)).toEqual([
      "Later 50",
      "Later 51",
      "Later 52",
      "Later 53",
      "Later 54",
    ]);
    expect(second.nextCursor).toBeNull();
  });
});

describe("scheduled message events and states", () => {
  it("publishes scheduled.changed on create, edit and send, and scheduled.removed on cancel", async () => {
    const { server } = harness();
    const events = collect(server);
    const created = await create(server, rooms.quiet, "Soon", NOW + HOUR);

    await expectStatus(
      server,
      "PATCH",
      `/api/v1/scheduled_messages/${created.id}`,
      { sendAt: new Date(NOW + 2 * HOUR).toISOString() },
      200,
    );

    const other = await create(server, rooms.quiet, "Now", NOW + 3 * HOUR);

    await expectStatus(
      server,
      "POST",
      `/api/v1/scheduled_messages/${other.id}/send_now`,
      null,
      200,
    );
    await send(server, "DELETE", `/api/v1/scheduled_messages/${created.id}`);

    const kinds = events.flatMap((event) => {
      if (event.type === "scheduled.changed")
        return [`changed:${event.data.id}:${event.data.state}`];

      if (event.type === "scheduled.removed") return [`removed:${event.data.id}`];

      return [];
    });

    expect(kinds).toEqual([
      `changed:${created.id}:pending`,
      `changed:${created.id}:pending`,
      `changed:${other.id}:pending`,
      `changed:${other.id}:sent`,
      `removed:${created.id}`,
    ]);
  });

  it("answers 409 to editing or cancelling one that's sending, and 202 to sending it now", async () => {
    const { server } = harness();
    const created = await create(server, rooms.quiet, "Busy", NOW + HOUR);
    const path = `/api/v1/scheduled_messages/${created.id}`;

    await send(server, "POST", "/__mock/schedule-sending", { id: created.id });

    expect((await send(server, "PATCH", path, { markdownSource: "Edited" })).status).toBe(409);
    expect((await send(server, "DELETE", path)).status).toBe(409);

    const held = await expectStatus<ScheduledMessage>(
      server,
      "POST",
      `${path}/send_now`,
      null,
      202,
    );

    expect(held.state).toBe("sending");
  });

  it("holds one for a locked thread (202) and keeps it scheduled", async () => {
    const { server } = harness();

    const created = await create(
      server,
      rooms.general,
      "In the locked thread",
      NOW + HOUR,
      threads.generalLocked,
    );

    const held = await expectStatus<ScheduledMessage>(
      server,
      "POST",
      `/api/v1/scheduled_messages/${created.id}/send_now`,
      null,
      202,
    );

    expect(held).toMatchObject({ state: "pending", sentAt: null, droppedAt: null });
  });

  it("drops the stranded one when it falls due, with an item in the inbox", async () => {
    const { server } = harness();
    const events = collect(server);

    await send(server, "POST", "/__mock/schedule-due", { all: true });

    const dropped = events.find(
      (event) => event.type === "scheduled.changed" && event.data.id === s3.scheduled.stranded,
    );

    const inbox = await get<ActivityList>(server, "/api/v1/activity?status=unread");

    expect(dropped?.data).toMatchObject({ state: "dropped", dropReason: null, sendable: false });
    expect(inbox.items).toContainEqual(
      expect.objectContaining({
        eventType: "scheduled_message_dropped",
        source: expect.objectContaining({
          sourceType: "scheduled_message",
          sourceId: s3.scheduled.stranded,
        }),
      }),
    );
  });

  it("answers 422 to sending one now that has to be dropped", async () => {
    const { server } = harness();

    const response = await send(
      server,
      "POST",
      `/api/v1/scheduled_messages/${s3.scheduled.stranded}/send_now`,
    );

    expect(response.status).toBe(422);
  });
});
