import { afterEach, describe, expect, it } from "@effect/vitest";
import { Effect } from "effect";
import {
  FakeApi,
  meFixture,
  messageFixture,
  pageFixture,
  sidebarFixture,
  sidebarRowFixture,
  userFixture,
} from "../api/testing.ts";
import { mutations, sidebarRowClock, store } from "../store/store.ts";
import { customIcons, ensureUsers, markUnreadFrom } from "./message-view-actions.ts";

const ROOM = 12;

describe("message view actions", () => {
  afterEach(() => mutations.reset());

  it.effect("fetch only the people the store doesn't know", () =>
    Effect.gen(function* () {
      mutations.reset();
      mutations.mergeUsers([userFixture(2, "Maya Okafor")]);

      const fake = yield* FakeApi;

      yield* fake.reply("GET /users", { users: [userFixture(3, "Theo Brandt")] });
      yield* ensureUsers([2, 3, 3]);

      const requests = yield* fake.requests;

      expect(requests).toHaveLength(1);
      expect(requests[0]?.query).toEqual({ ids: "3" });
      expect(store.getState().users[3]?.name).toBe("Theo Brandt");

      yield* ensureUsers([2, 3]);
      expect(yield* fake.requests).toHaveLength(1);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("mark the room unread here at once and answer the first unread message", () =>
    Effect.gen(function* () {
      mutations.reset();
      mutations.setMe(meFixture);
      mutations.loadSidebar(
        sidebarFixture([sidebarRowFixture(ROOM, "general")]),
        sidebarRowClock(),
      );

      const fake = yield* FakeApi;

      yield* fake.reply(`DELETE /rooms/${ROOM}/read`, {
        roomId: ROOM,
        unread: true,
        firstUnreadMessageId: 40,
        unreadCount: 3,
      });

      const first = yield* markUnreadFrom(ROOM, 40);
      const [request] = yield* fake.requests;

      expect(first).toBe(40);
      expect(request?.body).toEqual({ messageId: 40 });
      expect(store.getState().sidebar.rows[ROOM]?.membership.unreadAt).not.toBeNull();
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("count the sidebar badge the way the divider counts", () =>
    Effect.gen(function* () {
      mutations.reset();
      mutations.setMe(meFixture);
      mutations.loadSidebar(
        sidebarFixture([sidebarRowFixture(ROOM, "general")]),
        sidebarRowClock(),
      );
      mutations.applyPage(
        ROOM,
        pageFixture([39, 40, 41, 42].map((id) => messageFixture(id, ROOM))),
        "replace",
      );

      const fake = yield* FakeApi;

      yield* fake.reply(`DELETE /rooms/${ROOM}/read`, {
        roomId: ROOM,
        unread: true,
        firstUnreadMessageId: 40,
        unreadCount: 3,
      });

      yield* markUnreadFrom(ROOM, 40);

      expect(store.getState().timelines[ROOM]?.unreadCount).toBe(3);
      expect(store.getState().sidebar.rows[ROOM]?.unreadCount).toBe(3);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("list the workspace's icons", () =>
    Effect.gen(function* () {
      const fake = yield* FakeApi;

      const shipit = {
        name: "shipit",
        title: "Ship it",
        kind: "custom",
        character: null,
        imageUrl: "/icons/shipit",
      };

      yield* fake.reply("GET /icons", { icons: [shipit] });

      expect(yield* customIcons()).toEqual([shipit]);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );
});
