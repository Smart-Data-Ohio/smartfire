import { beforeEach, expect, it } from "@effect/vitest";
import { act, cleanup, renderHook } from "@testing-library/react";
import { Effect, Layer, Random } from "effect";
import { TestClock } from "effect/testing";
import { afterEach, vi } from "vitest";
import { ServerError } from "../api/errors.ts";
import { FakeApi, meFixture, sidebarFixture, sidebarRowFixture } from "../api/testing.ts";
import { useAppIndicators } from "../features/shell/app-shell.tsx";
import type { SyncEvent } from "../gen/SyncEvent.ts";
import type { Boot } from "../store/model.ts";
import { mutations, store } from "../store/store.ts";
import { Engine } from "./engine.ts";
import { SyncServices } from "./layers.ts";
import * as session from "./session.ts";
import { beginSettingsEpoch } from "./settings-snapshot.ts";
import { MemorySocket, TestLifecycle } from "./testing.ts";

const TestLayer = SyncServices.pipe(
  Layer.provideMerge(Layer.mergeAll(FakeApi.layerClient, MemorySocket.layer, TestLifecycle.layer)),
);

const noJitter: Random.Random = { nextIntUnsafe: () => 0, nextDoubleUnsafe: () => 0.5 };

const withSync = <A, E>(body: Effect.Effect<A, E, Layer.Success<typeof TestLayer>>) =>
  body.pipe(Effect.provide(TestLayer), Effect.provideService(Random.Random, noJitter));

/** Lets forked fibers run and the 16 ms coalescing window close. */
const settle = TestClock.adjust("20 millis");

/** The badge the device shows: the app's writes and the service worker's pushes alike. */
let badgeCount = 0;

const setAppBadge = vi.fn(async (count: number) => {
  badgeCount = count;
});

const clearAppBadge = vi.fn(async () => {
  badgeCount = 0;
});

const serviceWorker = new EventTarget();

/** The service worker shows a pushed count, then tells the open app. */
const pushBadge = (count: number) =>
  Effect.promise(() =>
    act(async () => {
      badgeCount = count;
      serviceWorker.dispatchEvent(new MessageEvent("message", { data: { type: "badge-pushed" } }));
    }),
  );

const appWrites = () => setAppBadge.mock.calls.length + clearAppBadge.mock.calls.length;

const unreadRow = (id: number) => {
  const row = sidebarRowFixture(id, `Room ${id}`);

  return { ...row, membership: { ...row.membership, unreadAt: row.room.createdAt } };
};

const unreadEvent = (seq: number): SyncEvent => ({
  seq,
  topic: "user",
  type: "room.unread",
  data: { roomId: 12, messageId: 100 + seq, mentioned: false },
});

const boot: Boot = {
  user: { id: meFixture.user.id, name: "Ada", avatarUrl: "/avatar.svg" },
  account: {
    name: "Smart Data",
    logoUrl: null,
    logoStillUrl: null,
    bannerUrl: null,
    bannerStillUrl: null,
    uploadLimitBytes: 100 * 1024 * 1024,
  },
  customStyles: null,
  theme: "system",
  textSize: "default",
  cableUrl: "/cable",
  serviceWorkerUrl: null,
  version: "test",
  revision: null,
  appearancePreferences: null,
};

/** Serves the engine's reads, with `rows` as the sidebar. */
const serve = (rows: ReturnType<typeof sidebarRowFixture>[]) =>
  Effect.gen(function* () {
    const api = yield* FakeApi;

    yield* api.reply("GET /boot", { ...boot, csrfToken: "token" });
    yield* api.reply("GET /blocks", { viewerId: meFixture.user.id, people: [], revision: 0 });
    yield* api.reply("GET /sidebar", sidebarFixture(rows));
  });

/** Starts the engine, opens the app, and lets the first welcome catch the sidebar up. */
const openApp = Effect.gen(function* () {
  const engine = yield* Engine;
  const socket = yield* MemorySocket;

  vi.stubGlobal("navigator", { ...navigator, setAppBadge, clearAppBadge, serviceWorker });
  renderHook(() => useAppIndicators(null, null));
  yield* Effect.forkChild(engine.run);
  yield* settle;
  yield* socket.push({
    t: "welcome",
    epoch: "e1",
    seq: 10,
    resumed: false,
    replayThrough: 10,
    viewerId: meFixture.user.id,
  });
  yield* settle;
});

/** Drops the socket and lets the engine dial again, without a welcome yet. */
const reconnect = Effect.gen(function* () {
  const socket = yield* MemorySocket;

  yield* socket.drop;
  yield* TestClock.adjust(250);
});

beforeEach(() => {
  beginSettingsEpoch();
  mutations.reset();
  session.resetRoomVisits();
  mutations.setMe(meFixture);
  sessionStorage.clear();
});

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  vi.clearAllMocks();
  badgeCount = 0;
});

it.effect("keeps pushed badges until the sidebar catches up again after a reconnect", () =>
  withSync(
    Effect.gen(function* () {
      const api = yield* FakeApi;
      const socket = yield* MemorySocket;

      yield* serve([sidebarRowFixture(12, "general")]);
      yield* openApp;
      expect(clearAppBadge).toHaveBeenCalledOnce();

      yield* reconnect;
      expect(store.getState().connection).toBe("online");
      expect(store.getState().sidebarCaughtUp).toBe(false);

      const before = appWrites();

      // Online but not welcomed: the old sidebar's 0 must not replace the push.
      yield* pushBadge(1);
      expect(badgeCount).toBe(1);

      // The welcome's sidebar read fails: the old sidebar stays, and so does the push.
      yield* api.route("GET /sidebar", () =>
        Effect.fail(new ServerError({ status: 503, message: "Try again later" })),
      );
      yield* socket.push({
        t: "welcome",
        epoch: "e2",
        seq: 1,
        resumed: false,
        replayThrough: 1,
        viewerId: meFixture.user.id,
      });
      yield* settle;
      expect(store.getState().sidebar.status).toBe("ready");
      expect(store.getState().sidebarCaughtUp).toBe(false);
      expect(badgeCount).toBe(1);

      yield* pushBadge(2);
      expect(badgeCount).toBe(2);
      expect(appWrites()).toBe(before);

      // A later read lands: the app writes the badge again, from the fresh sidebar.
      yield* serve([unreadRow(12), unreadRow(13), unreadRow(14)]);
      yield* socket.push({ t: "resync", topics: ["user"], reason: "lagged" });
      yield* settle;
      expect(store.getState().sidebarCaughtUp).toBe(true);
      expect(setAppBadge).toHaveBeenLastCalledWith(3);
      expect(badgeCount).toBe(3);
    }),
  ),
);

it.effect("waits for a resume's replay before writing the badge", () =>
  withSync(
    Effect.gen(function* () {
      const socket = yield* MemorySocket;

      yield* serve([sidebarRowFixture(12, "general")]);
      yield* openApp;
      yield* reconnect;

      const before = appWrites();

      yield* socket.push({
        t: "welcome",
        epoch: "e1",
        seq: 10,
        resumed: true,
        replayThrough: 11,
        viewerId: meFixture.user.id,
      });
      yield* settle;
      yield* pushBadge(1);
      expect(appWrites()).toBe(before);
      expect(badgeCount).toBe(1);

      yield* socket.push({ t: "batch", events: [unreadEvent(11)] });
      yield* settle;
      expect(store.getState().sidebarCaughtUp).toBe(true);
      expect(clearAppBadge).toHaveBeenCalledOnce();
      expect(setAppBadge).toHaveBeenLastCalledWith(1);
      expect(badgeCount).toBe(1);
    }),
  ),
);
