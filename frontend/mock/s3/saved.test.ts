import { describe, expect, it } from "vitest";
import type { ActivityList } from "../../src/gen/ActivityList.ts";
import type { SavedItem } from "../../src/gen/SavedItem.ts";
import type { SavedItemList } from "../../src/gen/SavedItemList.ts";
import { collect, expectStatus, get, harness, NOW, send } from "../s2/testing.ts";
import { SEED_IDS } from "../server.ts";
import { SAVED_PAGE_SIZE } from "./saved.ts";

const { messages, s3 } = SEED_IDS;

describe("the saved list", () => {
  it("pages 50 at a time, newest saved first, with each message, author and conversation", async () => {
    const { server } = harness();
    const first = await get<SavedItemList>(server, "/api/v1/saved");

    expect(first.items).toHaveLength(SAVED_PAGE_SIZE);
    expect(first.nextCursor).not.toBeNull();

    const second = await get<SavedItemList>(server, `/api/v1/saved?before=${first.nextCursor}`);
    const items = [...first.items, ...second.items];

    expect(second.nextCursor).toBeNull();
    expect(new Set(items.map((item) => item.id)).size).toBe(items.length);
    expect(
      items.every(
        (item, index) => index === 0 || (items[index - 1]?.createdAt ?? "") >= item.createdAt,
      ),
    ).toBe(true);

    for (const page of [first, second]) {
      const messageIds = new Set(page.messages.map((message) => message.id));
      const userIds = new Set(page.users.map((user) => user.id));

      const conversations = new Set(
        page.conversations.map((name) => `${name.roomId}:${name.threadId ?? ""}`),
      );

      for (const item of page.items) expect(messageIds).toContain(item.messageId);

      for (const message of page.messages) {
        expect(userIds).toContain(message.creatorId);
        expect(conversations).toContain(`${message.roomId}:${message.threadId ?? ""}`);
      }
    }
  });

  it("filters by status and holds thread and direct-message items with reminders", async () => {
    const { server } = harness();
    const inProgress = await get<SavedItemList>(server, "/api/v1/saved?status=in_progress");
    const done = await get<SavedItemList>(server, "/api/v1/saved?status=done");
    const all = [...inProgress.items, ...done.items];

    expect(inProgress.items.every((item) => item.status === "in_progress")).toBe(true);
    expect(done.items.every((item) => item.status === "done")).toBe(true);
    expect(all.length).toBeGreaterThan(SAVED_PAGE_SIZE);
    expect(all.some((item) => item.remindedAt !== null)).toBe(true);
    expect(all.some((item) => item.remindAt !== null && item.remindedAt === null)).toBe(true);
    expect(
      [...inProgress.messages, ...done.messages].some((message) => message.threadId !== null),
    ).toBe(true);
    expect(inProgress.conversations.some((name) => name.roomKind === "direct")).toBe(true);
  });
});

describe("saved item changes", () => {
  it("marks one done and reopens it, publishing saved.changed", async () => {
    const { server } = harness();
    const events = collect(server);
    const page = await get<SavedItemList>(server, "/api/v1/saved?status=in_progress");
    const item = page.items[0];

    if (item === undefined) throw new Error("no saved items");

    const done = await expectStatus<SavedItem>(
      server,
      "PATCH",
      `/api/v1/saved/${item.id}`,
      { status: "done" },
      200,
    );

    expect(done).toEqual({ ...item, status: "done" });
    expect(events.at(-1)).toMatchObject({
      type: "saved.changed",
      data: { messageId: item.messageId, item: done },
    });

    const reopened = await expectStatus<SavedItem>(
      server,
      "PATCH",
      `/api/v1/saved/${item.id}`,
      { status: "in_progress" },
      200,
    );

    expect(reopened.status).toBe("in_progress");
    expect(
      (await send(server, "PATCH", `/api/v1/saved/${item.id}`, { status: "later" })).status,
    ).toBe(422);
    expect((await send(server, "PATCH", "/api/v1/saved/99999", { status: "done" })).status).toBe(
      404,
    );
  });

  it("sends a due reminder: remindedAt set and a message_reminder in the inbox", async () => {
    const { server, clock } = harness();
    const events = collect(server);

    clock.advance(s3.dueReminderDelayMs);

    const changed = events.find((event) => event.type === "saved.changed");
    const recorded = events.find((event) => event.type === "activity.item");

    expect(changed?.data).toMatchObject({
      messageId: s3.messages.dueReminder,
      item: { remindedAt: new Date(NOW + s3.dueReminderDelayMs).toISOString() },
    });
    expect(recorded?.data).toMatchObject({
      item: {
        eventType: "message_reminder",
        state: "unread",
        source: { sourceType: "saved_item", messageId: s3.messages.dueReminder },
      },
    });
  });

  it("drops the reminder items when the message is unsaved", async () => {
    const { server } = harness();
    const events = collect(server);
    const remindAt = new Date(NOW + 60_000).toISOString();

    const item = await expectStatus<SavedItem>(
      server,
      "POST",
      "/api/v1/saved",
      { messageId: messages.generalChart, remindAt },
      201,
    );

    await send(server, "POST", "/__mock/remind-due", { all: true });

    const reminder = events.flatMap((event) =>
      event.type === "activity.item" && event.data.item.source.sourceId === item.id
        ? [event.data]
        : [],
    )[0];

    expect(reminder).toBeDefined();
    expect((await send(server, "DELETE", `/api/v1/saved/${item.id}`)).status).toBe(204);
    expect(events).toContainEqual(
      expect.objectContaining({
        type: "activity.removed",
        data: expect.objectContaining({ id: reminder?.item.id }),
      }),
    );

    const removed = events.find(
      (event) => event.type === "activity.removed" && event.data.id === reminder?.item.id,
    );

    expect(removed?.type === "activity.removed" ? removed.data.unreadRevision : null).toBe(
      (reminder?.unreadRevision ?? 0) + 1,
    );

    const reminders = await get<ActivityList>(
      server,
      "/api/v1/activity?status=unread&type=reminders",
    );

    expect(reminders.items.map((entry) => entry.id)).not.toContain(reminder?.item.id);
    expect(events.filter((event) => event.type === "saved.changed").at(-1)?.data).toEqual({
      messageId: messages.generalChart,
      item: null,
    });
  });
});
