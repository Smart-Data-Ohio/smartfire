import { beforeEach, describe, expect, it } from "@effect/vitest";
import { Clock, Deferred, Effect, Fiber, Layer, Random, Ref, Schema } from "effect";
import { TestClock } from "effect/testing";
import { NetworkError, NotFound, ServerError, Validation } from "../api/errors.ts";
import { CreateMessage as CreateMessageSchema } from "../api/schema/message.ts";
import {
  FakeApi,
  meFixture,
  messageFixture,
  pageFixture,
  roomDetailFixture,
  sidebarFixture,
  sidebarRowFixture,
  userFixture,
} from "../api/testing.ts";
import type { ActivityItem } from "../gen/ActivityItem.ts";
import type { ClientFrame } from "../gen/ClientFrame.ts";
import type { CreateMessage } from "../gen/CreateMessage.ts";
import type { MessageDTO } from "../gen/MessageDTO.ts";
import type { SidebarRow } from "../gen/SidebarRow.ts";
import type { SyncEvent } from "../gen/SyncEvent.ts";
import { activityListOf } from "../store/activity.ts";
import { mutations, store } from "../store/store.ts";
import { MAX_REMOVED_THREADS } from "../store/threads.ts";
import { BOARD, boardDetail, boardListing, boardThread } from "../test/board-fixtures.ts";
import * as activity from "./activity-actions.ts";
import * as boardActions from "./board-actions.ts";
import { CURSOR_STORAGE_KEY } from "./cursor.ts";
import { Engine } from "./engine.ts";
import { SyncServices } from "./layers.ts";
import { Outbox } from "./outbox.ts";
import * as roomActions from "./room-actions.ts";
import { invalidateRoom, onRoomRefresh } from "./room-refresh.ts";
import * as session from "./session.ts";
import { onResync } from "./signals.ts";
import { MemorySocket, TestLifecycle } from "./testing.ts";
import * as threadActions from "./thread-actions.ts";
import { Typing } from "./typing.ts";

const TestLayer = SyncServices.pipe(
  Layer.provideMerge(Layer.mergeAll(FakeApi.layerClient, MemorySocket.layer, TestLifecycle.layer)),
);

/** Jitter's random draw at 0.5 scales every delay by exactly 1. */
const noJitter: Random.Random = { nextIntUnsafe: () => 0, nextDoubleUnsafe: () => 0.5 };

type Services = Layer.Success<typeof TestLayer>;

/** Runs `body` against fresh sync services, an in-memory socket and API, and no jitter. */
const withSync = <A, E>(body: Effect.Effect<A, E, Services>) =>
  body.pipe(Effect.provide(TestLayer), Effect.provideService(Random.Random, noJitter));

/** Replies in board post 1's thread. */
const threadReplies = (ids: readonly number[]) =>
  pageFixture(ids.map((id) => messageFixture(id, BOARD, { threadId: 1 })));

/** Lets forked fibers run and the 16 ms coalescing window close. */
const settle = TestClock.adjust("20 millis");

/** Serves the routes the engine and room views fetch. */
const serve = (roomMessages: readonly MessageDTO[]) =>
  Effect.gen(function* () {
    const api = yield* FakeApi;

    yield* api.reply("GET /sidebar", sidebarFixture([sidebarRowFixture(12, "general")]));
    yield* api.reply("GET /rooms/12", roomDetailFixture(12));
    yield* api.reply("GET /rooms/12/messages", pageFixture(roomMessages));
    yield* api.reply("GET /users", { users: [] });
  });

/** Starts the engine in the background (the test's end interrupts it) and lets it connect. */
const startEngine = Effect.gen(function* () {
  const engine = yield* Engine;

  yield* Effect.forkChild(engine.run);
  yield* settle;
});

const pushEvents = (...events: readonly SyncEvent[]) =>
  Effect.gen(function* () {
    const socket = yield* MemorySocket;

    yield* socket.push({ t: "batch", events: [...events] });
    yield* settle;
  });

const welcome = (seq: number, resumed: boolean, epoch = "e1") =>
  Effect.gen(function* () {
    const socket = yield* MemorySocket;

    yield* socket.push({ t: "welcome", epoch, seq, resumed });
    yield* settle;
  });

function unreadEvent(seq: number, roomId = 12): SyncEvent {
  return {
    seq,
    topic: "user",
    type: "room.unread",
    data: { roomId, messageId: 100 + seq, mentioned: false },
  };
}

/** Milliseconds between successive dials. */
function gaps(dials: readonly number[]): readonly number[] {
  const result: number[] = [];

  for (let index = 1; index < dials.length; index++) {
    result.push((dials[index] ?? 0) - (dials[index - 1] ?? 0));
  }

  return result;
}

function hellos(sent: readonly ClientFrame[]) {
  return sent.filter((frame): frame is Extract<ClientFrame, { t: "hello" }> => frame.t === "hello");
}

function typingFrames(sent: readonly ClientFrame[]) {
  return sent.filter(
    (frame): frame is Extract<ClientFrame, { t: "typing" }> => frame.t === "typing",
  );
}

const activityItem: ActivityItem = {
  id: 40,
  eventType: "mention",
  state: "unread",
  readAt: null,
  handledAt: null,
  createdAt: "2026-10-06T09:00:00Z",
  updatedAt: "2026-10-06T09:00:00Z",
  source: {
    sourceType: "message",
    sourceId: 900,
    roomId: 12,
    threadId: null,
    messageId: 900,
    eventId: null,
    creatorId: 8,
    title: "general",
    body: "@you have a look?",
    occurredAt: "2026-10-06T09:00:00Z",
    approvalStatus: null,
    budgetCap: null,
    path: "/rooms/12/@900",
  },
};

const unreadCount = (roomId: number) => store.getState().sidebar.rows[roomId]?.unreadCount;

const timelineIds = (roomId: number) => store.getState().timelines[roomId]?.ids;

beforeEach(() => {
  mutations.reset();
  mutations.setMe(meFixture);
  sessionStorage.clear();
});

describe("reconnecting", () => {
  it.effect("retains a confirmed read interrupted while the socket reconnects", () =>
    withSync(
      Effect.gen(function* () {
        yield* serve([]);
        const fake = yield* FakeApi;
        const socket = yield* MemorySocket;
        const firstStarted = yield* Deferred.make<void>();
        const secondStarted = yield* Deferred.make<void>();
        const secondRelease = yield* Deferred.make<void>();
        const secondItem = { ...activityItem, id: 41 };

        const read: ActivityItem = {
          ...activityItem,
          state: "read",
          readAt: "2026-10-06T09:01:00Z",
          updatedAt: "2026-10-06T09:01:00Z",
        };

        yield* fake.reply("GET /activity/unread_count", { unreadCount: 5, unreadRevision: 1 });
        yield* startEngine;
        yield* welcome(10, false);
        mutations.landActivityPage(
          "all",
          "unread",
          {
            items: [activityItem, secondItem],
            users: [],
            unreadCount: 5,
            unreadRevision: 1,
            nextCursor: null,
          },
          "replace",
        );
        yield* fake.route("PATCH /activity/40", () =>
          Deferred.succeed(firstStarted, undefined).pipe(Effect.andThen(Effect.never)),
        );
        yield* fake.route("PATCH /activity/41", () =>
          Deferred.succeed(secondStarted, undefined).pipe(
            Effect.andThen(Deferred.await(secondRelease)),
            Effect.andThen(Effect.fail(new NetworkError({ message: "Connection lost" }))),
          ),
        );
        const first = yield* Effect.forkChild(activity.setState(40, "read"));

        yield* Deferred.await(firstStarted);
        expect(store.getState().activity.unreadCount).toBe(4);
        const second = yield* Effect.forkChild(Effect.exit(activity.setState(41, "read")));

        yield* Deferred.await(secondStarted);
        expect(store.getState().activity.unreadCount).toBe(3);
        yield* pushEvents({
          seq: 11,
          topic: "user",
          type: "activity.item",
          data: { item: read, unreadCount: 4, unreadRevision: 2 },
        });
        expect(store.getState().activity.unreadCount).toBe(3);
        yield* socket.drop;
        yield* Fiber.interrupt(first);
        expect(store.getState().activity.unreadCount).toBe(3);
        expect(store.getState().activity.items[40]).toEqual(read);

        // The welcome abandons B's unconfirmed optimism and installs A's authoritative count.
        yield* fake.reply("GET /activity/unread_count", { unreadCount: 4, unreadRevision: 2 });
        yield* TestClock.adjust("250 millis");
        yield* welcome(11, true);
        expect(store.getState().activity.unreadCount).toBe(4);
        expect(store.getState().activity.pendingUnread).toEqual({});
        expect(store.getState().activity.items[40]).toEqual(read);
        expect(store.getState().activity.items[41]).toBe(secondItem);
        yield* Deferred.succeed(secondRelease, undefined);
        expect((yield* Fiber.join(second))._tag).toBe("Failure");
        expect(store.getState().activity.unreadCount).toBe(4);
        expect(store.getState().activity.items[40]).toEqual(read);
      }),
    ),
  );

  it.effect("processes replay and live events after a reconnect count refresh times out", () =>
    withSync(
      Effect.gen(function* () {
        yield* serve([]);
        const fake = yield* FakeApi;
        const socket = yield* MemorySocket;
        const started = yield* Deferred.make<void>();
        const cancelled = yield* Deferred.make<void>();

        yield* fake.reply("GET /activity/unread_count", { unreadCount: 5, unreadRevision: 1 });
        yield* startEngine;
        yield* welcome(10, false);
        yield* fake.route("GET /activity/unread_count", () =>
          Deferred.succeed(started, undefined).pipe(
            Effect.andThen(Effect.never),
            Effect.ensuring(Deferred.succeed(cancelled, undefined)),
          ),
        );
        yield* socket.drop;
        yield* TestClock.adjust("250 millis");
        yield* welcome(11, true);
        yield* Deferred.await(started);
        yield* pushEvents(
          {
            seq: 11,
            topic: "user",
            type: "activity.item",
            data: { item: activityItem, unreadCount: 6, unreadRevision: 2 },
          },
          unreadEvent(12),
        );
        expect(store.getState().activity.unreadCount).toBe(6);
        yield* TestClock.adjust("15 seconds");
        yield* settle;
        expect(store.getState().activity.unreadCount).toBe(6);
        expect(store.getState().activity.items[40]).toEqual(activityItem);
        expect(unreadCount(12)).toBe(1);
        expect(yield* Deferred.isDone(cancelled)).toBe(true);
        yield* pushEvents({
          seq: 13,
          topic: "user",
          type: "activity.item",
          data: { item: { ...activityItem, id: 41 }, unreadCount: 7, unreadRevision: 3 },
        });
        expect(store.getState().activity.unreadCount).toBe(7);
        expect(store.getState().activity.items[41]).toBeDefined();
      }),
    ),
  );

  for (const resumed of [true, false]) {
    it.effect(
      `processes frames while the reconnect count refresh is pending (resumed=${resumed})`,
      () =>
        withSync(
          Effect.gen(function* () {
            yield* serve([]);
            const fake = yield* FakeApi;
            const socket = yield* MemorySocket;
            const started = yield* Deferred.make<void>();
            const release = yield* Deferred.make<void>();

            yield* fake.reply("GET /activity/unread_count", { unreadCount: 5, unreadRevision: 1 });
            yield* startEngine;
            yield* welcome(10, false);
            yield* fake.route("GET /activity/unread_count", () =>
              Deferred.succeed(started, undefined).pipe(
                Effect.andThen(Deferred.await(release)),
                Effect.as({ unreadCount: 6, unreadRevision: 2 }),
              ),
            );
            yield* socket.drop;
            yield* TestClock.adjust("250 millis");
            yield* welcome(11, resumed);
            yield* Deferred.await(started);
            yield* pushEvents(
              {
                seq: 12,
                topic: "user",
                type: "activity.item",
                data: { item: activityItem, unreadCount: 6, unreadRevision: 2 },
              },
              unreadEvent(13),
            );
            expect(store.getState().activity.unreadCount).toBe(6);
            expect(store.getState().activity.items[40]).toEqual(activityItem);
            expect(unreadCount(12)).toBe(1);
            yield* pushEvents({
              seq: 14,
              topic: "user",
              type: "activity.item",
              data: { item: { ...activityItem, id: 41 }, unreadCount: 7, unreadRevision: 3 },
            });
            expect(store.getState().activity.unreadCount).toBe(7);
            expect(yield* Deferred.isDone(release)).toBe(false);
            yield* Deferred.succeed(release, undefined);
            yield* settle;
            expect(store.getState().activity.unreadCount).toBe(7);
          }),
        ),
    );
  }

  it.effect("ignores replay coverage from a count refresh in an old generation", () =>
    withSync(
      Effect.gen(function* () {
        yield* serve([]);
        const fake = yield* FakeApi;
        const socket = yield* MemorySocket;
        const started = yield* Deferred.make<void>();
        const release = yield* Deferred.make<void>();

        yield* fake.reply("GET /activity/unread_count", { unreadCount: 5, unreadRevision: 1 });
        yield* startEngine;
        yield* welcome(10, false);
        yield* fake.route("GET /activity/unread_count", () =>
          Deferred.succeed(started, undefined).pipe(
            Effect.andThen(Deferred.await(release)),
            Effect.as({ unreadCount: 9, unreadRevision: 3 }),
          ),
        );
        yield* socket.drop;
        yield* TestClock.adjust("250 millis");
        yield* welcome(100, true);
        yield* Deferred.await(started);
        yield* socket.drop;
        yield* TestClock.adjust("500 millis");
        yield* fake.reply("GET /activity/unread_count", { unreadCount: 2, unreadRevision: 1 });
        yield* welcome(10, false, "e2");
        expect(store.getState().activity.unreadCount).toBe(2);
        yield* Deferred.succeed(release, undefined);
        yield* settle;
        expect(store.getState().activity.unreadCount).toBe(2);
        yield* pushEvents({
          seq: 11,
          topic: "user",
          type: "activity.item",
          data: { item: activityItem, unreadCount: 3, unreadRevision: 2 },
        });
        expect(store.getState().activity.unreadCount).toBe(3);
        expect(store.getState().activity.items[40]).toEqual(activityItem);
      }),
    ),
  );

  for (const resumed of [true, false]) {
    it.effect(`reconciles a stalled read on a same-epoch reconnect (resumed=${resumed})`, () =>
      withSync(
        Effect.gen(function* () {
          yield* serve([]);
          const fake = yield* FakeApi;
          const socket = yield* MemorySocket;
          const started = yield* Deferred.make<void>();
          const release = yield* Deferred.make<void>();

          yield* fake.reply("GET /activity/unread_count", { unreadCount: 5, unreadRevision: 1 });
          yield* startEngine;
          yield* welcome(10, false);
          mutations.landActivityPage(
            "all",
            "unread",
            {
              items: [activityItem],
              users: [],
              unreadCount: 5,
              unreadRevision: 1,
              nextCursor: null,
            },
            "replace",
          );
          yield* fake.route("PATCH /activity/40", () =>
            Deferred.succeed(started, undefined).pipe(
              Effect.andThen(Deferred.await(release)),
              Effect.as({
                item: {
                  ...activityItem,
                  state: "read",
                  readAt: "2026-10-06T09:01:00Z",
                  updatedAt: "2026-10-06T09:01:00Z",
                },
                unreadCount: 4,
                unreadRevision: 2,
              }),
            ),
          );
          const changing = yield* Effect.forkChild(activity.setState(40, "read"));

          yield* Deferred.await(started);
          expect(store.getState().activity.unreadCount).toBe(4);
          yield* fake.reply("GET /activity/unread_count", { unreadCount: 6, unreadRevision: 3 });
          yield* activity.loadUnreadCount();
          expect(store.getState().activity.unreadCount).toBe(4);
          yield* socket.drop;
          yield* TestClock.adjust("250 millis");
          yield* welcome(11, resumed);
          expect(store.getState().activity.unreadCount).toBe(6);
          expect(store.getState().activity.pendingUnread).toEqual({});
          expect(store.getState().activity.items[40]).toBe(activityItem);
          expect(activityListOf(store.getState(), "all", "unread").stale).toBe(true);
          const versions = store.getState().activity.versions;

          yield* fake.reply("GET /activity/unread_count", { unreadCount: 5, unreadRevision: 2 });
          yield* activity.loadUnreadCount();
          expect(store.getState().activity.unreadCount).toBe(6);
          expect(store.getState().activity.versions).toBe(versions);
          yield* Deferred.succeed(release, undefined);
          yield* Fiber.join(changing);
          expect(store.getState().activity.unreadCount).toBe(6);
          expect(store.getState().activity.items[40]).toBe(activityItem);
        }),
      ),
    );
  }

  for (const kind of ["page", "tokenless mutation"] as const) {
    it.effect(`fences an old ${kind} on reconnect without pending reads`, () =>
      withSync(
        Effect.gen(function* () {
          yield* serve([]);
          const fake = yield* FakeApi;
          const socket = yield* MemorySocket;
          const started = yield* Deferred.make<void>();
          const release = yield* Deferred.make<void>();

          yield* fake.reply("GET /activity/unread_count", { unreadCount: 5, unreadRevision: 1 });
          yield* startEngine;
          yield* welcome(10, false);
          mutations.landActivityPage(
            "all",
            "unread",
            {
              items: [activityItem],
              users: [],
              unreadCount: 5,
              unreadRevision: 1,
              nextCursor: null,
            },
            "replace",
          );

          const oldItem = { ...activityItem, updatedAt: "2026-10-06T09:01:00Z" };

          const hold = Deferred.succeed(started, undefined).pipe(
            Effect.andThen(Deferred.await(release)),
          );

          if (kind === "page") {
            yield* fake.route("GET /activity", () =>
              hold.pipe(
                Effect.as({
                  items: [oldItem],
                  users: [],
                  unreadCount: 5,
                  unreadRevision: 2,
                  nextCursor: null,
                }),
              ),
            );
          } else {
            yield* fake.route("PATCH /activity/40", () =>
              hold.pipe(
                Effect.as({
                  item: oldItem,
                  unreadCount: 5,
                  unreadRevision: 2,
                }),
              ),
            );
          }

          const oldRequest = yield* Effect.forkChild(
            kind === "page"
              ? activity.load("all", "unread")
              : activity.setState(40, "unhandled").pipe(Effect.asVoid),
          );

          yield* Deferred.await(started);
          expect(store.getState().activity.pendingUnread).toEqual({});
          yield* fake.reply("GET /activity/unread_count", { unreadCount: 5, unreadRevision: 2 });
          yield* socket.drop;
          yield* TestClock.adjust("250 millis");
          yield* welcome(11, true);
          yield* pushEvents({
            seq: 12,
            topic: "user",
            type: "activity.item",
            data: {
              item: { ...activityItem, id: 41, updatedAt: "2026-10-06T09:02:00Z" },
              unreadCount: 6,
              unreadRevision: 3,
            },
          });
          expect(activityListOf(store.getState(), "all", "unread").ids).toEqual([41, 40]);
          yield* Deferred.succeed(release, undefined);
          yield* Fiber.join(oldRequest);
          expect(activityListOf(store.getState(), "all", "unread").ids).toEqual([41, 40]);
          expect(store.getState().activity.items[40]).toBe(activityItem);
          expect(store.getState().activity.unreadCount).toBe(6);
          expect(activityListOf(store.getState(), "all", "unread").stale).toBe(true);
        }),
      ),
    );
  }

  it.effect("replaces a fenced count request on a resumed reconnect without pending reads", () =>
    withSync(
      Effect.gen(function* () {
        yield* serve([]);
        const fake = yield* FakeApi;
        const socket = yield* MemorySocket;
        const started = yield* Deferred.make<void>();
        const release = yield* Deferred.make<void>();

        yield* fake.reply("GET /activity/unread_count", { unreadCount: 5, unreadRevision: 1 });
        yield* startEngine;
        yield* welcome(10, false);
        yield* fake.route("GET /activity/unread_count", () =>
          Deferred.succeed(started, undefined).pipe(
            Effect.andThen(Deferred.await(release)),
            Effect.as({ unreadCount: 6, unreadRevision: 2 }),
          ),
        );
        const oldRequest = yield* Effect.forkChild(activity.loadUnreadCount());

        yield* Deferred.await(started);
        yield* fake.reply("GET /activity/unread_count", { unreadCount: 7, unreadRevision: 3 });
        yield* socket.drop;
        yield* TestClock.adjust("250 millis");
        yield* welcome(10, true);
        expect(store.getState().activity.pendingUnread).toEqual({});
        expect(store.getState().activity.unreadCount).toBe(7);
        yield* Deferred.succeed(release, undefined);
        yield* Fiber.join(oldRequest);
        expect(store.getState().activity.unreadCount).toBe(7);
      }),
    ),
  );

  it.effect("waits 250 ms, doubling to a 30 s cap; a minute online resets the backoff", () =>
    withSync(
      Effect.gen(function* () {
        const socket = yield* MemorySocket;

        yield* socket.setReachable(false);
        yield* startEngine;

        for (const delay of [250, 500, 1000, 2000, 4000, 8000, 16_000, 30_000, 30_000]) {
          yield* TestClock.adjust(delay);
        }

        expect(gaps(yield* socket.dials)).toEqual([
          250, 500, 1000, 2000, 4000, 8000, 16_000, 30_000, 30_000,
        ]);
        expect(store.getState().connection).toBe("reconnecting");

        yield* socket.setReachable(true);
        yield* TestClock.adjust(30_000);

        expect(yield* socket.isOpen).toBe(true);
        expect(store.getState().connection).toBe("online");

        // Pings keep a quiet connection alive past the 30 s silence limit.
        for (let ping = 0; ping < 3; ping++) {
          yield* TestClock.adjust(20_000);
          yield* socket.push({ t: "ping" });
        }

        yield* settle;

        expect(yield* socket.isOpen).toBe(true);

        const droppedAt = yield* Clock.currentTimeMillis;

        yield* socket.drop;
        yield* TestClock.adjust(250);

        expect((yield* socket.dials).at(-1)).toBe(droppedAt + 250);

        // A short-lived connection doesn't reset it: the next wait doubles.
        const droppedAgainAt = yield* Clock.currentTimeMillis;

        yield* socket.drop;
        yield* TestClock.adjust(500);

        expect((yield* socket.dials).at(-1)).toBe(droppedAgainAt + 500);
      }),
    ),
  );

  it.effect("dials at once when the browser comes back online or the tab is shown", () =>
    withSync(
      Effect.gen(function* () {
        const socket = yield* MemorySocket;
        const lifecycle = yield* TestLifecycle;

        yield* socket.setReachable(false);
        yield* startEngine;

        for (const delay of [250, 500, 1000, 2000]) {
          yield* TestClock.adjust(delay);
        }

        const before = (yield* socket.dials).length;
        const onlineAt = yield* Clock.currentTimeMillis;

        yield* lifecycle.fire("online");
        yield* Effect.yieldNow;

        expect((yield* socket.dials).length).toBe(before + 1);
        expect((yield* socket.dials).at(-1)).toBe(onlineAt);

        yield* lifecycle.fire("hidden");
        yield* Effect.yieldNow;

        expect((yield* socket.dials).length).toBe(before + 1);

        yield* lifecycle.fire("visible");
        yield* Effect.yieldNow;

        expect((yield* socket.dials).length).toBe(before + 2);
      }),
    ),
  );

  it.effect("treats 30 s without a frame as a dead socket", () =>
    withSync(
      Effect.gen(function* () {
        const socket = yield* MemorySocket;

        yield* startEngine;
        yield* TestClock.adjust(29_000);

        expect(yield* socket.isOpen).toBe(true);
        expect(yield* socket.dials).toEqual([0]);

        yield* TestClock.adjust(1_000);

        expect(store.getState().connection).toBe("reconnecting");

        yield* TestClock.adjust(250);

        expect(yield* socket.dials).toEqual([0, 30_250]);
        expect(yield* socket.isOpen).toBe(true);
      }),
    ),
  );

  it.effect("stops for good on bye{reconnect:false}", () =>
    withSync(
      Effect.gen(function* () {
        const socket = yield* MemorySocket;

        yield* startEngine;
        yield* socket.push({ t: "bye", reconnect: false, reason: "signed out" });
        yield* TestClock.adjust(60_000);

        expect(store.getState().connection).toBe("offline");
        expect(yield* socket.dials).toEqual([0]);
      }),
    ),
  );
});

describe("resuming", () => {
  it.effect("says hello with the cursor and applies a replayed event only once", () =>
    withSync(
      Effect.gen(function* () {
        const socket = yield* MemorySocket;

        yield* serve([]);
        yield* startEngine;
        yield* welcome(10, false);
        yield* pushEvents(unreadEvent(11), unreadEvent(12));

        expect(unreadCount(12)).toBe(2);
        expect(sessionStorage.getItem(CURSOR_STORAGE_KEY)).toBe(
          JSON.stringify({ epoch: "e1", seq: 12 }),
        );

        yield* socket.clearSent;
        yield* socket.drop;
        yield* TestClock.adjust(250);

        expect(hellos(yield* socket.sent)).toEqual([
          { t: "hello", v: 1, resume: { epoch: "e1", seq: 12 }, topics: [] },
        ]);

        yield* welcome(12, true);
        yield* pushEvents(unreadEvent(11), unreadEvent(12), unreadEvent(13), unreadEvent(13));

        expect(unreadCount(12)).toBe(3);
      }),
    ),
  );

  it.effect("starts from the stored cursor after a reload", () =>
    Effect.gen(function* () {
      sessionStorage.setItem(CURSOR_STORAGE_KEY, JSON.stringify({ epoch: "e1", seq: 40 }));

      yield* withSync(
        Effect.gen(function* () {
          const socket = yield* MemorySocket;

          yield* startEngine;

          expect(hellos(yield* socket.sent)[0]?.resume).toEqual({ epoch: "e1", seq: 40 });
        }),
      );
    }),
  );
  it.effect("doesn't count replayed unreads twice on a sidebar loaded after a reload", () =>
    Effect.gen(function* () {
      sessionStorage.setItem(CURSOR_STORAGE_KEY, JSON.stringify({ epoch: "e1", seq: 40 }));

      yield* withSync(
        Effect.gen(function* () {
          const api = yield* FakeApi;

          const counted = (unread: number) =>
            sidebarFixture([{ ...sidebarRowFixture(12, "general"), unreadCount: unread }]);

          // The page loaded the sidebar with 41 and 42 already counted.
          yield* serve([]);
          mutations.loadSidebar(counted(2));
          // By the time the socket says welcome, 43 has happened too.
          yield* api.reply("GET /sidebar", counted(3));
          yield* startEngine;
          yield* welcome(43, true);

          expect(unreadCount(12)).toBe(3);

          yield* pushEvents(unreadEvent(41), unreadEvent(42), unreadEvent(43));

          expect(unreadCount(12)).toBe(3);
          expect(sessionStorage.getItem(CURSOR_STORAGE_KEY)).toBe(
            JSON.stringify({ epoch: "e1", seq: 43 }),
          );

          yield* pushEvents(unreadEvent(44));

          expect(unreadCount(12)).toBe(4);
        }),
      );
    }),
  );

  it.effect("excludes activity items and removals already covered by the initial snapshot", () =>
    Effect.gen(function* () {
      sessionStorage.setItem(CURSOR_STORAGE_KEY, JSON.stringify({ epoch: "e1", seq: 40 }));

      yield* withSync(
        Effect.gen(function* () {
          const api = yield* FakeApi;

          yield* serve([]);
          mutations.landActivityPage(
            "all",
            "unread",
            {
              items: [activityItem],
              users: [],
              unreadCount: 2,
              unreadRevision: 2,
              nextCursor: null,
            },
            "replace",
          );
          yield* api.reply("GET /activity/unread_count", { unreadCount: 2, unreadRevision: 2 });
          yield* startEngine;
          yield* welcome(43, true);
          yield* pushEvents(
            {
              seq: 41,
              topic: "user",
              type: "activity.item",
              data: { item: { ...activityItem, id: 41 }, unreadCount: 3, unreadRevision: 0 },
            },
            {
              seq: 42,
              topic: "user",
              type: "activity.removed",
              data: { id: 40, unreadCount: 1, unreadRevision: 1 },
            },
          );

          expect(store.getState().activity.items[40]).toBeDefined();
          expect(store.getState().activity.items[41]).toBeUndefined();
          expect(store.getState().activity.unreadCount).toBe(2);

          yield* pushEvents(
            {
              seq: 44,
              topic: "user",
              type: "activity.item",
              data: { item: { ...activityItem, id: 41 }, unreadCount: 3, unreadRevision: 3 },
            },
            {
              seq: 45,
              topic: "user",
              type: "activity.removed",
              data: { id: 40, unreadCount: 2, unreadRevision: 4 },
            },
          );

          expect(store.getState().activity.items[40]).toBeUndefined();
          expect(store.getState().activity.items[41]).toBeDefined();
          expect(store.getState().activity.serverUnread).toEqual({
            unreadCount: 2,
            unreadRevision: 4,
          });
        }),
      );
    }),
  );

  it.effect("applies activity replay counts when the initial count refresh fails", () =>
    Effect.gen(function* () {
      sessionStorage.setItem(CURSOR_STORAGE_KEY, JSON.stringify({ epoch: "e1", seq: 40 }));

      yield* withSync(
        Effect.gen(function* () {
          const api = yield* FakeApi;

          yield* serve([]);
          mutations.landActivityPage(
            "all",
            "unread",
            {
              items: [activityItem],
              users: [],
              unreadCount: 1,
              unreadRevision: 1,
              nextCursor: null,
            },
            "replace",
          );
          yield* api.route("GET /activity/unread_count", () =>
            Effect.fail(new ServerError({ status: 500, message: "Count unavailable" })),
          );
          yield* startEngine;
          yield* welcome(41, true);
          yield* pushEvents({
            seq: 41,
            topic: "user",
            type: "activity.item",
            data: {
              item: {
                ...activityItem,
                state: "read",
                readAt: "2026-10-06T10:00:00Z",
                updatedAt: "2026-10-06T10:00:00Z",
              },
              unreadCount: 0,
              unreadRevision: 2,
            },
          });

          expect(store.getState().activity.items[40]?.state).toBe("read");
          expect(store.getState().activity.serverUnread).toEqual({
            unreadCount: 0,
            unreadRevision: 2,
          });
        }),
      );
    }),
  );

  for (const refreshFails of [false, true]) {
    it.effect(
      `accepts a LOWER activity revision after a restore when refresh ${refreshFails ? "fails" : "succeeds"}`,
      () =>
        withSync(
          Effect.gen(function* () {
            const api = yield* FakeApi;
            const socket = yield* MemorySocket;

            yield* serve([]);
            yield* api.reply("GET /activity/unread_count", { unreadCount: 8, unreadRevision: 100 });
            yield* startEngine;
            yield* welcome(10, false);
            expect(store.getState().activity.unreadCount).toBe(8);

            yield* socket.drop;
            yield* TestClock.adjust(250);

            if (refreshFails) {
              yield* api.route("GET /activity/unread_count", () =>
                Effect.fail(new ServerError({ status: 500, message: "Count unavailable" })),
              );
            } else {
              yield* api.reply("GET /activity/unread_count", {
                unreadCount: 2,
                unreadRevision: 50,
              });
            }

            yield* welcome(0, false, "restored");
            expect(store.getState().activity.unreadCount).toBe(refreshFails ? 8 : 2);
            yield* pushEvents({
              seq: 1,
              topic: "user",
              type: "activity.item",
              data: { item: activityItem, unreadCount: 3, unreadRevision: 51 },
            });

            expect(store.getState().activity.serverUnread).toEqual({
              unreadCount: 3,
              unreadRevision: 51,
            });
          }),
        ),
    );

    it.effect(
      `accepts activity events after an epoch restart when refresh ${refreshFails ? "fails" : "succeeds"}`,
      () =>
        Effect.gen(function* () {
          sessionStorage.setItem(CURSOR_STORAGE_KEY, JSON.stringify({ epoch: "e1", seq: 40 }));

          yield* withSync(
            Effect.gen(function* () {
              const api = yield* FakeApi;
              const socket = yield* MemorySocket;

              yield* serve([]);
              yield* api.reply("GET /activity/unread_count", { unreadCount: 2, unreadRevision: 2 });
              yield* startEngine;
              yield* welcome(43, true);
              yield* socket.drop;
              yield* TestClock.adjust(250);

              if (refreshFails) {
                yield* api.route("GET /activity/unread_count", () =>
                  Effect.fail(new ServerError({ status: 500, message: "Count unavailable" })),
                );
              }

              yield* welcome(0, false, "e2");
              yield* pushEvents({
                seq: 1,
                topic: "user",
                type: "activity.item",
                data: { item: activityItem, unreadCount: 3, unreadRevision: 3 },
              });

              expect(store.getState().activity.items[40]).toBeDefined();
              expect(store.getState().activity.serverUnread).toEqual({
                unreadCount: 3,
                unreadRevision: 3,
              });
            }),
          );
        }),
    );
  }

  for (const kind of ["count GET", "page", "mutation"] as const) {
    for (const beforeSnapshot of [true, false]) {
      it.effect(
        `fences an old ${kind} ${beforeSnapshot ? "before" : "after"} the restored count snapshot`,
        () =>
          withSync(
            Effect.gen(function* () {
              const api = yield* FakeApi;
              const socket = yield* MemorySocket;
              const oldStarted = yield* Deferred.make<void>();
              const oldRelease = yield* Deferred.make<void>();
              const freshStarted = yield* Deferred.make<void>();
              const freshRelease = yield* Deferred.make<void>();

              yield* serve([]);
              yield* api.reply("GET /activity/unread_count", {
                unreadCount: 8,
                unreadRevision: 100,
              });
              yield* startEngine;
              yield* welcome(10, false);
              mutations.landActivityPage(
                "all",
                "unread",
                {
                  items: [activityItem],
                  users: [],
                  unreadCount: 8,
                  unreadRevision: 100,
                  nextCursor: null,
                },
                "replace",
              );

              const oldCount = { unreadCount: 9, unreadRevision: 101 };

              const oldItem: ActivityItem = {
                ...activityItem,
                state: "read",
                readAt: "2026-10-06T10:00:00Z",
                updatedAt: "2026-10-06T10:00:00Z",
              };

              const hold = Deferred.succeed(oldStarted, undefined).pipe(
                Effect.andThen(Deferred.await(oldRelease)),
              );

              if (kind === "count GET") {
                yield* api.route("GET /activity/unread_count", () =>
                  hold.pipe(Effect.as(oldCount)),
                );
              } else if (kind === "page") {
                yield* api.route("GET /activity", () =>
                  hold.pipe(
                    Effect.as({ ...oldCount, items: [oldItem], users: [], nextCursor: null }),
                  ),
                );
              } else {
                yield* api.route("PATCH /activity/40", () =>
                  hold.pipe(Effect.as({ ...oldCount, item: oldItem })),
                );
              }

              const requests = {
                "count GET": activity.loadUnreadCount().pipe(Effect.asVoid),
                page: activity.load("all", "unread"),
                mutation: activity.setState(40, "read").pipe(Effect.asVoid),
              };

              const oldRequest = yield* Effect.forkChild(requests[kind]);

              yield* Deferred.await(oldStarted);
              yield* api.route("GET /activity/unread_count", () =>
                Deferred.succeed(freshStarted, undefined).pipe(
                  Effect.andThen(Deferred.await(freshRelease)),
                  Effect.as({ unreadCount: 2, unreadRevision: 50 }),
                ),
              );
              yield* socket.drop;
              yield* TestClock.adjust(250);
              yield* welcome(0, false, "restored");
              yield* Deferred.await(freshStarted);

              if (beforeSnapshot) {
                yield* Deferred.succeed(oldRelease, undefined);
                yield* Fiber.join(oldRequest);
                expect(store.getState().activity.unreadCount).toBe(8);
              }

              yield* Deferred.succeed(freshRelease, undefined);
              yield* settle;
              expect(store.getState().activity.unreadCount).toBe(2);
              yield* api.reply("GET /activity", {
                items: [activityItem],
                users: [],
                unreadCount: 2,
                unreadRevision: 50,
                nextCursor: null,
              });
              yield* activity.load("all", "unread");

              if (!beforeSnapshot) {
                yield* Deferred.succeed(oldRelease, undefined);
                yield* Fiber.join(oldRequest);
              }

              expect(store.getState().activity.items[40]?.state).toBe("unread");
              expect(store.getState().activity.serverUnread).toEqual({
                unreadCount: 2,
                unreadRevision: 50,
              });
            }),
          ),
      );
    }
  }

  it.effect("applies replays normally on a later reconnect", () =>
    Effect.gen(function* () {
      sessionStorage.setItem(CURSOR_STORAGE_KEY, JSON.stringify({ epoch: "e1", seq: 40 }));

      yield* withSync(
        Effect.gen(function* () {
          const api = yield* FakeApi;
          const socket = yield* MemorySocket;

          yield* serve([]);
          mutations.loadSidebar(sidebarFixture([sidebarRowFixture(12, "general")]));
          yield* startEngine;
          yield* welcome(40, true);
          yield* socket.drop;
          yield* TestClock.adjust(250);

          const before = (yield* api.requests).length;

          yield* welcome(42, true);
          yield* pushEvents(unreadEvent(41), unreadEvent(42));

          expect((yield* api.requests).slice(before)).toEqual([
            { method: "GET", path: "/activity/unread_count" },
          ]);
          expect(unreadCount(12)).toBe(2);
        }),
      );
    }),
  );
});

/** Thread 88's header in room 12. */
const threadDetail = (status: "active" | "closed", canClose: boolean) => ({
  thread: {
    id: 88,
    roomId: 12,
    parentMessageId: 1,
    creatorId: 7,
    name: "Plans",
    status,
    replyCount: 2,
    lastActivityAt: "2026-10-06T00:00:10.000Z",
    autoArchiveAfterMinutes: 4320,
    createdAt: "2026-10-06T00:00:01.000Z",
    work: null,
  },
  membership: null,
  parentMessage: null,
  permissions: {
    canRename: true,
    canClose,
    canReopen: !canClose,
    canLock: true,
    canUnlock: false,
    canDelete: true,
    canConvertWork: false,
    canManageWork: false,
    canUpdateWorkStatus: false,
    canAssignWork: false,
    canRemoveWork: false,
  },
  work: null,
  users: [],
});

describe("resync", () => {
  it.effect("refetches the sidebar and subscribed rooms when the server can't resume", () =>
    withSync(
      Effect.gen(function* () {
        const socket = yield* MemorySocket;
        const api = yield* FakeApi;

        yield* serve([messageFixture(1, 12), messageFixture(2, 12)]);
        yield* startEngine;
        yield* welcome(5, false);
        yield* session.openRoom(12, null);

        expect(timelineIds(12)).toEqual([1, 2]);
        expect(yield* socket.sent).toContainEqual({ t: "sub", topics: ["room:12"] });

        yield* api.reply(
          "GET /rooms/12/messages",
          pageFixture([1, 2, 3].map((id) => messageFixture(id, 12))),
        );
        yield* socket.drop;
        yield* TestClock.adjust(250);

        expect(hellos(yield* socket.sent).at(-1)?.topics).toEqual(["room:12"]);

        const before = (yield* api.requests).length;

        yield* welcome(1, false, "e2");

        const requests = (yield* api.requests).slice(before);

        expect(requests).toHaveLength(4);
        expect(requests).toEqual(
          expect.arrayContaining([
            { method: "GET", path: "/sidebar" },
            { method: "GET", path: "/activity/unread_count" },
            { method: "GET", path: "/rooms/12" },
            { method: "GET", path: "/rooms/12/messages" },
          ]),
        );
        expect(timelineIds(12)).toEqual([1, 2, 3]);
        expect(sessionStorage.getItem(CURSOR_STORAGE_KEY)).toBe(
          JSON.stringify({ epoch: "e2", seq: 1 }),
        );
      }),
    ),
  );

  it.effect("tells features the topics it resyncs, on a fresh welcome and a resync frame", () =>
    withSync(
      Effect.gen(function* () {
        const socket = yield* MemorySocket;
        const told: (readonly string[])[] = [];
        const stop = onResync((topics) => told.push(topics));

        yield* serve([messageFixture(1, 12)]);
        yield* startEngine;
        yield* welcome(5, false);
        yield* session.openRoom(12, null);
        yield* socket.drop;
        yield* TestClock.adjust(250);
        yield* welcome(1, false, "e2");
        yield* socket.push({ t: "resync", topics: ["room:12"], reason: "lagged" });
        yield* settle;
        stop();

        expect(told).toEqual([["user"], ["user", "room:12"], ["room:12"]]);
      }),
    ),
  );

  it.effect("refetches only the topics a resync frame names", () =>
    withSync(
      Effect.gen(function* () {
        const socket = yield* MemorySocket;
        const api = yield* FakeApi;

        yield* serve([messageFixture(1, 12)]);
        yield* startEngine;
        yield* welcome(5, false);
        yield* session.openRoom(12, null);
        yield* api.reply(
          "GET /rooms/12/messages",
          pageFixture([1, 2].map((id) => messageFixture(id, 12))),
        );

        const before = (yield* api.requests).length;

        yield* socket.push({ t: "resync", topics: ["room:12"], reason: "lagged" });
        yield* settle;

        expect((yield* api.requests).slice(before)).toEqual([
          { method: "GET", path: "/rooms/12" },
          { method: "GET", path: "/rooms/12/messages" },
        ]);
        expect(timelineIds(12)).toEqual([1, 2]);
      }),
    ),
  );

  it.effect("refetches a subscribed thread's header and replies", () =>
    withSync(
      Effect.gen(function* () {
        const socket = yield* MemorySocket;
        const api = yield* FakeApi;

        const replies = (ids: readonly number[]) =>
          pageFixture(ids.map((id) => messageFixture(id, 12, { threadId: 88 })));

        yield* serve([]);
        yield* api.reply("GET /threads/88", threadDetail("active", true));
        yield* api.reply("GET /threads/88/messages", replies([5, 6]));
        yield* startEngine;
        yield* welcome(5, false);
        yield* threadActions.open(88);

        expect(store.getState().threadTimelines[88]?.ids).toEqual([5, 6]);

        // Closed and one reply deleted while the events were lost.
        yield* api.reply("GET /threads/88", threadDetail("closed", false));
        yield* api.reply("GET /threads/88/messages", replies([5, 7]));
        yield* socket.push({ t: "resync", topics: ["thread:88"], reason: "lagged" });
        yield* settle;

        expect(store.getState().threads[88]?.status).toBe("closed");
        expect(store.getState().threadPanes[88]?.permissions?.canClose).toBe(false);
        expect(store.getState().threadTimelines[88]?.ids).toEqual([5, 7]);
      }),
    ),
  );

  it.effect("installs a resynced thread's header before 501 removals land during its replies", () =>
    withSync(
      Effect.gen(function* () {
        const socket = yield* MemorySocket;
        const api = yield* FakeApi;
        const entered = yield* Deferred.make<void>();
        const release = yield* Deferred.make<void>();
        let hold = false;

        yield* serve([]);
        yield* api.route("GET /threads/1/messages", () =>
          hold
            ? Effect.andThen(
                Effect.andThen(Deferred.succeed(entered, undefined), Deferred.await(release)),
                Effect.succeed(pageFixture([])),
              )
            : Effect.succeed(pageFixture([])),
        );
        yield* startEngine;
        yield* welcome(5, false);
        // The pane is open but holds no thread: its first load failed.
        yield* threadActions.open(1);
        expect(store.getState().threads[1]).toBeUndefined();

        yield* api.reply("GET /threads/1", boardDetail());
        hold = true;
        yield* socket.push({ t: "resync", topics: ["thread:1"], reason: "lagged" });
        yield* settle;
        yield* Deferred.await(entered);
        yield* settle;
        mutations.applyEvents(
          Array.from({ length: MAX_REMOVED_THREADS + 1 }, (_, index) => ({
            seq: 100 + index,
            topic: `room:${BOARD}`,
            type: "thread.removed" as const,
            data: { threadId: 20_000 + index, roomId: BOARD },
          })),
          0,
        );
        yield* Deferred.succeed(release, undefined);
        yield* settle;

        expect(store.getState().threads[1]?.name).toBe("Post 1");
        expect(store.getState().threadPanes[1]?.status).toBe("ready");
      }),
    ),
  );

  it.effect(
    "leaves a resynced pane's replies failed, for the timeline to retry, when they don't load",
    () =>
      withSync(
        Effect.gen(function* () {
          const socket = yield* MemorySocket;
          const api = yield* FakeApi;

          yield* serve([]);
          yield* startEngine;
          yield* welcome(5, false);
          // The first open fails outright: no header, no replies.
          yield* threadActions.open(1);
          expect(store.getState().threadPanes[1]?.status).toBe("error");

          // The resync gets the header, but the replies still fail.
          yield* api.reply("GET /threads/1", boardDetail());
          yield* api.route("GET /threads/1/messages", () =>
            Effect.fail(new ServerError({ status: 500, message: "boom" })),
          );
          yield* socket.push({ t: "resync", topics: ["thread:1"], reason: "lagged" });
          yield* settle;

          expect(store.getState().threadPanes[1]?.status).toBe("ready");
          expect(store.getState().threadTimelines[1]?.status).toBe("error");
        }),
      ),
  );

  it.effect("keeps the replies a resync installed when an older Try again answers after it", () =>
    withSync(
      Effect.gen(function* () {
        const socket = yield* MemorySocket;
        const api = yield* FakeApi;
        const retried = yield* Deferred.make<void>();
        const release = yield* Deferred.make<void>();
        let calls = 0;

        yield* serve([]);
        yield* api.reply("GET /threads/1", boardDetail());
        yield* api.route("GET /threads/1/messages", () => {
          calls += 1;

          if (calls === 1) {
            return Effect.fail(new ServerError({ status: 500, message: "boom" }));
          }

          // The Try again's snapshot is taken first and answers last.
          return calls === 2
            ? Effect.andThen(
                Effect.andThen(Deferred.succeed(retried, undefined), Deferred.await(release)),
                Effect.succeed(threadReplies([5])),
              )
            : Effect.succeed(threadReplies([5, 6, 7]));
        });
        yield* startEngine;
        yield* welcome(5, false);
        yield* threadActions.open(1);
        expect(store.getState().threadTimelines[1]?.status).toBe("error");

        const retry = yield* Effect.forkChild(threadActions.reload(1));

        yield* Deferred.await(retried);
        yield* socket.push({ t: "resync", topics: ["thread:1"], reason: "lagged" });
        yield* settle;
        expect(store.getState().threadTimelines[1]?.ids).toEqual([5, 6, 7]);

        yield* Deferred.succeed(release, undefined);
        yield* Fiber.join(retry);

        expect(store.getState().threadTimelines[1]?.ids).toEqual([5, 6, 7]);
        expect(store.getState().threadTimelines[1]?.status).toBe("ready");
      }),
    ),
  );

  it.effect("keeps the replies a Try again installed when an older resync answers after it", () =>
    withSync(
      Effect.gen(function* () {
        const socket = yield* MemorySocket;
        const api = yield* FakeApi;
        const resynced = yield* Deferred.make<void>();
        const release = yield* Deferred.make<void>();
        let calls = 0;

        yield* serve([]);
        yield* api.reply("GET /threads/1", boardDetail());
        yield* api.route("GET /threads/1/messages", () => {
          calls += 1;

          if (calls === 1) {
            return Effect.fail(new ServerError({ status: 500, message: "boom" }));
          }

          // The resync's snapshot is taken first and answers last.
          return calls === 2
            ? Effect.andThen(
                Effect.andThen(Deferred.succeed(resynced, undefined), Deferred.await(release)),
                Effect.succeed(threadReplies([5])),
              )
            : Effect.succeed(threadReplies([5, 6, 7]));
        });
        yield* startEngine;
        yield* welcome(5, false);
        yield* threadActions.open(1);
        expect(store.getState().threadTimelines[1]?.status).toBe("error");

        yield* socket.push({ t: "resync", topics: ["thread:1"], reason: "lagged" });
        yield* settle;
        yield* Deferred.await(resynced);
        yield* threadActions.reload(1);
        expect(store.getState().threadTimelines[1]?.ids).toEqual([5, 6, 7]);

        yield* Deferred.succeed(release, undefined);
        yield* settle;

        expect(store.getState().threadTimelines[1]?.ids).toEqual([5, 6, 7]);
        expect(store.getState().threadTimelines[1]?.status).toBe("ready");
      }),
    ),
  );

  it.effect(
    "says why, with Try again, when a resync supersedes the pane's load and both fail",
    () =>
      withSync(
        Effect.gen(function* () {
          const socket = yield* MemorySocket;
          const api = yield* FakeApi;
          const entered = yield* Deferred.make<void>();
          const release = yield* Deferred.make<void>();
          let details = 0;

          yield* serve([]);
          yield* api.reply("GET /threads/1/messages", threadReplies([5]));
          yield* api.route("GET /threads/1", () => {
            details += 1;

            return details === 1
              ? Effect.andThen(
                  Effect.andThen(Deferred.succeed(entered, undefined), Deferred.await(release)),
                  Effect.fail(new ServerError({ status: 500, message: "boom" })),
                )
              : Effect.fail(new ServerError({ status: 503, message: "Try again later" }));
          });
          yield* startEngine;
          yield* welcome(5, false);

          const open = yield* Effect.forkChild(threadActions.open(1));

          yield* Deferred.await(entered);
          expect(store.getState().threadPanes[1]?.status).toBe("loading");
          yield* socket.push({ t: "resync", topics: ["thread:1"], reason: "lagged" });
          yield* settle;
          yield* Deferred.succeed(release, undefined);
          yield* Fiber.join(open);

          expect(store.getState().threadPanes[1]).toMatchObject({
            status: "error",
            error: "Try again later",
          });
        }),
      ),
  );

  it.effect("keeps a Try again's permissions when an older resync's header answers after it", () =>
    withSync(
      Effect.gen(function* () {
        const socket = yield* MemorySocket;
        const api = yield* FakeApi;
        const entered = yield* Deferred.make<void>();
        const release = yield* Deferred.make<void>();
        const fresh = boardDetail();
        const stale = { ...fresh, permissions: { ...fresh.permissions, canClose: false } };
        let details = 0;

        yield* serve([]);
        yield* api.reply("GET /threads/1/messages", threadReplies([5]));
        yield* api.route("GET /threads/1", () => {
          details += 1;

          if (details === 1) {
            return Effect.fail(new ServerError({ status: 500, message: "boom" }));
          }

          // The resync's header is asked for first and answers last, from before the change.
          return details === 2
            ? Effect.andThen(
                Effect.andThen(Deferred.succeed(entered, undefined), Deferred.await(release)),
                Effect.succeed(stale),
              )
            : Effect.succeed(fresh);
        });
        yield* startEngine;
        yield* welcome(5, false);
        yield* threadActions.open(1);
        expect(store.getState().threadPanes[1]?.status).toBe("error");

        yield* socket.push({ t: "resync", topics: ["thread:1"], reason: "lagged" });
        yield* settle;
        yield* Deferred.await(entered);
        yield* threadActions.reload(1);
        expect(store.getState().threadPanes[1]?.permissions?.canClose).toBe(true);

        yield* Deferred.succeed(release, undefined);
        yield* settle;

        expect(store.getState().threadPanes[1]?.permissions?.canClose).toBe(true);
      }),
    ),
  );

  it.effect("keeps a resync's error when the pane's older header answers after it", () =>
    withSync(
      Effect.gen(function* () {
        const socket = yield* MemorySocket;
        const api = yield* FakeApi;
        const entered = yield* Deferred.make<void>();
        const release = yield* Deferred.make<void>();
        let details = 0;

        yield* serve([]);
        yield* api.reply("GET /threads/1/messages", threadReplies([5]));
        yield* api.route("GET /threads/1", () => {
          details += 1;

          return details === 1
            ? Effect.andThen(
                Effect.andThen(Deferred.succeed(entered, undefined), Deferred.await(release)),
                Effect.succeed(boardDetail()),
              )
            : Effect.fail(new ServerError({ status: 503, message: "Try again later" }));
        });
        yield* startEngine;
        yield* welcome(5, false);

        const open = yield* Effect.forkChild(threadActions.open(1));

        yield* Deferred.await(entered);
        yield* socket.push({ t: "resync", topics: ["thread:1"], reason: "lagged" });
        yield* settle;
        expect(store.getState().threadPanes[1]?.error).toBe("Try again later");

        yield* Deferred.succeed(release, undefined);
        yield* Fiber.join(open);

        expect(store.getState().threadPanes[1]).toMatchObject({
          status: "error",
          error: "Try again later",
        });
      }),
    ),
  );

  it.effect("keeps a permalink's reply pending when access goes before its replies load", () =>
    withSync(
      Effect.gen(function* () {
        const socket = yield* MemorySocket;
        const api = yield* FakeApi;
        let details = 0;
        let member = false;

        yield* serve([]);
        // The header answers while the viewer is still a member; everything after is a 404.
        yield* api.route("GET /threads/1", () => {
          details += 1;

          return details === 1 || member
            ? Effect.succeed(boardDetail())
            : Effect.fail(new NotFound({ message: "Not found" }));
        });
        yield* api.route("GET /threads/1/messages", (request) => {
          if (!member) {
            return Effect.fail(new NotFound({ message: "Not found" }));
          }

          return Effect.succeed(
            request.query?.around === "7" ? threadReplies([6, 7, 8]) : threadReplies([20, 21]),
          );
        });
        yield* startEngine;
        yield* welcome(5, false);
        yield* threadActions.open(1, 7);

        // The reply isn't known to be gone, so the pane says why instead of opening elsewhere.
        expect(store.getState().threadPanes[1]?.status).toBe("error");
        expect(store.getState().threadTimelines[1]?.status).toBe("error");

        // Access is back: the next resync still opens around the reply.
        member = true;
        yield* socket.push({ t: "resync", topics: ["thread:1"], reason: "lagged" });
        yield* settle;

        expect(store.getState().threadTimelines[1]?.ids).toEqual([6, 7, 8]);
        expect(store.getState().threadTimelines[1]?.status).toBe("ready");
      }),
    ),
  );

  it.effect("keeps a permalink's reply in view when a resync supersedes its load", () =>
    withSync(
      Effect.gen(function* () {
        const socket = yield* MemorySocket;
        const api = yield* FakeApi;
        const entered = yield* Deferred.make<void>();
        const release = yield* Deferred.make<void>();
        let focused = 0;

        yield* serve([]);
        yield* api.reply("GET /threads/1", boardDetail());
        yield* api.route("GET /threads/1/messages", (request) => {
          if (request.query?.around !== "7") {
            return Effect.succeed(threadReplies([20, 21]));
          }

          focused += 1;

          return focused === 1
            ? Effect.andThen(
                Effect.andThen(Deferred.succeed(entered, undefined), Deferred.await(release)),
                Effect.succeed(threadReplies([6, 7, 8])),
              )
            : Effect.succeed(threadReplies([6, 7, 8]));
        });
        yield* startEngine;
        yield* welcome(5, false);

        const open = yield* Effect.forkChild(threadActions.open(1, 7));

        yield* Deferred.await(entered);
        yield* socket.push({ t: "resync", topics: ["thread:1"], reason: "lagged" });
        yield* settle;
        yield* Deferred.succeed(release, undefined);
        yield* Fiber.join(open);

        expect(store.getState().threadTimelines[1]?.ids).toEqual([6, 7, 8]);
        expect(store.getState().threadPanes[1]?.status).toBe("ready");

        // The reply is in view: a later resync asks for the newest replies again.
        const before = (yield* api.requests).length;

        yield* socket.push({ t: "resync", topics: ["thread:1"], reason: "lagged" });
        yield* settle;

        const pages = (yield* api.requests)
          .slice(before)
          .filter((request) => request.path === "/threads/1/messages");

        expect(pages.length).toBeGreaterThan(0);
        expect(pages[0]?.query).toBeUndefined();
      }),
    ),
  );

  it.effect("opens at the newest replies when a permalink's reply is gone", () =>
    withSync(
      Effect.gen(function* () {
        const socket = yield* MemorySocket;
        const api = yield* FakeApi;

        yield* serve([]);
        yield* api.reply("GET /threads/1", boardDetail());
        yield* api.route("GET /threads/1/messages", (request) =>
          request.query?.around === "7"
            ? Effect.fail(new NotFound({ message: "Not found" }))
            : Effect.succeed(threadReplies([20, 21])),
        );
        yield* startEngine;
        yield* welcome(5, false);
        yield* threadActions.open(1, 7);

        expect(store.getState().threadTimelines[1]?.ids).toEqual([20, 21]);
        expect(store.getState().threadTimelines[1]?.status).toBe("ready");
        expect(store.getState().threadPanes[1]?.status).toBe("ready");

        // The gone reply isn't asked for again.
        const before = (yield* api.requests).length;

        yield* socket.push({ t: "resync", topics: ["thread:1"], reason: "lagged" });
        yield* settle;

        const pages = (yield* api.requests)
          .slice(before)
          .filter((request) => request.path === "/threads/1/messages");

        expect(pages.length).toBeGreaterThan(0);
        expect(pages.every((request) => request.query?.around !== "7")).toBe(true);
      }),
    ),
  );

  it.effect(
    "opens at the newest replies when a resync finds a superseded permalink's reply gone",
    () =>
      withSync(
        Effect.gen(function* () {
          const socket = yield* MemorySocket;
          const api = yield* FakeApi;
          const entered = yield* Deferred.make<void>();
          const release = yield* Deferred.make<void>();
          let focused = 0;

          yield* serve([]);
          yield* api.reply("GET /threads/1", boardDetail());
          yield* api.route("GET /threads/1/messages", (request) => {
            if (request.query?.around !== "7") {
              return Effect.succeed(threadReplies([20, 21]));
            }

            focused += 1;

            return focused === 1
              ? Effect.andThen(
                  Effect.andThen(Deferred.succeed(entered, undefined), Deferred.await(release)),
                  Effect.fail(new NotFound({ message: "Not found" })),
                )
              : Effect.fail(new NotFound({ message: "Not found" }));
          });
          yield* startEngine;
          yield* welcome(5, false);

          const open = yield* Effect.forkChild(threadActions.open(1, 7));

          yield* Deferred.await(entered);
          yield* socket.push({ t: "resync", topics: ["thread:1"], reason: "lagged" });
          yield* settle;
          yield* Deferred.succeed(release, undefined);
          yield* Fiber.join(open);

          expect(store.getState().threadTimelines[1]?.ids).toEqual([20, 21]);
          expect(store.getState().threadTimelines[1]?.status).toBe("ready");

          yield* socket.push({ t: "resync", topics: ["thread:1"], reason: "lagged" });
          yield* settle;
          expect(focused).toBe(2);
        }),
      ),
  );

  it.effect("keeps a permalink's reply pending through a resync that fails to load it", () =>
    withSync(
      Effect.gen(function* () {
        const socket = yield* MemorySocket;
        const api = yield* FakeApi;
        const entered = yield* Deferred.make<void>();
        const release = yield* Deferred.make<void>();
        let focused = 0;

        yield* serve([]);
        yield* api.reply("GET /threads/1", boardDetail());
        yield* api.route("GET /threads/1/messages", (request) => {
          if (request.query?.around !== "7") {
            return Effect.succeed(threadReplies([20, 21]));
          }

          focused += 1;

          if (focused === 1) {
            return Effect.andThen(
              Effect.andThen(Deferred.succeed(entered, undefined), Deferred.await(release)),
              Effect.succeed(threadReplies([6, 7, 8])),
            );
          }

          return focused === 2
            ? Effect.fail(new ServerError({ status: 500, message: "boom" }))
            : Effect.succeed(threadReplies([6, 7, 8]));
        });
        yield* startEngine;
        yield* welcome(5, false);

        const open = yield* Effect.forkChild(threadActions.open(1, 7));

        yield* Deferred.await(entered);
        // The resync takes over the permalink's load, and its page around the reply fails.
        yield* socket.push({ t: "resync", topics: ["thread:1"], reason: "lagged" });
        yield* settle;
        yield* Deferred.succeed(release, undefined);
        yield* Fiber.join(open);
        expect(store.getState().threadTimelines[1]?.status).toBe("error");

        // The next resync still opens around the reply, not at the newest replies.
        yield* socket.push({ t: "resync", topics: ["thread:1"], reason: "lagged" });
        yield* settle;

        expect(focused).toBe(3);
        expect(store.getState().threadTimelines[1]?.ids).toEqual([6, 7, 8]);
        expect(store.getState().threadTimelines[1]?.status).toBe("ready");
      }),
    ),
  );

  it.effect("drops a thread load that answers after a later reload around a reply", () =>
    withSync(
      Effect.gen(function* () {
        const api = yield* FakeApi;
        const newest = yield* Deferred.make<void>();
        const around = yield* Deferred.make<void>();

        const replies = (ids: readonly number[]) =>
          pageFixture(ids.map((id) => messageFixture(id, 12, { threadId: 88 })));

        yield* serve([]);
        yield* api.reply("GET /threads/88", threadDetail("active", true));
        yield* api.route("GET /threads/88/messages", (request) =>
          request.query?.around === "3"
            ? Effect.andThen(Deferred.await(around), Effect.succeed(replies([2, 3, 4])))
            : Effect.andThen(Deferred.await(newest), Effect.succeed(replies([50, 51]))),
        );
        yield* startEngine;
        yield* welcome(5, false);

        // The pane opens on the newest replies; before they arrive, it's read in around reply 3.
        yield* Effect.forkChild(threadActions.open(88));
        yield* settle;
        yield* Effect.forkChild(threadActions.reload(88, 3));
        yield* settle;
        yield* Deferred.succeed(around, undefined);
        yield* settle;

        expect(store.getState().threadTimelines[88]?.ids).toEqual([2, 3, 4]);

        // The newest replies answer last, and are dropped.
        yield* Deferred.succeed(newest, undefined);
        yield* settle;

        expect(store.getState().threadTimelines[88]?.ids).toEqual([2, 3, 4]);
      }),
    ),
  );

  it.effect("re-reads a window that stops short of the present in place, dropping deletions", () =>
    withSync(
      Effect.gen(function* () {
        const socket = yield* MemorySocket;
        const api = yield* FakeApi;

        yield* serve([]);
        yield* api.reply(
          "GET /rooms/12/messages",
          pageFixture(
            [4, 5, 6].map((id) => messageFixture(id, 12)),
            3,
            7,
          ),
        );
        yield* startEngine;
        yield* welcome(5, false);
        yield* session.openRoom(12, 5);

        expect(timelineIds(12)).toEqual([4, 5, 6]);

        // The newest page would yank the reader away; the page around the window's middle (5,
        // since deleted) is re-read instead.
        yield* api.route("GET /rooms/12/messages", (request) =>
          Effect.succeed(
            request.query?.around === "5"
              ? pageFixture(
                  [4, 6].map((id) => messageFixture(id, 12)),
                  3,
                  7,
                )
              : pageFixture(
                  [40, 41].map((id) => messageFixture(id, 12)),
                  39,
                ),
          ),
        );
        yield* socket.push({ t: "resync", topics: ["room:12"], reason: "lagged" });
        yield* settle;

        expect(timelineIds(12)).toEqual([4, 6]);
        expect(store.getState().timelines[12]?.after).toBe(7);
      }),
    ),
  );

  /** Opens room 12 around 5 (4 to 6, more after) and pages down to the present (7, 8). */
  const openAndReachPresent = Effect.gen(function* () {
    const api = yield* FakeApi;

    yield* serve([]);
    yield* api.route("GET /rooms/12/messages", (request) =>
      Effect.succeed(
        request.query?.after === "6"
          ? pageFixture([7, 8].map((id) => messageFixture(id, 12)))
          : pageFixture(
              [4, 5, 6].map((id) => messageFixture(id, 12)),
              4,
              6,
            ),
      ),
    );
    yield* startEngine;
    yield* session.openRoom(12, 5);
    yield* session.loadNewer(12);

    expect(timelineIds(12)).toEqual([4, 5, 6, 7, 8]);
    expect(store.getState().timelines[12]?.after).toBeNull();
  });

  it.effect(
    "merges the first welcome's newest page into a window that paged down to the present",
    () =>
      withSync(
        Effect.gen(function* () {
          const api = yield* FakeApi;

          yield* openAndReachPresent;

          const generation = store.getState().timelines[12]?.generation;

          // Replacing the window with the newest page would drop 4 and 5 and jump the reader.
          yield* api.reply(
            "GET /rooms/12/messages",
            pageFixture(
              [6, 7, 8, 9].map((id) => messageFixture(id, 12)),
              6,
            ),
          );
          yield* welcome(5, false);

          expect(timelineIds(12)).toEqual([4, 5, 6, 7, 8, 9]);
          expect(store.getState().timelines[12]?.before).toBe(4);
          expect(store.getState().timelines[12]?.generation).toBe(generation);
        }),
      ),
  );

  it.effect("keeps a window the newest page no longer meets and re-reads it in place", () =>
    withSync(
      Effect.gen(function* () {
        const api = yield* FakeApi;

        yield* openAndReachPresent;

        const generation = store.getState().timelines[12]?.generation;

        // More was posted than a page holds: the window now stops short of the present, and the
        // page around its middle (6) is re-read, without 5, deleted meanwhile.
        yield* api.route("GET /rooms/12/messages", (request) =>
          Effect.succeed(
            request.query?.around === "6"
              ? pageFixture(
                  [4, 6, 7, 8].map((id) => messageFixture(id, 12)),
                  4,
                  8,
                )
              : pageFixture(
                  [40, 41].map((id) => messageFixture(id, 12)),
                  40,
                ),
          ),
        );
        yield* welcome(5, false);

        expect(timelineIds(12)).toEqual([4, 6, 7, 8]);
        expect(store.getState().timelines[12]?.after).toBe(8);
        expect(store.getState().timelines[12]?.generation).toBe(generation);
      }),
    ),
  );

  it.effect("refreshes only the activity count when the server resumes", () =>
    withSync(
      Effect.gen(function* () {
        const socket = yield* MemorySocket;
        const api = yield* FakeApi;

        yield* serve([]);
        yield* api.reply("GET /activity/unread_count", { unreadCount: 5, unreadRevision: 1 });
        yield* startEngine;
        yield* welcome(5, false);

        const before = (yield* api.requests).length;

        yield* socket.drop;
        yield* TestClock.adjust(250);
        yield* api.reply("GET /activity/unread_count", { unreadCount: 6, unreadRevision: 2 });
        yield* welcome(9, true);

        expect((yield* api.requests).slice(before)).toEqual([
          { method: "GET", path: "/activity/unread_count" },
        ]);
        expect(store.getState().activity.unreadCount).toBe(6);
      }),
    ),
  );
});

describe("outbox", () => {
  /** Sends "hello" with the POST held until `confirm`, and the `message.created` event ready. */
  const holdSend = Effect.gen(function* () {
    const api = yield* FakeApi;
    const gate = yield* Deferred.make<void>();
    const clientId = yield* Ref.make("");

    const confirmed = Effect.map(Ref.get(clientId), (id) =>
      messageFixture(3, 12, { clientMessageId: id }),
    );

    yield* serve([messageFixture(1, 12), messageFixture(2, 12)]);
    yield* api.route("POST /rooms/12/messages", () =>
      Effect.andThen(Deferred.await(gate), confirmed),
    );
    yield* startEngine;
    yield* session.openRoom(12, null);

    const id = yield* session.send(12, "hello");

    yield* Ref.set(clientId, id);
    yield* settle;

    expect(store.getState().pending[id]?.state).toBe("sending");

    const event: SyncEvent = {
      seq: 1,
      topic: "room:12",
      type: "message.created",
      data: yield* confirmed,
    };

    return { id, confirm: Effect.andThen(Deferred.succeed(gate, undefined), settle), event };
  });

  it.effect("reconciles when the POST reply lands before the event", () =>
    withSync(
      Effect.gen(function* () {
        const { id, confirm, event } = yield* holdSend;

        yield* confirm;

        expect(store.getState().pending[id]).toBeUndefined();
        expect(timelineIds(12)).toEqual([1, 2, 3]);

        yield* pushEvents(event);

        expect(timelineIds(12)).toEqual([1, 2, 3]);
        expect(store.getState().pendingByRoom[12]).toEqual([]);
      }),
    ),
  );

  it.effect("reconciles when the event lands before the POST reply", () =>
    withSync(
      Effect.gen(function* () {
        const { id, confirm, event } = yield* holdSend;

        yield* pushEvents(event);

        expect(store.getState().pending[id]).toBeUndefined();
        expect(timelineIds(12)).toEqual([1, 2, 3]);

        yield* confirm;

        expect(timelineIds(12)).toEqual([1, 2, 3]);
        expect(store.getState().pendingByRoom[12]).toEqual([]);
      }),
    ),
  );

  it.effect("marks a rejected send failed, and retries it with the same client id", () =>
    withSync(
      Effect.gen(function* () {
        const api = yield* FakeApi;
        const outbox = yield* Outbox;

        yield* serve([messageFixture(1, 12)]);
        yield* api.route("POST /rooms/12/messages", () =>
          Effect.fail(new Validation({ message: "Body is too long", fields: {} })),
        );
        yield* startEngine;
        yield* session.openRoom(12, null);

        const id = yield* session.send(12, "hello");

        yield* settle;

        expect(store.getState().pending[id]).toMatchObject({
          state: "failed",
          error: "Body is too long",
        });

        yield* api.reply("POST /rooms/12/messages", messageFixture(2, 12, { clientMessageId: id }));
        yield* outbox.retry(id);
        yield* settle;

        const posts = (yield* api.requests).filter((request) => request.method === "POST");

        expect(posts).toHaveLength(2);
        expect(posts[0]?.body).toMatchObject({ clientMessageId: id, markdownSource: "hello" });
        expect(posts[1]?.body).toEqual(posts[0]?.body);
        expect(store.getState().pending[id]).toBeUndefined();
        expect(timelineIds(12)).toEqual([1, 2]);
      }),
    ),
  );

  it.effect("resends after a server error, with backoff", () =>
    withSync(
      Effect.gen(function* () {
        const api = yield* FakeApi;
        const attempts = yield* Ref.make(0);

        yield* serve([]);
        yield* api.route("POST /rooms/12/messages", () =>
          Effect.flatMap(
            Ref.updateAndGet(attempts, (count) => count + 1),
            (count) =>
              count === 1
                ? Effect.fail(new ServerError({ status: 503, message: "Service Unavailable" }))
                : Effect.succeed(messageFixture(1, 12, { clientMessageId: "unused" })),
          ),
        );
        yield* startEngine;

        const id = yield* session.send(12, "hello");

        yield* settle;

        expect(yield* Ref.get(attempts)).toBe(1);
        expect(store.getState().pending[id]?.state).toBe("sending");

        yield* TestClock.adjust(1000);

        expect(yield* Ref.get(attempts)).toBe(2);
      }),
    ),
  );
  it.effect("resends at once when the socket reconnects, without waiting out the backoff", () =>
    withSync(
      Effect.gen(function* () {
        const api = yield* FakeApi;
        const socket = yield* MemorySocket;
        const lifecycle = yield* TestLifecycle;
        const attempts = yield* Ref.make(0);
        const reachable = yield* Ref.make(false);
        const clientId = yield* Ref.make("");

        yield* serve([]);
        yield* api.route("POST /rooms/12/messages", () =>
          Effect.flatMap(
            Ref.updateAndGet(attempts, (count) => count + 1),
            () =>
              Effect.flatMap(Ref.get(reachable), (up) =>
                up
                  ? Effect.map(Ref.get(clientId), (id) =>
                      messageFixture(1, 12, { clientMessageId: id }),
                    )
                  : Effect.fail(new NetworkError({ message: "offline" })),
              ),
          ),
        );
        yield* socket.setReachable(false);
        yield* startEngine;

        const id = yield* session.send(12, "hello");

        yield* Ref.set(clientId, id);

        // Failures at 0, 1 s, 3 s and 7 s; the next resend would wait until 15 s.
        for (const delay of [0, 1000, 2000, 4000]) {
          yield* TestClock.adjust(delay);
        }

        expect(yield* Ref.get(attempts)).toBe(4);

        yield* Ref.set(reachable, true);
        yield* socket.setReachable(true);
        yield* lifecycle.fire("online");
        yield* settle;

        expect(yield* socket.isOpen).toBe(true);
        expect(yield* Ref.get(attempts)).toBe(5);
        expect(store.getState().pending[id]).toBeUndefined();
      }),
    ),
  );
  it.effect("posts a thread reply to its thread and keeps it off the room's timeline", () =>
    withSync(
      Effect.gen(function* () {
        const api = yield* FakeApi;
        const posted = yield* Ref.make<CreateMessage | null>(null);

        yield* serve([messageFixture(1, 12)]);
        yield* api.route("POST /threads/88/messages", (request) =>
          Effect.gen(function* () {
            const body = yield* Schema.decodeUnknownEffect(CreateMessageSchema)(request.body);

            yield* Ref.set(posted, body);

            return messageFixture(5, 12, { threadId: 88, clientMessageId: body.clientMessageId });
          }).pipe(Effect.orDie),
        );
        yield* startEngine;
        yield* session.openRoom(12, null);

        const id = yield* session.send(12, "in the thread", {
          threadId: 88,
          attachmentSignedId: "signed-1",
        });

        yield* settle;

        expect(yield* Ref.get(posted)).toMatchObject({
          clientMessageId: id,
          markdownSource: "in the thread",
          attachmentSignedId: "signed-1",
        });
        expect(store.getState().pending[id]).toBeUndefined();
        expect(store.getState().pendingByThread[88]).toEqual([]);
        expect(store.getState().pendingByRoom[12] ?? []).toEqual([]);
        expect(timelineIds(12)).toEqual([1]);
      }),
    ),
  );
});

describe("tombstones", () => {
  it.effect("keep a removed message out of late updates and pages until they lapse", () =>
    withSync(
      Effect.gen(function* () {
        const socket = yield* MemorySocket;

        yield* serve([1, 2, 3].map((id) => messageFixture(id, 12)));
        yield* startEngine;
        yield* session.openRoom(12, null);
        yield* pushEvents({
          seq: 1,
          topic: "room:12",
          type: "message.removed",
          data: { id: 2, roomId: 12, threadId: null },
        });

        expect(timelineIds(12)).toEqual([1, 3]);

        const edited = messageFixture(2, 12, {
          markdownSource: "edited",
          updatedAt: "2026-10-06T01:00:00.000Z",
        });

        yield* pushEvents({ seq: 2, topic: "room:12", type: "message.updated", data: edited });
        yield* socket.push({ t: "resync", topics: ["room:12"], reason: "lagged" });
        yield* settle;

        expect(store.getState().messages[2]).toBeUndefined();
        expect(timelineIds(12)).toEqual([1, 3]);

        yield* TestClock.adjust(61_000);

        expect(store.getState().tombstones).toEqual({});

        yield* session.jumpToPresent(12);

        expect(timelineIds(12)).toEqual([1, 2, 3]);
      }),
    ),
  );
});

describe("typing", () => {
  it.effect("sends typing on at most every 3 s, and off at once", () =>
    withSync(
      Effect.gen(function* () {
        const socket = yield* MemorySocket;
        const typing = yield* Typing;

        yield* startEngine;
        yield* typing.set("room:12", true);
        yield* TestClock.adjust(1000);
        yield* typing.set("room:12", true);
        yield* TestClock.adjust(2000);
        yield* typing.set("room:12", true);
        yield* typing.set("room:12", false);
        yield* typing.set("room:12", false);

        expect(typingFrames(yield* socket.sent)).toEqual([
          { t: "typing", conv: "room:12", on: true },
          { t: "typing", conv: "room:12", on: true },
          { t: "typing", conv: "room:12", on: false },
        ]);
      }),
    ),
  );

  it.effect("drops someone else's typing after 6 s, and ignores the echo of mine", () =>
    withSync(
      Effect.gen(function* () {
        yield* startEngine;
        yield* pushEvents(
          { seq: 1, topic: "room:12", type: "typing", data: { userId: 9, on: true } },
          { seq: 2, topic: "room:12", type: "typing", data: { userId: 7, on: true } },
        );

        expect(Object.keys(store.getState().typing["room:12"] ?? {})).toEqual(["9"]);

        const expiresAt = store.getState().typing["room:12"]?.[9] ?? 0;
        const now = yield* Clock.currentTimeMillis;

        expect(expiresAt - now).toBeLessThanOrEqual(6000);

        // Dropped at the exact millisecond it lapses, not on a periodic sweep.
        yield* TestClock.adjust(expiresAt - now - 1);

        expect(store.getState().typing["room:12"]?.[9]).toBeDefined();

        yield* TestClock.adjust(1);

        expect(store.getState().typing["room:12"]?.[9]).toBeUndefined();
      }),
    ),
  );

  it.effect("a refreshed typist's earlier expiry doesn't drop them", () =>
    withSync(
      Effect.gen(function* () {
        const typing = (seq: number) =>
          pushEvents({ seq, topic: "room:12", type: "typing", data: { userId: 9, on: true } });

        yield* startEngine;
        yield* typing(1);
        yield* TestClock.adjust(4000);
        yield* typing(2);
        yield* TestClock.adjust(3000);

        expect(store.getState().typing["room:12"]?.[9]).toBeDefined();

        yield* TestClock.adjust(3000);

        expect(store.getState().typing["room:12"]?.[9]).toBeUndefined();
      }),
    ),
  );
});

describe("people", () => {
  it.effect("fetches a live activity item's unknown creator, so its row can name them", () =>
    withSync(
      Effect.gen(function* () {
        const api = yield* FakeApi;

        yield* serve([]);
        yield* startEngine;
        yield* welcome(0, false);
        yield* api.reply("GET /users", { users: [userFixture(8, "Lucía Fernández")] });

        yield* pushEvents({
          seq: 1,
          topic: "user",
          type: "activity.item",
          data: {
            unreadCount: 1,
            unreadRevision: 1,
            item: activityItem,
          },
        });

        const lookups = (yield* api.requests).filter((request) => request.path === "/users");

        expect(lookups.at(-1)?.query).toEqual({ ids: "8" });
        expect(store.getState().activity.items[40]?.source.creatorId).toBe(8);
        expect(store.getState().users[8]?.name).toBe("Lucía Fernández");
      }),
    ),
  );
});

describe("activity follows the room list", () => {
  const countRequests = Effect.gen(function* () {
    const api = yield* FakeApi;

    return (yield* api.requests).filter((request) => request.path === "/activity/unread_count")
      .length;
  });

  const rowEvent = (seq: number, row: SidebarRow): SyncEvent => ({
    seq,
    topic: "user",
    type: "sidebar.row.upserted",
    data: row,
  });

  /** The sidebar loaded and the inbox's Unread tab shown once, the badge at 2. */
  const loaded = Effect.gen(function* () {
    const api = yield* FakeApi;

    yield* serve([]);
    yield* api.reply("GET /activity/unread_count", { unreadCount: 2, unreadRevision: 1 });
    yield* startEngine;
    yield* welcome(0, false);
    mutations.landActivityPage(
      "all",
      "unread",
      { items: [], users: [], unreadCount: 2, unreadRevision: 1, nextCursor: null },
      "replace",
    );
  });

  const unreadListStale = () => activityListOf(store.getState(), "all", "unread").stale;

  it.effect("reloads the inbox and its badge when a room comes into the sidebar", () =>
    withSync(
      Effect.gen(function* () {
        const api = yield* FakeApi;

        yield* loaded;

        const before = yield* countRequests;

        expect(unreadListStale()).toBe(false);

        yield* api.reply("GET /activity/unread_count", { unreadCount: 5, unreadRevision: 2 });
        yield* pushEvents(
          rowEvent(1, sidebarRowFixture(30, "design")),
          rowEvent(2, sidebarRowFixture(31, "ops")),
        );

        expect(unreadListStale()).toBe(true);
        expect((yield* countRequests) - before).toBe(1);
        expect(store.getState().activity.unreadCount).toBe(5);
      }),
    ),
  );

  it.effect("leaves the inbox alone when a row it already shows changes", () =>
    withSync(
      Effect.gen(function* () {
        yield* loaded;

        const before = yield* countRequests;

        yield* pushEvents(rowEvent(1, { ...sidebarRowFixture(12, "general"), unreadCount: 4 }));

        expect(unreadListStale()).toBe(false);
        expect((yield* countRequests) - before).toBe(0);
      }),
    ),
  );

  it.effect("reloads it too for a room the viewer adds here, like a new DM", () =>
    withSync(
      Effect.gen(function* () {
        yield* loaded;

        const before = yield* countRequests;

        mutations.applyEvents(
          [{ ...rowEvent(0, sidebarRowFixture(44, "Ada")), topic: "user:1" }],
          0,
        );
        yield* settle;

        expect(unreadListStale()).toBe(true);
        expect((yield* countRequests) - before).toBe(1);
      }),
    ),
  );
});

describe("room management refresh", () => {
  const loaded = Effect.gen(function* () {
    yield* serve([]);
    yield* startEngine;
    yield* welcome(0, false);
    yield* session.openRoom(12, null);
  });

  const changed = (seq: number, roomId = 12): SyncEvent => ({
    seq,
    topic: "user",
    type: "sidebar.row.upserted",
    data: { ...sidebarRowFixture(roomId, "Edited room"), refreshRoom: true },
  });

  it.effect(
    "refreshes loaded roster facts once per flagged batch and ignores ordinary hot rows",
    () =>
      withSync(
        Effect.gen(function* () {
          yield* loaded;
          const api = yield* FakeApi;
          const before = (yield* api.requests).length;
          let reloads = 0;

          const unsubscribe = onRoomRefresh(12, () => {
            reloads++;
          });

          yield* api.reply("GET /rooms/12", {
            ...roomDetailFixture(12),
            memberCount: 8,
            memberPreviewIds: [7, 8],
          });
          yield* pushEvents(changed(1), changed(2), changed(3, 30));

          expect(
            (yield* api.requests).slice(before).filter((request) => request.path === "/rooms/12"),
          ).toHaveLength(1);
          expect(
            (yield* api.requests).slice(before).some((request) => request.path === "/rooms/30"),
          ).toBe(false);
          expect(store.getState().rooms[12]?.detail).toMatchObject({
            memberCount: 8,
            memberPreviewIds: [7, 8],
            displayName: "Edited room",
          });
          expect(reloads).toBe(1);

          const after = (yield* api.requests).length;

          yield* pushEvents({
            seq: 4,
            topic: "user",
            type: "sidebar.row.upserted",
            data: { ...sidebarRowFixture(12, "Edited room"), unreadCount: 2 },
          });

          expect(
            (yield* api.requests).slice(after).some((request) => request.path === "/rooms/12"),
          ).toBe(false);
          expect(reloads).toBe(1);
          unsubscribe();
          invalidateRoom(12);
          expect(reloads).toBe(1);
        }),
      ),
  );

  it.effect("doesn't let a save that was on its way during a resync bring back a lost room", () =>
    withSync(
      Effect.gen(function* () {
        yield* loaded;
        const api = yield* FakeApi;
        const socket = yield* MemorySocket;
        const release = yield* Deferred.make<void>();
        const row = sidebarRowFixture(12, "general");
        const stale = { room: row.room, detail: roomDetailFixture(12), row };

        // The save is sent, then its reply is held until after the reconnect.
        yield* api.route("PATCH /rooms/12", () => Deferred.await(release).pipe(Effect.as(stale)));

        const saving = yield* Effect.forkChild(roomActions.update(12, { type: "open" }));

        yield* settle;

        // Meanwhile the viewer is removed; the server can't resume, so the client resyncs.
        yield* api.reply("GET /sidebar", sidebarFixture([]));
        yield* api.route("GET /rooms/12", () => Effect.fail(new NotFound({ message: "gone" })));
        yield* socket.drop;
        yield* TestClock.adjust(250);
        yield* welcome(1, false, "e2");

        expect(store.getState().sidebar.rows[12]).toBeUndefined();
        expect(store.getState().rooms[12]?.detail).toBeNull();

        const before = (yield* api.requests).length;

        yield* Deferred.succeed(release, undefined);
        yield* Fiber.join(saving);
        yield* settle;

        expect(store.getState().sidebar.rows[12]).toBeUndefined();
        expect(store.getState().rooms[12]?.detail).toBeNull();
        expect((yield* api.requests).slice(before)).toContainEqual({
          method: "GET",
          path: "/rooms/12",
        });
      }),
    ),
  );

  it.effect(
    "a repair read's NotFound that beats a pending resync leaves the resync to clear the room",
    () =>
      withSync(
        Effect.gen(function* () {
          yield* loaded;
          const api = yield* FakeApi;
          const socket = yield* MemorySocket;
          const releaseSave = yield* Deferred.make<void>();
          const releaseRepair = yield* Deferred.make<void>();
          const releaseResync = yield* Deferred.make<void>();
          const row = sidebarRowFixture(12, "general");
          const stale = { room: row.room, detail: roomDetailFixture(12), row };
          const gone = new NotFound({ message: "gone" });
          let reads = 0;

          yield* api.route("PATCH /rooms/12", () =>
            Deferred.await(releaseSave).pipe(Effect.as(stale)),
          );
          // The save's repair read and the resync's read both find the room deleted, each held.
          yield* api.route("GET /rooms/12", () => {
            reads += 1;

            if (reads === 1)
              return Deferred.await(releaseRepair).pipe(Effect.andThen(Effect.fail(gone)));

            if (reads === 2)
              return Deferred.await(releaseResync).pipe(Effect.andThen(Effect.fail(gone)));

            return Effect.fail(gone);
          });

          const saving = yield* Effect.forkChild(roomActions.update(12, { type: "open" }));

          yield* settle;
          invalidateRoom(12);
          yield* Deferred.succeed(releaseSave, undefined);
          yield* settle;
          expect(reads).toBe(1);

          // Reconnect: the sidebar no longer lists the room, and its resync read is on its way.
          yield* api.reply("GET /sidebar", sidebarFixture([]));
          yield* socket.drop;
          yield* TestClock.adjust(250);
          yield* welcome(1, false, "e2");
          expect(reads).toBe(2);

          // The repair's NotFound arrives first: it's older than the snapshot, so it's dropped.
          yield* Deferred.succeed(releaseRepair, undefined);
          yield* Fiber.join(saving);
          yield* settle;

          // The resync's own NotFound still counts, and clears the room.
          yield* Deferred.succeed(releaseResync, undefined);
          yield* settle;

          expect(store.getState().rooms[12]?.detail).toBeNull();
          expect(store.getState().sidebar.rows[12]).toBeUndefined();
          expect(reads).toBe(2);
        }),
      ),
  );

  it.effect("drops a repair read that was on its way during a resync", () =>
    withSync(
      Effect.gen(function* () {
        yield* loaded;
        const api = yield* FakeApi;
        const socket = yield* MemorySocket;
        const releaseSave = yield* Deferred.make<void>();
        const releaseRepair = yield* Deferred.make<void>();
        const row = sidebarRowFixture(12, "general");
        const stale = { room: row.room, detail: roomDetailFixture(12), row };
        let reads = 0;

        yield* api.route("PATCH /rooms/12", () =>
          Deferred.await(releaseSave).pipe(Effect.as(stale)),
        );
        // The save's repair read is held across the resync; later reads find the room gone.
        yield* api.route("GET /rooms/12", () => {
          reads += 1;

          return reads === 1
            ? Deferred.await(releaseRepair).pipe(Effect.as(roomDetailFixture(12)))
            : Effect.fail(new NotFound({ message: "gone" }));
        });

        const saving = yield* Effect.forkChild(roomActions.update(12, { type: "open" }));

        yield* settle;
        // A management change lands while the save is on its way: its reply is superseded.
        invalidateRoom(12);
        yield* Deferred.succeed(releaseSave, undefined);
        yield* settle;
        expect(reads).toBe(1);

        // While the repair read is held, the viewer loses the room and the client resyncs.
        yield* api.reply("GET /sidebar", sidebarFixture([]));
        yield* socket.drop;
        yield* TestClock.adjust(250);
        yield* welcome(1, false, "e2");

        expect(store.getState().rooms[12]?.detail).toBeNull();

        yield* Deferred.succeed(releaseRepair, undefined);
        yield* Fiber.join(saving);
        yield* settle;

        expect(store.getState().rooms[12]?.detail).toBeNull();
        expect(store.getState().sidebar.rows[12]).toBeUndefined();
      }),
    ),
  );

  it.effect("doesn't land a create that was on its way during a resync over the snapshot", () =>
    withSync(
      Effect.gen(function* () {
        yield* loaded;
        const api = yield* FakeApi;
        const socket = yield* MemorySocket;
        const release = yield* Deferred.make<void>();
        const row = sidebarRowFixture(40, "launch");
        const made = { room: row.room, detail: roomDetailFixture(40), row };

        yield* api.route("POST /rooms", () => Deferred.await(release).pipe(Effect.as(made)));

        const creating = yield* Effect.forkChild(
          roomActions.create({ type: "open", name: "launch", iconName: null, clientRoomId: "k1" }),
        );

        yield* settle;

        // The snapshot doesn't list the new room: by then the viewer has lost it again.
        yield* api.reply("GET /sidebar", sidebarFixture([]));
        yield* api.route("GET /rooms/40", () => Effect.fail(new NotFound({ message: "gone" })));
        yield* socket.drop;
        yield* TestClock.adjust(250);
        yield* welcome(1, false, "e2");

        yield* Deferred.succeed(release, undefined);
        yield* Fiber.join(creating);
        yield* settle;

        expect(store.getState().sidebar.rows[40]).toBeUndefined();
        expect(store.getState().rooms[40]?.detail).toBeNull();
      }),
    ),
  );

  it.effect("refreshes a hidden row without revoking its still-readable room", () =>
    withSync(
      Effect.gen(function* () {
        yield* loaded;
        const api = yield* FakeApi;

        yield* api.reply("GET /rooms/12", { ...roomDetailFixture(12), memberCount: 5 });
        yield* pushEvents({
          seq: 1,
          topic: "user",
          type: "sidebar.row.removed",
          data: { roomId: 12, refreshRoom: true },
        });

        expect(store.getState().sidebar.rows[12]).toBeUndefined();
        expect(store.getState().rooms[12]?.detail?.memberCount).toBe(5);
        expect(store.getState().rooms[12]?.status).toBe("ready");
      }),
    ),
  );

  it.effect("drops an older pending roster response after a newer management change", () =>
    withSync(
      Effect.gen(function* () {
        yield* loaded;
        const api = yield* FakeApi;
        const oldReply = yield* Deferred.make<ReturnType<typeof roomDetailFixture>>();
        let requests = 0;

        yield* api.route("GET /rooms/12", () =>
          ++requests === 1
            ? Deferred.await(oldReply)
            : Effect.succeed({ ...roomDetailFixture(12), memberCount: 9 }),
        );
        yield* pushEvents(changed(1));
        yield* pushEvents(changed(2));

        expect(store.getState().rooms[12]?.detail?.memberCount).toBe(9);
        yield* Deferred.succeed(oldReply, { ...roomDetailFixture(12), memberCount: 4 });
        yield* settle;
        expect(store.getState().rooms[12]?.detail?.memberCount).toBe(9);
      }),
    ),
  );

  it.effect("drops a pending response after a locally confirmed management reply", () =>
    withSync(
      Effect.gen(function* () {
        yield* loaded;
        const api = yield* FakeApi;
        const reply = yield* Deferred.make<ReturnType<typeof roomDetailFixture>>();

        yield* api.route("GET /rooms/12", () => Deferred.await(reply));
        yield* pushEvents(changed(1));
        mutations.setRoomDetail({ ...roomDetailFixture(12), memberCount: 11 });
        invalidateRoom(12);
        yield* Deferred.succeed(reply, { ...roomDetailFixture(12), memberCount: 4 });
        yield* settle;

        expect(store.getState().rooms[12]?.detail?.memberCount).toBe(11);
      }),
    ),
  );
});

describe("room access refresh", () => {
  const loaded = Effect.gen(function* () {
    yield* serve([]);
    yield* startEngine;
    yield* welcome(0, false);
    yield* session.openRoom(12, null);
  });

  it.effect(
    "a flagged access loss clears detail and prevents a pending older success restoring it",
    () =>
      withSync(
        Effect.gen(function* () {
          yield* loaded;
          const api = yield* FakeApi;
          const oldReply = yield* Deferred.make<ReturnType<typeof roomDetailFixture>>();
          let requests = 0;

          yield* api.route("GET /rooms/12", () =>
            ++requests === 1
              ? Deferred.await(oldReply)
              : Effect.fail(new NotFound({ message: "Room not found" })),
          );
          yield* pushEvents({
            seq: 1,
            topic: "user",
            type: "sidebar.row.upserted",
            data: { ...sidebarRowFixture(12, "Room"), refreshRoom: true },
          });
          yield* pushEvents({
            seq: 2,
            topic: "user",
            type: "sidebar.row.removed",
            data: { roomId: 12, refreshRoom: true },
          });

          expect(store.getState().rooms[12]?.detail).toBeNull();
          expect(store.getState().sidebar.rows[12]).toBeUndefined();
          yield* Deferred.succeed(oldReply, roomDetailFixture(12));
          yield* settle;
          expect(store.getState().rooms[12]?.detail).toBeNull();
        }),
      ),
  );

  it.effect("a disconnected access loss clears stale detail even when both REST reads fail", () =>
    withSync(
      Effect.gen(function* () {
        yield* loaded;
        const api = yield* FakeApi;
        const socket = yield* MemorySocket;

        yield* api.route("GET /rooms/12", () =>
          Effect.fail(new NotFound({ message: "Room not found" })),
        );
        yield* api.route("GET /rooms/12/messages", () =>
          Effect.fail(new NotFound({ message: "Room not found" })),
        );
        yield* socket.push({ t: "resync", topics: ["room:12"], reason: "lagged" });
        yield* settle;

        expect(store.getState().rooms[12]?.detail).toBeNull();
        expect(store.getState().rooms[12]?.status).toBe("error");
        expect(store.getState().sidebar.rows[12]).toBeUndefined();
      }),
    ),
  );

  it.effect("a delayed resync preserves later membership fields and ignores a local delete", () =>
    withSync(
      Effect.gen(function* () {
        yield* loaded;
        const api = yield* FakeApi;
        const socket = yield* MemorySocket;
        const reply = yield* Deferred.make<ReturnType<typeof roomDetailFixture>>();

        yield* api.route("GET /rooms/12", () => Deferred.await(reply));
        yield* socket.push({ t: "resync", topics: ["room:12"], reason: "lagged" });
        yield* settle;
        const row = sidebarRowFixture(12, "Room");

        mutations.applyEvents(
          [
            {
              seq: 0,
              topic: "user",
              type: "sidebar.row.upserted",
              data: {
                ...row,
                membership: {
                  ...row.membership,
                  involvement: "muted",
                  roomCategoryId: 4,
                  lastReadMessageId: 90,
                },
              },
            },
          ],
          0,
        );
        yield* Deferred.succeed(reply, roomDetailFixture(12));
        yield* settle;

        expect(store.getState().rooms[12]?.detail?.membership).toMatchObject({
          involvement: "muted",
          roomCategoryId: 4,
          lastReadMessageId: 90,
        });

        const deletedReply = yield* Deferred.make<ReturnType<typeof roomDetailFixture>>();

        yield* api.route("GET /rooms/12", () => Deferred.await(deletedReply));
        yield* socket.push({ t: "resync", topics: ["room:12"], reason: "lagged" });
        yield* settle;
        mutations.setRoomUnavailable(12);
        invalidateRoom(12);
        yield* Deferred.succeed(deletedReply, roomDetailFixture(12));
        yield* settle;
        expect(store.getState().rooms[12]?.detail).toBeNull();
      }),
    ),
  );
});

describe("decoding", () => {
  it.effect("drops an event it doesn't know and applies the rest of the batch", () =>
    withSync(
      Effect.gen(function* () {
        const socket = yield* MemorySocket;

        yield* serve([]);
        yield* startEngine;
        yield* welcome(0, false);
        yield* socket.pushText("not json");
        yield* socket.pushText(
          JSON.stringify({
            t: "batch",
            events: [{ seq: 1, topic: "user", type: "mystery", data: {} }, unreadEvent(2)],
          }),
        );
        yield* settle;

        expect(unreadCount(12)).toBe(1);
        expect(yield* socket.isOpen).toBe(true);
      }),
    ),
  );
});

describe("boards and open work panes", () => {
  it.effect(
    "has an open board's automations refetched when its room resyncs or the server restarts",
    () =>
      withSync(
        Effect.gen(function* () {
          const api = yield* FakeApi;
          const socket = yield* MemorySocket;
          const detail = roomDetailFixture(BOARD);
          detail.room.kind = "board";
          yield* api.reply(
            "GET /sidebar",
            sidebarFixture([sidebarRowFixture(BOARD, "Roadmap", "board")]),
          );
          yield* api.reply(`GET /rooms/${BOARD}`, detail);
          yield* api.reply(`GET /rooms/${BOARD}/board`, boardListing());
          yield* session.openRoom(BOARD, null);
          yield* boardActions.open(BOARD, { status: "all", owner: "anyone", tag: "" });
          yield* startEngine;
          yield* welcome(0, true);

          const signal = () => store.getState().boardAutomationsChanged[BOARD] ?? 0;
          const before = signal();

          // Replay expired: the events it names are gone, a settings change among them.
          yield* socket.push({ t: "resync", topics: [`room:${BOARD}`], reason: "missed" });
          yield* settle;
          expect(signal()).toBe(before + 1);

          // The server restarted and can't resume.
          yield* socket.drop;
          yield* TestClock.adjust(250);
          yield* welcome(1, false, "e2");
          expect(signal()).toBe(before + 2);
        }),
      ),
  );

  it.effect("resyncs board metadata, access and work without fetching a board timeline", () =>
    withSync(
      Effect.gen(function* () {
        const api = yield* FakeApi;
        const socket = yield* MemorySocket;
        const detail = roomDetailFixture(BOARD);
        detail.room.kind = "board";
        yield* api.reply(
          "GET /sidebar",
          sidebarFixture([sidebarRowFixture(BOARD, "Roadmap", "board")]),
        );
        yield* api.reply(`GET /rooms/${BOARD}`, detail);
        yield* api.reply(`GET /rooms/${BOARD}/board`, boardListing());
        yield* api.reply("GET /threads/1", boardDetail());
        yield* api.reply("GET /threads/1/messages", pageFixture([]));
        yield* session.openRoom(BOARD, null);
        yield* boardActions.open(BOARD, { status: "all", owner: "anyone", tag: "" });
        yield* threadActions.open(1);
        yield* startEngine;
        yield* welcome(0, true);
        const beforeRequests = yield* api.requests;

        const detailLoads = beforeRequests.filter(
          (request) => request.path === "/threads/1",
        ).length;

        const boardLoads = beforeRequests.filter(
          (request) => request.path === "/rooms/900/board",
        ).length;

        const changed = boardDetail(boardThread(1, "done", 7, ["api"], 1));

        if (changed.work !== null) changed.work.resultMarkdown = "Finished";
        yield* api.reply("GET /threads/1", changed);
        yield* pushEvents({
          seq: 1,
          topic: "room:900",
          type: "thread.updated",
          data: changed.thread,
        });
        expect(store.getState().threadPanes[1]?.work?.resultMarkdown).toBe("Finished");
        expect(store.getState().threadPanes[1]?.workFacts?.tags).toEqual(["api"]);
        const requests = yield* api.requests;
        expect(requests.filter((request) => request.path === "/threads/1")).toHaveLength(
          detailLoads + 1,
        );
        expect(requests.some((request) => request.path === "/rooms/900/messages")).toBe(false);
        const roomLoads = requests.filter((request) => request.path === "/rooms/900").length;
        yield* api.reply(`GET /rooms/${BOARD}`, { ...detail, memberCount: 7 });
        yield* socket.push({ t: "resync", topics: ["room:900"], reason: "missed" });
        yield* settle;
        expect(store.getState().rooms[BOARD]?.detail?.memberCount).toBe(7);
        expect(
          (yield* api.requests).filter((request) => request.path === "/rooms/900"),
        ).toHaveLength(roomLoads + 1);
        expect(
          (yield* api.requests).filter((request) => request.path === "/rooms/900/board"),
        ).toHaveLength(boardLoads + 1);
        expect(
          (yield* api.requests).some((request) => request.path === "/rooms/900/messages"),
        ).toBe(false);
        yield* api.route(`GET /rooms/${BOARD}`, () =>
          Effect.fail(new NotFound({ message: "gone" })),
        );
        yield* socket.push({ t: "resync", topics: ["room:900"], reason: "missed" });
        yield* settle;
        expect(store.getState().rooms[BOARD]?.detail).toBeNull();
        expect(store.getState().sidebar.rows[BOARD]).toBeUndefined();
        expect(store.getState().boards[BOARD]?.status).toBe("error");
      }),
    ),
  );
});
