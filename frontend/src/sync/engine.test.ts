import { beforeEach, describe, expect, it } from "@effect/vitest";
import { Clock, Deferred, Effect, Layer, Random, Ref, Schema } from "effect";
import { TestClock } from "effect/testing";
import { NetworkError, ServerError, Validation } from "../api/errors.ts";
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
import { CURSOR_STORAGE_KEY } from "./cursor.ts";
import { Engine } from "./engine.ts";
import { SyncServices } from "./layers.ts";
import { Outbox } from "./outbox.ts";
import * as session from "./session.ts";
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

          expect((yield* api.requests).length).toBe(before);
          expect(unreadCount(12)).toBe(2);
        }),
      );
    }),
  );
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

        expect((yield* api.requests).slice(before)).toEqual([
          { method: "GET", path: "/sidebar" },
          { method: "GET", path: "/activity/unread_count" },
          { method: "GET", path: "/rooms/12/messages" },
        ]);
        expect(timelineIds(12)).toEqual([1, 2, 3]);
        expect(sessionStorage.getItem(CURSOR_STORAGE_KEY)).toBe(
          JSON.stringify({ epoch: "e2", seq: 1 }),
        );
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

        const detail = (status: "active" | "closed", canClose: boolean) => ({
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

        const replies = (ids: readonly number[]) =>
          pageFixture(ids.map((id) => messageFixture(id, 12, { threadId: 88 })));

        yield* serve([]);
        yield* api.reply("GET /threads/88", detail("active", true));
        yield* api.reply("GET /threads/88/messages", replies([5, 6]));
        yield* startEngine;
        yield* welcome(5, false);
        yield* threadActions.open(88);

        expect(store.getState().threadTimelines[88]?.ids).toEqual([5, 6]);

        // Closed and one reply deleted while the events were lost.
        yield* api.reply("GET /threads/88", detail("closed", false));
        yield* api.reply("GET /threads/88/messages", replies([5, 7]));
        yield* socket.push({ t: "resync", topics: ["thread:88"], reason: "lagged" });
        yield* settle;

        expect(store.getState().threads[88]?.status).toBe("closed");
        expect(store.getState().threadPanes[88]?.permissions?.canClose).toBe(false);
        expect(store.getState().threadTimelines[88]?.ids).toEqual([5, 7]);
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

  it.effect("fetches nothing when the server resumes", () =>
    withSync(
      Effect.gen(function* () {
        const socket = yield* MemorySocket;
        const api = yield* FakeApi;

        yield* serve([]);
        yield* startEngine;
        yield* welcome(5, false);

        const before = (yield* api.requests).length;

        yield* socket.drop;
        yield* TestClock.adjust(250);
        yield* welcome(9, true);

        expect((yield* api.requests).length).toBe(before);
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
