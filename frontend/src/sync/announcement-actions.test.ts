import { afterEach, describe, expect, it } from "@effect/vitest";
import { Effect } from "effect";
import { FakeApi, sidebarFixture, sidebarRowFixture } from "../api/testing.ts";
import type { Room } from "../gen/Room.ts";
import type { SyncEvent } from "../gen/SyncEvent.ts";
import { isAnnouncementRoom } from "../store/announcements.ts";
import { mutations, sidebarRowClock, store } from "../store/store.ts";
import * as announcements from "./announcement-actions.ts";

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
});
