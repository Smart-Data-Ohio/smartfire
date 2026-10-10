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
    replyTarget: null,
    markdownSource: `Later ${id}`,
    excerpt: `Later ${id}`,
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
      unreadRevision: 1,
      nextCursor: null,
    },
    "replace",
  );
  mutations.landActivityPage(
    "all",
    "handled",
    {
      items: [item(1, 10, "handled")],
      users: [],
      unreadCount: 5,
      unreadRevision: 1,
      nextCursor: null,
    },
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

        return Effect.succeed({ item: item(3, 30, "handled"), unreadCount: 9, unreadRevision: 2 });
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

      yield* fake.reply("POST /activity/3/open", {
        item: item(3, 30, "read"),
        unreadCount: 4,
        unreadRevision: 2,
      });

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
            ? {
                items: [item(1, 10)],
                users: [],
                unreadCount: 2,
                unreadRevision: 1,
                nextCursor: null,
              }
            : {
                items: [item(2, 20)],
                users: [],
                unreadCount: 2,
                unreadRevision: 1,
                nextCursor: "page-2",
              },
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
        mutations.applyActivityItem(item(2, 50, "handled"), { unreadCount: 3, unreadRevision: 2 });

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
        mutations.applyActivityItem(item(3, 50, "read"), { unreadCount: 4, unreadRevision: 2 });

        return Effect.succeed({
          items: [item(3, 30), item(2, 20)],
          users: [],
          unreadCount: 5,
          unreadRevision: 1,
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
        unreadRevision: 1,
        nextCursor: "page-2",
      });
      yield* activity.load("all", "unread");

      yield* fake.route("GET /activity", () => {
        // A reload starts while this next page is on its way.
        mutations.setActivityListLoading("all", "unread", false);
        mutations.landActivityPage(
          "all",
          "unread",
          { items: [item(4, 40)], users: [], unreadCount: 2, unreadRevision: 1, nextCursor: null },
          "replace",
        );

        return Effect.succeed({
          items: [item(1, 10)],
          users: [],
          unreadCount: 2,
          unreadRevision: 1,
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

      yield* fake.reply("GET /activity/unread_count", { unreadCount: 12, unreadRevision: 2 });

      expect(yield* activity.loadUnreadCount()).toBe(12);
      expect(store.getState().activity.unreadCount).toBe(12);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  for (const confirmation of [
    "websocket",
    "same-time websocket",
    "page",
    "count-only snapshot",
    "removal",
  ] as const) {
    it.effect(`counts an optimistic read only once when a ${confirmation} confirms it`, () =>
      Effect.gen(function* () {
        seedInbox();
        const fake = yield* FakeApi;
        const started = yield* Deferred.make<void>();
        const release = yield* Deferred.make<void>();
        const read = item(3, confirmation === "same-time websocket" ? 30 : 40, "read");
        const snapshot = { unreadCount: 4, unreadRevision: 2 };

        expect(store.getState().activity.unreadCount).toBe(5);
        yield* fake.route("PATCH /activity/3", () =>
          Deferred.succeed(started, undefined).pipe(
            Effect.andThen(Deferred.await(release)),
            Effect.as({ item: read, ...snapshot }),
          ),
        );
        const changing = yield* Effect.forkChild(activity.setState(3, "read"));

        yield* Deferred.await(started);
        expect(store.getState().activity.unreadCount).toBe(4);

        if (confirmation === "count-only snapshot") {
          yield* fake.reply("GET /activity/unread_count", snapshot);
          yield* activity.loadUnreadCount();
        } else if (confirmation === "page") {
          yield* fake.reply("GET /activity", {
            items: [read],
            users: [],
            ...snapshot,
            nextCursor: null,
          });
          yield* activity.load("all", "read");
        } else {
          mutations.applyEvents(
            confirmation === "removal"
              ? [{ seq: 1, topic: "user", type: "activity.removed", data: { id: 3, ...snapshot } }]
              : [
                  {
                    seq: 1,
                    topic: "user",
                    type: "activity.item",
                    data: { item: read, ...snapshot },
                  },
                ],
            0,
          );
        }

        expect(store.getState().activity.unreadCount).toBe(4);
        yield* Deferred.succeed(release, undefined);
        yield* Fiber.join(changing);
        expect(store.getState().activity.unreadCount).toBe(4);

        if (confirmation === "removal") {
          expect(store.getState().activity.items[3]).toBeUndefined();
        }
      }).pipe(Effect.provide(FakeApi.layerClient)),
    );
  }

  for (const outcome of ["reply", "failure"] as const) {
    it.effect(`keeps the newest count-only increase after a pending read's ${outcome}`, () =>
      Effect.gen(function* () {
        seedInbox();
        const fake = yield* FakeApi;
        const started = yield* Deferred.make<void>();
        const release = yield* Deferred.make<void>();
        const counts = [store.getState().activity.unreadCount];

        yield* fake.route("PATCH /activity/3", () =>
          Deferred.succeed(started, undefined).pipe(
            Effect.andThen(Deferred.await(release)),
            Effect.andThen(
              outcome === "failure"
                ? refuse()
                : Effect.succeed({ item: item(3, 40, "read"), unreadCount: 4, unreadRevision: 2 }),
            ),
          ),
        );
        const changing = yield* Effect.forkChild(Effect.exit(activity.setState(3, "read")));

        yield* Deferred.await(started);
        counts.push(store.getState().activity.unreadCount);
        yield* fake.reply("GET /activity/unread_count", { unreadCount: 6, unreadRevision: 4 });
        yield* activity.loadUnreadCount();
        counts.push(store.getState().activity.unreadCount);
        yield* fake.reply("GET /activity/unread_count", { unreadCount: 5, unreadRevision: 3 });
        yield* activity.loadUnreadCount();
        counts.push(store.getState().activity.unreadCount);
        yield* Deferred.succeed(release, undefined);
        const result = yield* Fiber.join(changing);

        counts.push(store.getState().activity.unreadCount);
        expect(result._tag).toBe(outcome === "reply" ? "Success" : "Failure");
        expect(counts).toEqual([5, 4, 4, 4, 6]);
        expect(store.getState().activity.serverUnread?.unreadRevision).toBe(4);
        expect(store.getState().activity.deferredUnread).toBeNull();
      }).pipe(Effect.provide(FakeApi.layerClient)),
    );
  }

  it.effect("reconciles an already-read confirmation at the same count revision", () =>
    Effect.gen(function* () {
      seedInbox();
      const fake = yield* FakeApi;

      expect(store.getState().activity.unreadCount).toBe(5);
      yield* fake.route("PATCH /activity/3", () => {
        expect(store.getState().activity.unreadCount).toBe(4);
        const reply = { item: item(3, 30, "read"), unreadCount: 5, unreadRevision: 1 };

        mutations.applyActivityItem(reply.item, reply);
        expect(store.getState().activity.unreadCount).toBe(5);

        return Effect.succeed(reply);
      });
      yield* activity.setState(3, "read");
      expect(store.getState().activity.unreadCount).toBe(5);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("reconciles only the confirmed item while another optimistic read is pending", () =>
    Effect.gen(function* () {
      seedInbox();
      const fake = yield* FakeApi;
      const firstStarted = yield* Deferred.make<void>();
      const firstRelease = yield* Deferred.make<void>();
      const secondStarted = yield* Deferred.make<void>();
      const secondRelease = yield* Deferred.make<void>();

      yield* fake.route("PATCH /activity/3", () =>
        Deferred.succeed(firstStarted, undefined).pipe(
          Effect.andThen(Deferred.await(firstRelease)),
          Effect.as({ item: item(3, 40, "read"), unreadCount: 4, unreadRevision: 3 }),
        ),
      );
      yield* fake.route("PATCH /activity/2", () =>
        Deferred.succeed(secondStarted, undefined).pipe(
          Effect.andThen(Deferred.await(secondRelease)),
          Effect.as({ item: item(2, 50, "read"), unreadCount: 3, unreadRevision: 4 }),
        ),
      );

      expect(store.getState().activity.unreadCount).toBe(5);
      const first = yield* Effect.forkChild(activity.setState(3, "read"));

      yield* Deferred.await(firstStarted);
      expect(store.getState().activity.unreadCount).toBe(4);
      const second = yield* Effect.forkChild(activity.setState(2, "read"));

      yield* Deferred.await(secondStarted);
      expect(store.getState().activity.unreadCount).toBe(3);
      yield* fake.reply("GET /activity/unread_count", { unreadCount: 3, unreadRevision: 4 });
      yield* activity.loadUnreadCount();
      expect(store.getState().activity.unreadCount).toBe(3);
      // This page's item predates the pending read, although its count is newer.
      yield* fake.reply("GET /activity", {
        items: [item(3, 20, "read")],
        users: [],
        unreadCount: 5,
        unreadRevision: 2,
        nextCursor: null,
      });
      yield* activity.load("all", "read");
      expect(store.getState().activity.unreadCount).toBe(3);
      mutations.applyActivityItem(item(3, 40, "read"), { unreadCount: 4, unreadRevision: 3 });
      expect(store.getState().activity.unreadCount).toBe(3);
      expect(store.getState().activity.deferredUnread?.unreadRevision).toBe(4);
      yield* Deferred.succeed(firstRelease, undefined);
      yield* Fiber.join(first);
      expect(store.getState().activity.unreadCount).toBe(3);
      yield* Deferred.succeed(secondRelease, undefined);
      yield* Fiber.join(second);
      expect(store.getState().activity.unreadCount).toBe(3);
      expect(store.getState().activity.deferredUnread).toBeNull();
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("never subtracts a read twice when replies settle in reverse order", () =>
    Effect.gen(function* () {
      seedInbox();
      const fake = yield* FakeApi;
      const firstStarted = yield* Deferred.make<void>();
      const firstRelease = yield* Deferred.make<void>();
      const secondStarted = yield* Deferred.make<void>();
      const secondRelease = yield* Deferred.make<void>();

      yield* fake.route("PATCH /activity/3", () =>
        Deferred.succeed(firstStarted, undefined).pipe(
          Effect.andThen(Deferred.await(firstRelease)),
          Effect.as({ item: item(3, 40, "read"), unreadCount: 4, unreadRevision: 2 }),
        ),
      );
      yield* fake.route("PATCH /activity/2", () =>
        Deferred.succeed(secondStarted, undefined).pipe(
          Effect.andThen(Deferred.await(secondRelease)),
          Effect.as({ item: item(2, 50, "read"), unreadCount: 3, unreadRevision: 3 }),
        ),
      );

      expect(store.getState().activity.unreadCount).toBe(5);
      const first = yield* Effect.forkChild(activity.setState(3, "read"));

      yield* Deferred.await(firstStarted);
      expect(store.getState().activity.unreadCount).toBe(4);
      const second = yield* Effect.forkChild(activity.setState(2, "read"));

      yield* Deferred.await(secondStarted);
      expect(store.getState().activity.unreadCount).toBe(3);
      yield* fake.reply("GET /activity/unread_count", { unreadCount: 3, unreadRevision: 3 });
      yield* activity.loadUnreadCount();
      expect(store.getState().activity.unreadCount).toBe(3);
      yield* Deferred.succeed(secondRelease, undefined);
      yield* Fiber.join(second);
      expect(store.getState().activity.unreadCount).toBe(3);
      yield* Deferred.succeed(firstRelease, undefined);
      yield* Fiber.join(first);
      expect(store.getState().activity.unreadCount).toBe(3);
      expect(store.getState().activity.pendingUnread).toEqual({});
      expect(store.getState().activity.deferredUnread).toBeNull();
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  for (const outcome of ["failure", "interruption"] as const) {
    for (const sameTime of [false, true]) {
      it.effect(`retains a confirmed read after HTTP ${outcome} (same timestamp=${sameTime})`, () =>
        Effect.gen(function* () {
          seedInbox();
          const fake = yield* FakeApi;
          const firstStarted = yield* Deferred.make<void>();
          const firstRelease = yield* Deferred.make<void>();
          const secondStarted = yield* Deferred.make<void>();
          const secondRelease = yield* Deferred.make<void>();
          const read = item(3, sameTime ? 30 : 40, "read");

          yield* fake.route("PATCH /activity/3", () =>
            Deferred.succeed(firstStarted, undefined).pipe(
              Effect.andThen(Deferred.await(firstRelease)),
              Effect.andThen(refuse()),
            ),
          );
          yield* fake.route("PATCH /activity/2", () =>
            Deferred.succeed(secondStarted, undefined).pipe(
              Effect.andThen(Deferred.await(secondRelease)),
              Effect.andThen(refuse()),
            ),
          );

          expect(store.getState().activity.unreadCount).toBe(5);
          const first = yield* Effect.forkChild(Effect.exit(activity.setState(3, "read")));

          yield* Deferred.await(firstStarted);
          expect(store.getState().activity.unreadCount).toBe(4);
          const second = yield* Effect.forkChild(Effect.exit(activity.setState(2, "read")));

          yield* Deferred.await(secondStarted);
          expect(store.getState().activity.unreadCount).toBe(3);
          mutations.applyActivityItem(read, { unreadCount: 4, unreadRevision: 2 });
          expect(store.getState().activity.unreadCount).toBe(3);
          expect(store.getState().activity.serverUnread?.unreadRevision).toBe(1);
          expect(store.getState().activity.deferredUnread?.unreadRevision).toBe(2);

          if (outcome === "failure") {
            yield* Deferred.succeed(firstRelease, undefined);
            expect((yield* Fiber.join(first))._tag).toBe("Failure");
          } else {
            yield* Fiber.interrupt(first);
          }

          expect(store.getState().activity.unreadCount).toBe(3);
          expect(store.getState().activity.items[3]).toEqual(read);
          expect(Object.values(store.getState().activity.pendingUnread)).toContainEqual(
            expect.objectContaining({ itemId: 3, delta: -1, coveredAtRevision: 2, settled: true }),
          );
          yield* fake.route("PATCH /activity/3", () => {
            expect(store.getState().activity.unreadCount).toBe(3);

            return Effect.succeed({ item: read, unreadCount: 4, unreadRevision: 2 });
          });
          yield* activity.setState(3, "read");
          expect(store.getState().activity.unreadCount).toBe(3);
          expect(store.getState().activity.items[3]).toEqual(read);
          expect(
            Object.values(store.getState().activity.pendingUnread).filter(
              (change) => change.itemId === 3,
            ),
          ).toHaveLength(1);
          yield* Deferred.succeed(secondRelease, undefined);
          expect((yield* Fiber.join(second))._tag).toBe("Failure");
          expect(store.getState().activity.unreadCount).toBe(4);
          expect(store.getState().activity.items[2]?.state).toBe("unread");
          expect(store.getState().activity.pendingUnread).toEqual({});
          expect(store.getState().activity.deferredUnread).toBeNull();
        }).pipe(Effect.provide(FakeApi.layerClient)),
      );
    }
  }

  it.effect("does not count a retry of an equal-timestamp confirmed read twice", () =>
    Effect.gen(function* () {
      seedInbox();
      const fake = yield* FakeApi;
      const firstStarted = yield* Deferred.make<void>();
      const firstRelease = yield* Deferred.make<void>();
      const secondStarted = yield* Deferred.make<void>();
      const secondRelease = yield* Deferred.make<void>();
      const read = item(3, 30, "read");

      yield* fake.route("PATCH /activity/3", () =>
        Deferred.succeed(firstStarted, undefined).pipe(
          Effect.andThen(Deferred.await(firstRelease)),
          Effect.andThen(refuse()),
        ),
      );
      yield* fake.route("PATCH /activity/2", () =>
        Deferred.succeed(secondStarted, undefined).pipe(
          Effect.andThen(Deferred.await(secondRelease)),
          Effect.andThen(refuse()),
        ),
      );
      const first = yield* Effect.forkChild(Effect.exit(activity.setState(3, "read")));

      yield* Deferred.await(firstStarted);
      const second = yield* Effect.forkChild(Effect.exit(activity.setState(2, "read")));

      yield* Deferred.await(secondStarted);
      mutations.applyActivityItem(read, { unreadCount: 4, unreadRevision: 2 });
      yield* Deferred.succeed(firstRelease, undefined);
      yield* Fiber.join(first);
      const counts = [store.getState().activity.unreadCount];

      yield* fake.route("PATCH /activity/3", () => {
        counts.push(store.getState().activity.unreadCount);

        return Effect.succeed({ item: read, unreadCount: 4, unreadRevision: 2 });
      });
      yield* activity.setState(3, "read");
      counts.push(store.getState().activity.unreadCount);
      expect(counts).toEqual([3, 3, 3]);
      expect(
        Object.values(store.getState().activity.pendingUnread).filter(
          (change) => change.itemId === 3,
        ),
      ).toHaveLength(1);
      yield* Deferred.succeed(secondRelease, undefined);
      yield* Fiber.join(second);
      expect(store.getState().activity.unreadCount).toBe(4);
      expect(store.getState().activity.pendingUnread).toEqual({});
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect(
    "composes a confirmed read with a subsequent local unread while another read is pending",
    () =>
      Effect.gen(function* () {
        seedInbox();
        const fake = yield* FakeApi;
        const firstStarted = yield* Deferred.make<void>();
        const firstRelease = yield* Deferred.make<void>();
        const secondStarted = yield* Deferred.make<void>();
        const secondRelease = yield* Deferred.make<void>();

        yield* fake.route("PATCH /activity/3", () =>
          Deferred.succeed(firstStarted, undefined).pipe(
            Effect.andThen(Deferred.await(firstRelease)),
            Effect.andThen(refuse()),
          ),
        );
        yield* fake.route("PATCH /activity/2", () =>
          Deferred.succeed(secondStarted, undefined).pipe(
            Effect.andThen(Deferred.await(secondRelease)),
            Effect.andThen(refuse()),
          ),
        );
        const first = yield* Effect.forkChild(Effect.exit(activity.setState(3, "read")));

        yield* Deferred.await(firstStarted);
        const second = yield* Effect.forkChild(Effect.exit(activity.setState(2, "read")));

        yield* Deferred.await(secondStarted);
        mutations.applyActivityItem(item(3, 30, "read"), { unreadCount: 4, unreadRevision: 2 });
        yield* Deferred.succeed(firstRelease, undefined);
        yield* Fiber.join(first);
        const counts = [store.getState().activity.unreadCount];

        yield* fake.route("PATCH /activity/3", () => {
          counts.push(store.getState().activity.unreadCount);
          const reply = { item: item(3, 30), unreadCount: 5, unreadRevision: 3 };

          mutations.applyActivityItem(reply.item, reply);
          counts.push(store.getState().activity.unreadCount);

          return Effect.succeed(reply);
        });
        yield* activity.setState(3, "unread");
        counts.push(store.getState().activity.unreadCount);
        expect(counts).toEqual([3, 4, 4, 4]);
        expect(store.getState().activity.items[3]?.state).toBe("unread");
        yield* Deferred.succeed(secondRelease, undefined);
        yield* Fiber.join(second);
        expect(store.getState().activity.unreadCount).toBe(5);
        expect(store.getState().activity.pendingUnread).toEqual({});
      }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("absorbs an already-counted no-op read while another read is unresolved", () =>
    Effect.gen(function* () {
      seedInbox();
      const fake = yield* FakeApi;
      const started = yield* Deferred.make<void>();
      const release = yield* Deferred.make<void>();

      yield* fake.route("PATCH /activity/3", () =>
        Deferred.succeed(started, undefined).pipe(
          Effect.andThen(Deferred.await(release)),
          Effect.as({ item: item(3, 40, "read"), unreadCount: 4, unreadRevision: 2 }),
        ),
      );
      expect(store.getState().activity.unreadCount).toBe(5);
      const first = yield* Effect.forkChild(activity.setState(3, "read"));

      yield* Deferred.await(started);
      expect(store.getState().activity.unreadCount).toBe(4);
      yield* fake.route("PATCH /activity/2", () => {
        expect(store.getState().activity.unreadCount).toBe(3);

        // This item's read already belongs to the installed revision; the PATCH is a no-op.
        return Effect.succeed({ item: item(2, 20, "read"), unreadCount: 5, unreadRevision: 1 });
      });
      yield* activity.setState(2, "read");
      expect(store.getState().activity.unreadCount).toBe(4);
      yield* Deferred.succeed(release, undefined);
      yield* Fiber.join(first);
      expect(store.getState().activity.unreadCount).toBe(4);
      expect(store.getState().activity.pendingUnread).toEqual({});
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("releases the newest held count after a read stalls for 15 seconds", () =>
    Effect.gen(function* () {
      seedInbox();
      const fake = yield* FakeApi;
      const started = yield* Deferred.make<void>();

      yield* fake.route("PATCH /activity/3", () =>
        Deferred.succeed(started, undefined).pipe(Effect.andThen(Effect.never)),
      );
      const changing = yield* Effect.forkChild(Effect.exit(activity.setState(3, "read")));

      yield* Deferred.await(started);
      expect(store.getState().activity.unreadCount).toBe(4);
      yield* fake.reply("GET /activity/unread_count", { unreadCount: 6, unreadRevision: 3 });
      yield* activity.loadUnreadCount();
      expect(store.getState().activity.unreadCount).toBe(4);
      const refreshStarted = yield* Deferred.make<void>();
      const refreshRelease = yield* Deferred.make<void>();

      yield* fake.route("GET /activity/unread_count", () =>
        Deferred.succeed(refreshStarted, undefined).pipe(
          Effect.andThen(Deferred.await(refreshRelease)),
          Effect.as({ unreadCount: 5, unreadRevision: 2 }),
        ),
      );
      yield* TestClock.adjust("14999 millis");
      expect(store.getState().activity.unreadCount).toBe(4);
      yield* TestClock.adjust("1 millis");
      expect(store.getState().activity.unreadCount).toBe(6);
      expect((yield* Fiber.join(changing))._tag).toBe("Failure");
      expect(store.getState().activity.pendingUnread).toEqual({});
      expect(store.getState().activity.items[3]?.state).toBe("unread");
      yield* Deferred.await(refreshStarted);
      yield* Deferred.succeed(refreshRelease, undefined);
      yield* TestClock.adjust("20 millis");
      expect(store.getState().activity.unreadCount).toBe(6);
      yield* fake.reply("PATCH /activity/3", {
        item: item(3, 40, "read"),
        unreadCount: 5,
        unreadRevision: 4,
      });
      yield* activity.setState(3, "read");
      expect(store.getState().activity.unreadCount).toBe(5);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect(
    "clears 40 items without double subtraction as every reply arrives in reverse order",
    () =>
      Effect.gen(function* () {
        mutations.reset();
        const fake = yield* FakeApi;
        const items = Array.from({ length: 40 }, (_, index) => item(index + 1, index));
        // Three other unread items keep a zero clamp from hiding incorrect intermediate counts.
        mutations.landActivityPage(
          "all",
          "unread",
          {
            items,
            users: [],
            unreadCount: 43,
            unreadRevision: 1,
            nextCursor: null,
          },
          "replace",
        );
        const changes = [];

        for (const [index, current] of items.entries()) {
          const started = yield* Deferred.make<void>();
          const release = yield* Deferred.make<void>();

          yield* fake.route(`PATCH /activity/${current.id}`, () =>
            Deferred.succeed(started, undefined).pipe(
              Effect.andThen(Deferred.await(release)),
              Effect.as({
                item: item(current.id, 60 + index, "handled"),
                unreadCount: 42 - index,
                unreadRevision: 2 + index,
              }),
            ),
          );
          const changing = yield* Effect.forkChild(activity.setState(current.id, "handled"));

          yield* Deferred.await(started);
          expect(store.getState().activity.unreadCount).toBe(42 - index);
          changes.push({ release, changing });
        }

        yield* fake.reply("GET /activity/unread_count", { unreadCount: 3, unreadRevision: 41 });
        yield* activity.loadUnreadCount();
        expect(store.getState().activity.unreadCount).toBe(3);

        for (const { release, changing } of changes.reverse()) {
          yield* Deferred.succeed(release, undefined);
          yield* Fiber.join(changing);
          expect(store.getState().activity.unreadCount).toBe(3);
        }

        expect(inboxIds("unread")).toEqual([]);
        expect(store.getState().activity.pendingUnread).toEqual({});
        expect(store.getState().activity.deferredUnread).toBeNull();
        expect((yield* fake.requests).filter((request) => request.method === "PATCH")).toHaveLength(
          40,
        );
      }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("rejects a stale matching item when a newer opposite state is already held", () =>
    Effect.gen(function* () {
      seedInbox();
      const fake = yield* FakeApi;
      const started = yield* Deferred.make<void>();
      const release = yield* Deferred.make<void>();

      yield* fake.route("PATCH /activity/3", () =>
        Deferred.succeed(started, undefined).pipe(
          Effect.andThen(Deferred.await(release)),
          Effect.as({ item: item(3, 60, "read"), unreadCount: 4, unreadRevision: 4 }),
        ),
      );
      expect(store.getState().activity.unreadCount).toBe(5);
      const changing = yield* Effect.forkChild(activity.setState(3, "read"));

      yield* Deferred.await(started);
      expect(store.getState().activity.unreadCount).toBe(4);
      mutations.applyActivityItem(item(3, 50), { unreadCount: 5, unreadRevision: 2 });
      expect(store.getState().activity.unreadCount).toBe(4);
      mutations.applyActivityItem(item(3, 40, "read"), { unreadCount: 5, unreadRevision: 3 });
      expect(store.getState().activity.items[3]?.state).toBe("unread");
      expect(store.getState().activity.unreadCount).toBe(4);
      mutations.applyActivityItem(item(3, 60, "read"), { unreadCount: 4, unreadRevision: 4 });
      expect(store.getState().activity.unreadCount).toBe(4);
      yield* Deferred.succeed(release, undefined);
      yield* Fiber.join(changing);
      expect(store.getState().activity.unreadCount).toBe(4);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect(
    "keeps a removed item absent when a delayed reply carries an unrelated newer count",
    () =>
      Effect.gen(function* () {
        seedInbox();
        const fake = yield* FakeApi;
        const started = yield* Deferred.make<void>();
        const release = yield* Deferred.make<void>();

        yield* fake.route("PATCH /activity/3", () =>
          Deferred.succeed(started, undefined).pipe(
            Effect.andThen(Deferred.await(release)),
            Effect.as({ item: item(3, 40, "read"), unreadCount: 5, unreadRevision: 3 }),
          ),
        );
        expect(store.getState().activity.unreadCount).toBe(5);
        const changing = yield* Effect.forkChild(activity.setState(3, "read"));

        yield* Deferred.await(started);
        expect(store.getState().activity.unreadCount).toBe(4);
        mutations.applyEvents(
          [
            {
              seq: 1,
              topic: "user",
              type: "activity.removed",
              data: { id: 3, unreadCount: 4, unreadRevision: 2 },
            },
          ],
          0,
        );
        expect(store.getState().activity.unreadCount).toBe(4);
        mutations.setActivityUnreadCount({ unreadCount: 5, unreadRevision: 3 });
        expect(store.getState().activity.unreadCount).toBe(5);
        yield* Deferred.succeed(release, undefined);
        yield* Fiber.join(changing);
        expect(store.getState().activity.unreadCount).toBe(5);
        expect(store.getState().activity.items[3]).toBeUndefined();
      }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("keep a cleared inbox at zero when the boot count arrives late", () =>
    Effect.gen(function* () {
      mutations.reset();

      const fake = yield* FakeApi;
      const started = yield* Deferred.make<void>();
      const release = yield* Deferred.make<void>();

      yield* fake.route("GET /activity/unread_count", () =>
        Deferred.succeed(started, undefined).pipe(
          Effect.andThen(Deferred.await(release)),
          Effect.as({ unreadCount: 10, unreadRevision: 1 }),
        ),
      );

      const boot = yield* Effect.forkChild(activity.loadUnreadCount());

      yield* Deferred.await(started);
      mutations.landActivityPage(
        "all",
        "unread",
        { items: [item(2, 20)], users: [], unreadCount: 1, unreadRevision: 2, nextCursor: null },
        "replace",
      );
      yield* fake.reply("PATCH /activity/2", {
        item: item(2, 30, "handled"),
        unreadCount: 0,
        unreadRevision: 3,
      });
      yield* activity.setState(2, "handled");

      expect(store.getState().activity.unreadCount).toBe(0);

      yield* Deferred.succeed(release, undefined);
      yield* Fiber.join(boot);

      expect(inboxIds("unread")).toEqual([]);
      expect(store.getState().activity.unreadCount).toBe(0);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("keep a newer count when an older state reply arrives after an event", () =>
    Effect.gen(function* () {
      seedInbox();

      const fake = yield* FakeApi;

      yield* fake.route("PATCH /activity/2", () => {
        mutations.applyActivityItem(item(2, 50, "handled"), { unreadCount: 0, unreadRevision: 3 });

        return Effect.succeed({ item: item(2, 40, "read"), unreadCount: 4, unreadRevision: 2 });
      });
      yield* activity.setState(2, "read");

      expect(store.getState().activity.items[2]?.state).toBe("handled");
      expect(store.getState().activity.unreadCount).toBe(0);
      expect(store.getState().activity.pendingUnread).toEqual({});
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("reconcile concurrent clears whose replies arrive in reverse order", () =>
    Effect.gen(function* () {
      mutations.reset();
      mutations.landActivityPage(
        "all",
        "unread",
        {
          items: [item(3, 30), item(2, 20)],
          users: [],
          unreadCount: 2,
          unreadRevision: 1,
          nextCursor: null,
        },
        "replace",
      );

      const fake = yield* FakeApi;
      const started = yield* Deferred.make<void>();
      const release = yield* Deferred.make<void>();

      yield* fake.route("PATCH /activity/3", () =>
        Deferred.succeed(started, undefined).pipe(
          Effect.andThen(Deferred.await(release)),
          Effect.as({ item: item(3, 40, "handled"), unreadCount: 1, unreadRevision: 2 }),
        ),
      );
      yield* fake.reply("PATCH /activity/2", {
        item: item(2, 50, "read"),
        unreadCount: 0,
        unreadRevision: 3,
      });

      const first = yield* Effect.forkChild(activity.setState(3, "handled"));

      yield* Deferred.await(started);
      yield* activity.setState(2, "read");
      yield* Deferred.succeed(release, undefined);
      yield* Fiber.join(first);

      expect(inboxIds("unread")).toEqual([]);
      expect(store.getState().activity.unreadCount).toBe(0);
      expect(store.getState().activity.pendingUnread).toEqual({});
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  for (const freshFirst of [true, false]) {
    it.effect(
      `keeps the fresh count when overlapping GETs return ${freshFirst ? "fresh" : "old"} first`,
      () =>
        Effect.gen(function* () {
          seedInbox();

          const fake = yield* FakeApi;
          const started = yield* Deferred.make<void>();
          const release = yield* Deferred.make<void>();

          yield* fake.route("GET /activity/unread_count", () =>
            Deferred.succeed(started, undefined).pipe(
              Effect.andThen(Deferred.await(release)),
              Effect.as({ unreadCount: 10, unreadRevision: 2 }),
            ),
          );

          const older = yield* Effect.forkChild(activity.loadUnreadCount());

          yield* Deferred.await(started);
          const newerStarted = yield* Deferred.make<void>();
          const newerRelease = yield* Deferred.make<void>();

          yield* fake.route("GET /activity/unread_count", () =>
            Deferred.succeed(newerStarted, undefined).pipe(
              Effect.andThen(Deferred.await(newerRelease)),
              Effect.as({ unreadCount: 7, unreadRevision: 3 }),
            ),
          );
          const newer = yield* Effect.forkChild(activity.loadUnreadCount());

          yield* Deferred.await(newerStarted);

          if (freshFirst) {
            yield* Deferred.succeed(newerRelease, undefined);
            yield* Fiber.join(newer);
            expect(store.getState().activity.unreadCount).toBe(7);
          }

          yield* Deferred.succeed(release, undefined);
          yield* Fiber.join(older);

          if (!freshFirst) {
            yield* Deferred.succeed(newerRelease, undefined);
            yield* Fiber.join(newer);
          }

          expect(store.getState().activity.serverUnread).toEqual({
            unreadCount: 7,
            unreadRevision: 3,
          });
          expect(
            (yield* fake.requests).filter((request) => request.path === "/activity/unread_count"),
          ).toHaveLength(2);
        }).pipe(Effect.provide(FakeApi.layerClient)),
    );
  }

  it.effect(
    "accepts a legitimate increase from a blocked count GET after an older page lands",
    () =>
      Effect.gen(function* () {
        seedInbox();
        const fake = yield* FakeApi;
        const started = yield* Deferred.make<void>();
        const release = yield* Deferred.make<void>();

        yield* fake.route("GET /activity/unread_count", () =>
          Deferred.succeed(started, undefined).pipe(
            Effect.andThen(Deferred.await(release)),
            Effect.as({ unreadCount: 6, unreadRevision: 3 }),
          ),
        );
        const count = yield* Effect.forkChild(activity.loadUnreadCount());

        yield* Deferred.await(started);
        mutations.applyActivityItem(item(3, 50, "read"), { unreadCount: 4, unreadRevision: 2 });
        yield* fake.reply("GET /activity", {
          items: [item(3, 30), item(2, 20)],
          users: [],
          unreadCount: 5,
          unreadRevision: 1,
          nextCursor: null,
        });
        yield* activity.load("all", "unread");
        expect(store.getState().activity.unreadCount).toBe(4);

        yield* Deferred.succeed(release, undefined);
        yield* Fiber.join(count);
        expect(store.getState().activity.unreadCount).toBe(6);
      }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  for (const concurrent of ["arrival", "other-tab item", "other-tab removal"]) {
    it.effect(`keeps a ${concurrent} count that races a delayed clear reply`, () =>
      Effect.gen(function* () {
        seedInbox();
        const fake = yield* FakeApi;
        const started = yield* Deferred.make<void>();
        const release = yield* Deferred.make<void>();

        yield* fake.route("PATCH /activity/3", () =>
          Deferred.succeed(started, undefined).pipe(
            Effect.andThen(Deferred.await(release)),
            Effect.as({ item: item(3, 40, "read"), unreadCount: 4, unreadRevision: 2 }),
          ),
        );
        const clearing = yield* Effect.forkChild(activity.setState(3, "read"));

        yield* Deferred.await(started);
        const unreadCount = concurrent === "arrival" ? 5 : 3;

        mutations.applyEvents(
          concurrent === "other-tab removal"
            ? [
                {
                  seq: 1,
                  topic: "user",
                  type: "activity.removed",
                  data: { id: 2, unreadCount, unreadRevision: 3 },
                },
              ]
            : [
                {
                  seq: 1,
                  topic: "user",
                  type: "activity.item",
                  data: {
                    item: concurrent === "arrival" ? item(4, 50) : item(2, 50, "handled"),
                    unreadCount,
                    unreadRevision: 3,
                  },
                },
              ],
          Date.parse(at(50)),
        );
        yield* Deferred.succeed(release, undefined);
        yield* Fiber.join(clearing);

        expect(store.getState().activity.unreadCount).toBe(unreadCount);
        expect(store.getState().activity.pendingUnread).toEqual({});
        expect((yield* fake.requests).map((request) => request.path)).toEqual(["/activity/3"]);
      }).pipe(Effect.provide(FakeApi.layerClient)),
    );
  }
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
