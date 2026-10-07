import { afterEach, describe, expect, it } from "@effect/vitest";
import { Deferred, Effect, Fiber } from "effect";
import { TestClock } from "effect/testing";
import { Conflict, Forbidden, NotFound, ServerError, Validation } from "../api/errors.ts";
import { FakeApi, messageFixture, userFixture } from "../api/testing.ts";
import type { ActivityEventType } from "../gen/ActivityEventType.ts";
import type { ActivityItem } from "../gen/ActivityItem.ts";
import type { ActivityState } from "../gen/ActivityState.ts";
import type { SavedItem } from "../gen/SavedItem.ts";
import type { ScheduledMessage } from "../gen/ScheduledMessage.ts";
import { activityListOf } from "../store/activity.ts";
import { savedItemForMessage, savedListOf } from "../store/saved-list.ts";
import { roomScheduledKey, scheduledListOf } from "../store/scheduled.ts";
import { mutations, store } from "../store/store.ts";
import * as activity from "./activity-actions.ts";
import * as saved from "./saved-actions.ts";
import * as scheduled from "./scheduled-actions.ts";

const ROOM = 12;

/** A time on the test day, `minute` minutes past 09:00. */
function at(minute: number): string {
  return new Date(Date.UTC(2026, 9, 6, 9, minute, 0)).toISOString();
}

function item(
  id: number,
  minute: number,
  state: ActivityState = "unread",
  eventType: ActivityEventType = "mention",
): ActivityItem {
  return {
    id,
    eventType,
    state,
    readAt: state === "unread" ? null : at(minute),
    handledAt: state === "handled" ? at(minute) : null,
    createdAt: at(minute),
    updatedAt: at(minute),
    source: {
      sourceType: "message",
      sourceId: id * 10,
      roomId: ROOM,
      threadId: null,
      messageId: id * 10,
      eventId: null,
      creatorId: 2,
      title: "general",
      body: `Item ${id}`,
      occurredAt: at(minute),
      approvalStatus: null,
      budgetCap: null,
      path: `/rooms/${ROOM}/@${id * 10}`,
    },
  };
}

function savedItem(id: number, messageId: number, minute: number, done = false): SavedItem {
  return {
    id,
    messageId,
    status: done ? "done" : "in_progress",
    remindAt: null,
    remindedAt: null,
    createdAt: at(minute),
  };
}

function scheduledMessage(
  id: number,
  minute: number,
  extra: Partial<ScheduledMessage> = {},
): ScheduledMessage {
  return {
    id,
    roomId: ROOM,
    threadId: null,
    replyToMessageId: null,
    markdownSource: `Later ${id}`,
    sendAt: at(minute),
    state: "pending",
    sendable: true,
    sentAt: null,
    sentMessageId: null,
    droppedAt: null,
    dropReason: null,
    createdAt: at(0),
    ...extra,
  };
}

/** Unread 3 and 2, Handled 1, in All; the badge at 5. */
function seedInbox(): void {
  mutations.reset();
  mutations.landActivityPage(
    "all",
    "unread",
    {
      items: [item(3, 30), item(2, 20)],
      users: [userFixture(2)],
      unreadCount: 5,
      nextCursor: null,
    },
    "replace",
  );
  mutations.landActivityPage(
    "all",
    "handled",
    { items: [item(1, 10, "handled")], users: [], unreadCount: 5, nextCursor: null },
    "replace",
  );
}

/** Saved items 2 (newest) and 1, in progress, in All and In progress; Done loaded empty. */
function seedSaved(): void {
  const items = [savedItem(2, 102, 20), savedItem(1, 101, 10)];

  const page = {
    items,
    messages: [messageFixture(102, ROOM), messageFixture(101, ROOM)],
    users: [userFixture(2)],
    conversations: [],
    nextCursor: null,
  };

  mutations.reset();
  mutations.landSavedPage("all", page, "replace");
  mutations.landSavedPage("in_progress", page, "replace");
  mutations.landSavedPage("done", { ...page, items: [], messages: [] }, "replace");
}

/** Pending 1 and 2 in Pending and the room's list; Past holds 9. */
function seedScheduled(): void {
  const pending = {
    scheduledMessages: [scheduledMessage(1, 10), scheduledMessage(2, 20)],
    conversations: [],
    nextCursor: null,
  };

  mutations.reset();
  mutations.landScheduledPage("pending", pending, "replace");
  mutations.landScheduledPage(roomScheduledKey(ROOM), pending, "replace");
  mutations.landScheduledPage(
    "past",
    {
      scheduledMessages: [
        scheduledMessage(9, 5, {
          state: "sent",
          sendable: false,
          sentAt: at(5),
          sentMessageId: 900,
        }),
      ],
      conversations: [],
      nextCursor: null,
    },
    "replace",
  );
}

const refuse = () => Effect.fail(new Forbidden({ message: "Not allowed" }));

function inboxIds(status: ActivityState): readonly number[] {
  return activityListOf(store.getState(), "all", status).ids;
}

describe("activity actions", () => {
  afterEach(() => mutations.reset());

  it.effect("move a handled item at once, then take the server's item and count", () =>
    Effect.gen(function* () {
      seedInbox();
      yield* TestClock.setTime(Date.UTC(2026, 9, 6, 10, 0, 0));

      const fake = yield* FakeApi;
      let during: readonly (readonly number[] | number | null)[] = [];

      yield* fake.route("PATCH /activity/3", (request) => {
        during = [inboxIds("unread"), inboxIds("handled"), store.getState().activity.unreadCount];

        expect(request.body).toEqual({ action: "handled" });

        return Effect.succeed({ item: item(3, 30, "handled"), unreadCount: 9 });
      });

      const answered = yield* activity.setState(3, "handled");

      expect(during).toEqual([[2], [3, 1], 4]);
      expect(answered.state).toBe("handled");
      expect(inboxIds("handled")).toEqual([3, 1]);
      expect(store.getState().activity.unreadCount).toBe(9);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("put the item and the badge back when the server refuses", () =>
    Effect.gen(function* () {
      seedInbox();

      const fake = yield* FakeApi;

      yield* fake.route("PATCH /activity/2", refuse);

      const failure = yield* Effect.flip(activity.setState(2, "read"));

      expect(failure.message).toBe("Not allowed");
      expect(inboxIds("unread")).toEqual([3, 2]);
      expect(store.getState().activity.items[2]?.state).toBe("unread");
      expect(store.getState().activity.unreadCount).toBe(5);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("open an item through its open route, marking it read", () =>
    Effect.gen(function* () {
      seedInbox();

      const fake = yield* FakeApi;

      yield* fake.reply("POST /activity/3/open", { item: item(3, 30, "read"), unreadCount: 4 });

      const opened = yield* activity.open(3);

      expect(opened.state).toBe("read");
      expect(inboxIds("unread")).toEqual([2]);
      expect(store.getState().activity.unreadCount).toBe(4);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("load a tab's pages with the cursor, stop at the last, and land a failure", () =>
    Effect.gen(function* () {
      mutations.reset();

      const fake = yield* FakeApi;

      yield* fake.route("GET /activity", (request) =>
        Effect.succeed(
          request.query?.before === "page-2"
            ? { items: [item(1, 10)], users: [], unreadCount: 2, nextCursor: null }
            : { items: [item(2, 20)], users: [], unreadCount: 2, nextCursor: "page-2" },
        ),
      );

      yield* activity.load("mentions", "unread");
      yield* activity.loadMore("mentions", "unread");
      yield* activity.loadMore("mentions", "unread");

      const queries = (yield* fake.requests).map((request) => request.query);

      expect(activityListOf(store.getState(), "mentions", "unread").ids).toEqual([2, 1]);
      expect(queries).toEqual([
        { status: "unread", type: "mentions" },
        { status: "unread", type: "mentions", before: "page-2" },
      ]);

      yield* fake.route("GET /activity", () =>
        Effect.fail(new ServerError({ status: 500, message: "Down" })),
      );
      yield* activity.load("huddles", "read");

      expect(activityListOf(store.getState(), "huddles", "read")).toMatchObject({
        status: "error",
        error: "Down",
      });
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("roll two failing changes back to the server's count, not to a snapshot", () =>
    Effect.gen(function* () {
      seedInbox();

      const fake = yield* FakeApi;
      const gate = yield* Deferred.make<void>();
      let during: number | null = null;

      for (const id of [3, 2]) {
        yield* fake.route(`PATCH /activity/${id}`, () => {
          during = store.getState().activity.unreadCount;

          return Deferred.await(gate).pipe(Effect.andThen(refuse));
        });
      }

      const both = yield* Effect.forkChild(
        Effect.all([activity.setState(3, "read"), activity.setState(2, "read")], {
          concurrency: "unbounded",
          mode: "result",
        }),
      );

      yield* Effect.yieldNow;
      yield* Effect.yieldNow;

      expect(during).toBe(3);

      yield* Deferred.succeed(gate, undefined);
      yield* Fiber.await(both);

      expect(store.getState().activity.unreadCount).toBe(5);
      expect(inboxIds("unread")).toEqual([3, 2]);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("keep a newer event when a refused change ends", () =>
    Effect.gen(function* () {
      seedInbox();

      const fake = yield* FakeApi;

      yield* fake.route("PATCH /activity/2", () => {
        // Someone handled it elsewhere meanwhile.
        mutations.applyActivityItem(item(2, 50, "handled"), 3);

        return refuse();
      });

      yield* Effect.flip(activity.setState(2, "read"));

      expect(store.getState().activity.items[2]?.state).toBe("handled");
      expect(inboxIds("handled")).toEqual([2, 1]);
      expect(store.getState().activity.unreadCount).toBe(3);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("ignore an older page's copy, and its count once a newer count landed", () =>
    Effect.gen(function* () {
      seedInbox();

      const fake = yield* FakeApi;

      yield* fake.route("GET /activity", () => {
        mutations.applyActivityItem(item(3, 50, "read"), 4);

        return Effect.succeed({
          items: [item(3, 30), item(2, 20)],
          users: [],
          unreadCount: 5,
          nextCursor: null,
        });
      });

      yield* activity.load("all", "unread");

      expect(store.getState().activity.items[3]?.state).toBe("read");
      expect(inboxIds("unread")).toEqual([2]);
      expect(store.getState().activity.unreadCount).toBe(4);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("drop a next page that lands after the list reloaded", () =>
    Effect.gen(function* () {
      mutations.reset();

      const fake = yield* FakeApi;

      yield* fake.reply("GET /activity", {
        items: [item(3, 30)],
        users: [],
        unreadCount: 2,
        nextCursor: "page-2",
      });
      yield* activity.load("all", "unread");

      yield* fake.route("GET /activity", () => {
        // A reload starts while this next page is on its way.
        mutations.setActivityListLoading("all", "unread", false);
        mutations.landActivityPage(
          "all",
          "unread",
          { items: [item(4, 40)], users: [], unreadCount: 2, nextCursor: null },
          "replace",
        );

        return Effect.succeed({
          items: [item(1, 10)],
          users: [],
          unreadCount: 2,
          nextCursor: "page-3",
        });
      });
      yield* activity.loadMore("all", "unread");

      expect(inboxIds("unread")).toEqual([4]);
      expect(activityListOf(store.getState(), "all", "unread").nextCursor).toBeNull();
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("refresh the badge", () =>
    Effect.gen(function* () {
      seedInbox();

      const fake = yield* FakeApi;

      yield* fake.reply("GET /activity/unread_count", { unreadCount: 12 });

      expect(yield* activity.loadUnreadCount()).toBe(12);
      expect(store.getState().activity.unreadCount).toBe(12);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );
});

describe("saved actions", () => {
  afterEach(() => mutations.reset());

  it.effect("move an item marked done at once, and back when the server refuses", () =>
    Effect.gen(function* () {
      seedSaved();

      const fake = yield* FakeApi;
      let during: readonly number[] = [];

      yield* fake.route("PATCH /saved/2", () => {
        during = savedListOf(store.getState(), "done").ids;

        return Effect.succeed(savedItem(2, 102, 20, true));
      });

      yield* saved.setStatus(2, "done");

      expect(during).toEqual([2]);
      expect(savedListOf(store.getState(), "in_progress").ids).toEqual([1]);

      yield* fake.route("PATCH /saved/1", refuse);
      yield* Effect.flip(saved.setStatus(1, "done"));

      expect(savedListOf(store.getState(), "in_progress").ids).toEqual([1]);
      expect(savedListOf(store.getState(), "done").ids).toEqual([2]);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("remove an item and its mark at once, and bring both back when refused", () =>
    Effect.gen(function* () {
      seedSaved();

      const fake = yield* FakeApi;
      let markDuring: number | undefined = -1;

      yield* fake.route("DELETE /saved/2", () => {
        markDuring = store.getState().saved[102];

        return refuse();
      });

      yield* Effect.flip(saved.remove(2));

      expect(markDuring).toBeUndefined();
      expect(store.getState().saved[102]).toBe(2);
      expect(savedListOf(store.getState(), "all").ids).toEqual([2, 1]);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("keep an event's copy when a refused change ends, and reload on 404", () =>
    Effect.gen(function* () {
      seedSaved();

      const fake = yield* FakeApi;

      yield* fake.route("PATCH /saved/2", () => {
        mutations.applySavedChange(102, { ...savedItem(2, 102, 20), remindAt: at(59) });

        return refuse();
      });
      yield* Effect.flip(saved.setStatus(2, "done"));

      expect(store.getState().savedList.items[2]?.remindAt).toBe(at(59));
      expect(savedListOf(store.getState(), "in_progress").ids).toEqual([2, 1]);

      yield* fake.route("DELETE /saved/1", () => Effect.fail(new NotFound({ message: "Gone" })));
      yield* Effect.flip(saved.remove(1));

      expect(store.getState().saved[101]).toBeUndefined();
      expect(savedListOf(store.getState(), "all")).toMatchObject({ ids: [2], stale: true });
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("set a reminder by saving again, landing the server's item", () =>
    Effect.gen(function* () {
      seedSaved();

      const fake = yield* FakeApi;
      const reminded = { ...savedItem(1, 101, 10), remindAt: at(59) };

      yield* fake.route("POST /saved", (request) => {
        expect(request.body).toMatchObject({ messageId: 101, remindAt: at(59) });

        return Effect.succeed(reminded);
      });

      yield* saved.setReminder(101, at(59));

      expect(savedItemForMessage(store.getState(), 101)?.remindAt).toBe(at(59));
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );
});

describe("saved restore (Undo)", () => {
  afterEach(() => mutations.reset());

  it.effect("saves a done item again and marks the new item done", () =>
    Effect.gen(function* () {
      seedSaved();
      yield* TestClock.setTime(Date.parse(at(30)));

      // Item 1, done with a reminder still to come, was just removed.
      const removed = { ...savedItem(1, 101, 10, true), remindAt: at(50) };

      mutations.applySavedChange(101, null);

      const fake = yield* FakeApi;
      const posted: unknown[] = [];

      yield* fake.route("POST /saved", (request) => {
        posted.push(request.body);

        return Effect.succeed({ ...savedItem(7, 101, 30), remindAt: at(50) });
      });
      yield* fake.reply("PATCH /saved/7", { ...savedItem(7, 101, 30, true), remindAt: at(50) });

      const restored = yield* saved.restore(removed);

      expect(posted).toEqual([{ messageId: 101, remindAt: at(50) }]);
      expect(restored.status).toBe("done");
      expect(savedItemForMessage(store.getState(), 101)?.status).toBe("done");
      expect(savedListOf(store.getState(), "done").ids).toEqual([7]);
      expect(savedListOf(store.getState(), "in_progress").ids).toEqual([2]);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("leaves out a reminder that already went out or is now past, and skips the PATCH", () =>
    Effect.gen(function* () {
      seedSaved();
      yield* TestClock.setTime(Date.parse(at(30)));

      const fake = yield* FakeApi;
      const posted: unknown[] = [];

      yield* fake.route("POST /saved", (request) => {
        posted.push(request.body);

        return Effect.succeed(savedItem(8, 104, 30));
      });

      yield* saved.restore({ ...savedItem(4, 104, 5), remindAt: at(20) });
      yield* saved.restore({ ...savedItem(4, 104, 5), remindAt: at(50), remindedAt: at(25) });

      expect(posted).toEqual([
        { messageId: 104, remindAt: null },
        { messageId: 104, remindAt: null },
      ]);
      expect((yield* fake.requests).filter((request) => request.method === "PATCH")).toEqual([]);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );
});

describe("scheduled actions", () => {
  afterEach(() => mutations.reset());

  it.effect("send one now: sent moves it to Past", () =>
    Effect.gen(function* () {
      seedScheduled();

      const fake = yield* FakeApi;

      yield* fake.reply(
        "POST /scheduled_messages/1/send_now",
        scheduledMessage(1, 10, {
          state: "sent",
          sendable: false,
          sentAt: at(11),
          sentMessageId: 901,
        }),
      );

      expect(yield* scheduled.sendNow(1)).toBe("sent");
      expect(scheduledListOf(store.getState(), "pending").ids).toEqual([2]);
      expect(scheduledListOf(store.getState(), roomScheduledKey(ROOM)).ids).toEqual([2]);
      expect(scheduledListOf(store.getState(), "past").ids).toEqual([1, 9]);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("send one now: held keeps it pending", () =>
    Effect.gen(function* () {
      seedScheduled();

      const fake = yield* FakeApi;

      yield* fake.reply(
        "POST /scheduled_messages/2/send_now",
        scheduledMessage(2, 20, { state: "sending" }),
      );

      expect(yield* scheduled.sendNow(2)).toBe("held");
      expect(scheduledListOf(store.getState(), "pending").ids).toEqual([1, 2]);
      expect(store.getState().scheduled.items[2]?.state).toBe("sending");
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("send one now: a drop rejects with the reason and marks the lists for a reload", () =>
    Effect.gen(function* () {
      seedScheduled();

      const fake = yield* FakeApi;

      yield* fake.route("POST /scheduled_messages/1/send_now", () =>
        Effect.fail(new Validation({ message: "its thread was deleted", fields: {} })),
      );

      const failure = yield* Effect.flip(scheduled.sendNow(1));

      expect(failure).toBeInstanceOf(scheduled.ScheduledDropped);
      expect(failure.message).toBe("its thread was deleted");
      expect(scheduledListOf(store.getState(), "pending").stale).toBe(true);
      expect(scheduledListOf(store.getState(), "past").stale).toBe(true);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("send one now: another failure stays itself and leaves the lists alone", () =>
    Effect.gen(function* () {
      seedScheduled();

      const fake = yield* FakeApi;

      yield* fake.route("POST /scheduled_messages/1/send_now", () =>
        Effect.fail(new ServerError({ status: 500, message: "Something went wrong" })),
      );

      const failure = yield* Effect.flip(scheduled.sendNow(1));

      expect(failure).toBeInstanceOf(ServerError);
      expect(scheduledListOf(store.getState(), "pending").stale).toBe(false);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("cancel at once, and bring it back when it's sending (409)", () =>
    Effect.gen(function* () {
      seedScheduled();

      const fake = yield* FakeApi;
      let during: readonly number[] = [];

      yield* fake.route("DELETE /scheduled_messages/1", () => {
        during = scheduledListOf(store.getState(), "pending").ids;

        return Effect.fail(new Conflict({ message: "That message is sending right now" }));
      });

      yield* Effect.flip(scheduled.cancel(1));

      expect(during).toEqual([2]);
      expect(scheduledListOf(store.getState(), "pending").ids).toEqual([1, 2]);
      expect(scheduledListOf(store.getState(), roomScheduledKey(ROOM)).ids).toEqual([1, 2]);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("cancel: a 404 leaves it gone and reloads; a sent copy meanwhile stays sent", () =>
    Effect.gen(function* () {
      seedScheduled();

      const fake = yield* FakeApi;

      yield* fake.route("DELETE /scheduled_messages/1", () =>
        Effect.fail(new NotFound({ message: "Not found" })),
      );
      yield* Effect.flip(scheduled.cancel(1));

      expect(scheduledListOf(store.getState(), "pending")).toMatchObject({
        ids: [2],
        stale: true,
      });

      const sent = scheduledMessage(2, 20, {
        state: "sent",
        sendable: false,
        sentAt: at(21),
        sentMessageId: 902,
      });

      yield* fake.route("DELETE /scheduled_messages/2", () => {
        mutations.applyScheduled(sent);

        return Effect.fail(new Conflict({ message: "That message is sending right now" }));
      });
      yield* Effect.flip(scheduled.cancel(2));

      expect(store.getState().scheduled.items[2]?.state).toBe("sent");
      expect(scheduledListOf(store.getState(), "pending").ids).toEqual([]);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("create and reschedule land in order, and a room list asks for its room", () =>
    Effect.gen(function* () {
      seedScheduled();

      const fake = yield* FakeApi;

      yield* fake.reply(`POST /rooms/${ROOM}/scheduled_messages`, scheduledMessage(3, 15));
      yield* fake.reply("PATCH /scheduled_messages/1", scheduledMessage(1, 40));

      yield* scheduled.create(ROOM, {
        markdownSource: "Later 3",
        sendAt: at(15),
        threadId: null,
        replyToMessageId: null,
      });

      expect(scheduledListOf(store.getState(), "pending").ids).toEqual([1, 3, 2]);

      yield* scheduled.update(1, { sendAt: at(40) });

      expect(scheduledListOf(store.getState(), roomScheduledKey(ROOM)).ids).toEqual([3, 2, 1]);

      yield* fake.reply("GET /scheduled_messages", {
        scheduledMessages: [],
        conversations: [],
        nextCursor: null,
      });
      yield* scheduled.load(roomScheduledKey(ROOM));

      const last = (yield* fake.requests).at(-1);

      expect(last?.query).toEqual({ status: "pending", roomId: String(ROOM) });
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );
});
