import { afterEach, describe, expect, it } from "@effect/vitest";
import { Clock, Effect } from "effect";
import { FakeApi, roomDetailFixture, sidebarFixture, sidebarRowFixture } from "../api/testing.ts";
import type { Room } from "../gen/Room.ts";
import type { SyncEvent } from "../gen/SyncEvent.ts";
import { isAnnouncementRoom } from "../store/announcements.ts";
import { mutations, sidebarRowClock, store } from "../store/store.ts";
import * as announcements from "./announcement-actions.ts";
import * as rooms from "./room-actions.ts";

const NEWS = 5;

const row = sidebarRowFixture(NEWS, "news");

const room = (announcement: boolean, updatedAt: string): Room => ({
  ...row.room,
  announcement,
  updatedAt,
});

/** The `sidebar.row.upserted` the server sends after a flag change, from any session. */
const synced = (announcement: boolean, updatedAt: string): SyncEvent => ({
  seq: 0,
  topic: "user:7",
  type: "sidebar.row.upserted",
  data: { ...row, room: room(announcement, updatedAt), refreshRoom: true },
});

const ON_AT = "2026-10-11T10:00:00.000Z";

const OFF_AT = "2026-10-11T10:00:05.000Z";

const flagged = () => isAnnouncementRoom(store.getState(), NEWS);

describe("announcement flag writes", () => {
  afterEach(() => mutations.reset());

  it.effect("a delayed reply never undoes what sync brought while it was in flight", () =>
    Effect.gen(function* () {
      mutations.loadSidebar(sidebarFixture([row]), sidebarRowClock());

      const fake = yield* FakeApi;

      // Enabling is slow: this write's own echo arrives, then another session turns it off, and
      // only then does the "on" reply come back.
      yield* fake.route(`PUT /rooms/${NEWS}/announcement`, () =>
        Effect.sync(() => {
          mutations.applyEvents([synced(true, ON_AT)], Date.now());
          mutations.applyEvents([synced(false, OFF_AT)], Date.now());

          return room(true, ON_AT);
        }),
      );

      expect((yield* announcements.setAnnouncement(NEWS, true)).announcement).toBe(true);
      expect(flagged()).toBe(false);

      // A later "off" changes nothing either.
      mutations.applyEvents([synced(false, OFF_AT)], Date.now());
      expect(flagged()).toBe(false);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("sync that lands while the reply is being landed keeps its touches", () =>
    Effect.gen(function* () {
      mutations.loadSidebar(sidebarFixture([{ ...row, unreadCount: 3 }]), sidebarRowClock());

      const fake = yield* FakeApi;

      yield* fake.reply(`PUT /rooms/${NEWS}/announcement`, room(true, ON_AT));

      // The room is read on another device just as the reply is about to land.
      const clock = yield* Clock.Clock;

      const reading: Clock.Clock = {
        currentTimeMillisUnsafe: () => clock.currentTimeMillisUnsafe(),
        currentTimeMillis: Effect.sync(() => {
          mutations.applyEvents(
            [{ seq: 0, topic: "user:7", type: "room.read", data: { roomId: NEWS } }],
            clock.currentTimeMillisUnsafe(),
          );

          return clock.currentTimeMillisUnsafe();
        }),
        currentTimeNanosUnsafe: () => clock.currentTimeNanosUnsafe(),
        currentTimeNanos: clock.currentTimeNanos,
        monotonicTimeNanosUnsafe: () => clock.monotonicTimeNanosUnsafe(),
        monotonicTimeNanos: clock.monotonicTimeNanos,
        sleep: (duration) => clock.sleep(duration),
      };

      yield* announcements
        .setAnnouncement(NEWS, true)
        .pipe(Effect.provideService(Clock.Clock, reading));

      expect(store.getState().sidebar.rows[NEWS]?.unreadCount).toBe(0);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("a reply older than the store's room doesn't land, even on an untouched row", () =>
    Effect.gen(function* () {
      mutations.loadSidebar(
        sidebarFixture([{ ...row, room: room(false, OFF_AT) }]),
        sidebarRowClock(),
      );

      const fake = yield* FakeApi;

      yield* fake.reply(`PUT /rooms/${NEWS}/announcement`, room(true, ON_AT));

      yield* announcements.setAnnouncement(NEWS, true);
      expect(flagged()).toBe(false);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("with nothing newer in the store, the reply lands on the row", () =>
    Effect.gen(function* () {
      mutations.loadSidebar(sidebarFixture([row]), sidebarRowClock());

      const fake = yield* FakeApi;

      yield* fake.reply(`PUT /rooms/${NEWS}/announcement`, room(true, ON_AT));

      yield* announcements.setAnnouncement(NEWS, true);
      expect(flagged()).toBe(true);
      expect(store.getState().sidebar.rows[NEWS]?.room.updatedAt).toBe(ON_AT);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("a management reply that removes the row keeps its newer detail's flag", () =>
    Effect.gen(function* () {
      mutations.loadSidebar(
        sidebarFixture([{ ...row, room: room(true, ON_AT) }]),
        sidebarRowClock(),
      );

      const fake = yield* FakeApi;
      const base = roomDetailFixture(NEWS);
      const detail = { ...base, room: { ...base.room, announcement: false, updatedAt: OFF_AT } };

      yield* fake.reply(`PATCH /rooms/${NEWS}`, { room: detail.room, detail, row: null });
      yield* rooms.update(NEWS, { type: "open" });

      expect(store.getState().sidebar.rows[NEWS]).toBeUndefined();
      expect(store.getState().rooms[NEWS]?.detail?.room).toMatchObject({
        announcement: false,
        updatedAt: OFF_AT,
      });
      expect(flagged()).toBe(false);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );
});
