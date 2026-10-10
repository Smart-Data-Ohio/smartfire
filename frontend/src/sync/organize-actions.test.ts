import { afterEach, describe, expect, it } from "@effect/vitest";
import { Deferred, Effect, Exit, Fiber } from "effect";
import { TestClock } from "effect/testing";
import { createMockServer } from "../../mock/server.ts";
import { Conflict, NetworkError, Validation } from "../api/errors.ts";
import {
  FakeApi,
  meFixture,
  roomDetailFixture,
  sidebarFixture,
  sidebarRowFixture,
} from "../api/testing.ts";
import { beginRoomRequest } from "../store/join-state.ts";
import type { RoomCategory, SidebarRow } from "../store/model.ts";
import { favoriteRows, organizedSidebar } from "../store/organize.ts";
import { mutations, sidebarRowClock, store } from "../store/store.ts";
import * as organize from "./organize-actions.ts";

const launch: RoomCategory = { id: 1, name: "Launch", collapsed: false, position: 1 };

const team: RoomCategory = { id: 2, name: "Team", collapsed: false, position: 2 };

function withMembership(row: SidebarRow, change: Partial<SidebarRow["membership"]>): SidebarRow {
  return { ...row, membership: { ...row.membership, ...change } };
}

const general = sidebarRowFixture(1, "general");

const design = withMembership(sidebarRowFixture(2, "design"), { roomCategoryId: launch.id });

const engineering = withMembership(sidebarRowFixture(3, "engineering"), { favoritePosition: 0 });

const ada = sidebarRowFixture(4, "Ada", "direct", [5]);

function seed(): void {
  mutations.reset();
  mutations.setMe(meFixture);
  mutations.loadSidebar(
    {
      ...sidebarFixture([general, design, engineering, ada]),
      categories: [launch, team],
    },
    sidebarRowClock(),
  );
}

/** The sidebar as the viewer sees it, pending changes included. */
const view = () => organizedSidebar(store.getState().sidebar);

const overlayIsEmpty = () => {
  const { memberships, categories } = store.getState().sidebar.overlay;

  return Object.keys(memberships).length === 0 && Object.keys(categories).length === 0;
};

describe("organize actions", () => {
  afterEach(() => mutations.reset());

  it.effect("show a favourite at once and keep the server's row once it lands", () =>
    Effect.gen(function* () {
      seed();

      const fake = yield* FakeApi;
      let during: number | null | undefined;

      yield* fake.route("POST /rooms/1/favorite", () => {
        during = view().rows[1]?.membership.favoritePosition;

        return Effect.succeed(withMembership(general, { favoritePosition: 4 }));
      });

      yield* organize.favorite(1);

      expect(during).toBe(1);
      expect(store.getState().sidebar.rows[1]?.membership.favoritePosition).toBe(4);
      expect(favoriteRows(view()).map((row) => row.room.id)).toEqual([3, 1]);
      expect(overlayIsEmpty()).toBe(true);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("keep newer counts a sync event brought while the reply was on its way", () =>
    Effect.gen(function* () {
      seed();

      const fake = yield* FakeApi;

      yield* fake.route("POST /rooms/1/favorite", () => {
        // A message arrives meanwhile; the reply was written before it.
        mutations.applyEvents(
          [
            {
              seq: 0,
              topic: "user:7",
              type: "sidebar.row.upserted",
              data: {
                ...withMembership(general, { unreadAt: "2026-10-05T00:00:00.000Z" }),
                unreadCount: 5,
              },
            },
          ],
          0,
        );

        return Effect.succeed(withMembership(general, { favoritePosition: 4 }));
      });

      yield* organize.favorite(1);

      const row = store.getState().sidebar.rows[1];

      expect(row?.membership.favoritePosition).toBe(4);
      expect(row?.unreadCount).toBe(5);
      expect(row?.membership.unreadAt).toBe("2026-10-05T00:00:00.000Z");
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("put a room back when the server refuses the move", () =>
    Effect.gen(function* () {
      seed();

      const fake = yield* FakeApi;
      let during: number | null | undefined;

      yield* fake.route("PUT /rooms/1/category", () => {
        during = view().rows[1]?.membership.roomCategoryId;

        return Effect.fail(new Validation({ message: "Nope", fields: {} }));
      });

      const exit = yield* Effect.exit(organize.moveRoom(1, { kind: "category", categoryId: 2 }));

      expect(Exit.isFailure(exit)).toBe(true);
      expect(during).toBe(2);
      expect(view().rows[1]?.membership.roomCategoryId).toBeNull();
      expect(overlayIsEmpty()).toBe(true);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("star then move a room dropped among the favourites", () =>
    Effect.gen(function* () {
      seed();

      const fake = yield* FakeApi;

      yield* fake.reply("POST /rooms/2/favorite", withMembership(design, { favoritePosition: 1 }));
      yield* fake.reply("PATCH /rooms/2/favorite", {
        rows: [
          withMembership(design, { favoritePosition: 0 }),
          withMembership(engineering, { favoritePosition: 1 }),
        ],
      });

      yield* organize.moveRoom(2, { kind: "favorite", index: 0 });

      const requests = yield* fake.requests;

      expect(requests.map((request) => `${request.method} ${request.path}`)).toEqual([
        "POST /rooms/2/favorite",
        "PATCH /rooms/2/favorite",
      ]);
      expect(requests.at(-1)?.body).toEqual({ position: 0 });
      expect(favoriteRows(view()).map((row) => row.room.id)).toEqual([2, 3]);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("skip the move when a new favourite lands at the end anyway", () =>
    Effect.gen(function* () {
      seed();

      const fake = yield* FakeApi;

      yield* fake.reply("POST /rooms/1/favorite", withMembership(general, { favoritePosition: 1 }));
      yield* organize.favorite(1);

      expect((yield* fake.requests).map((request) => request.method)).toEqual(["POST"]);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("unstar and assign when a favourite is dropped in a category", () =>
    Effect.gen(function* () {
      seed();

      const fake = yield* FakeApi;

      yield* fake.reply("DELETE /rooms/3/favorite", engineering);
      yield* fake.reply(
        "PUT /rooms/3/category",
        withMembership(engineering, {
          favoritePosition: null,
          roomCategoryId: team.id,
        }),
      );

      yield* organize.moveRoom(3, { kind: "category", categoryId: team.id });

      const requests = yield* fake.requests;

      expect(requests.map((request) => `${request.method} ${request.path}`)).toEqual([
        "DELETE /rooms/3/favorite",
        "PUT /rooms/3/category",
      ]);
      expect(requests.at(-1)?.body).toEqual({ roomCategoryId: team.id });
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("create a category under a temporary id, then move the room with the real one", () =>
    Effect.gen(function* () {
      seed();

      const fake = yield* FakeApi;
      let shown: readonly RoomCategory[] = [];
      let placedIn: number | null | undefined;

      yield* fake.route("POST /room_categories", (request) => {
        shown = view().categories;
        placedIn = view().rows[1]?.membership.roomCategoryId;

        expect(request.body).toEqual({ name: "Ops" });

        return Effect.succeed({ id: 9, name: "Ops", collapsed: false, position: 3 });
      });

      let whileAssigning: readonly number[] = [];
      let placedWhileAssigning: number | null | undefined;

      yield* fake.route("PUT /rooms/1/category", () => {
        // One Ops, not the draft beside the real one, with the room already in it.
        whileAssigning = view().categories.map((category) => category.id);
        placedWhileAssigning = view().rows[1]?.membership.roomCategoryId;

        return Effect.succeed(withMembership(general, { roomCategoryId: 9 }));
      });

      const created = yield* organize.createCategory("  Ops ", 1);
      const requests = yield* fake.requests;

      expect(shown.at(-1)).toMatchObject({ name: "Ops", position: 3 });
      expect(shown.at(-1)?.id).toBeLessThan(0);
      expect(placedIn).toBe(shown.at(-1)?.id);
      expect(created.id).toBe(9);
      expect(whileAssigning).toEqual([1, 2, 9]);
      expect(placedWhileAssigning).toBe(9);
      expect(requests.at(-1)?.body).toEqual({ roomCategoryId: 9 });
      expect(view().categories.map((category) => category.id)).toEqual([1, 2, 9]);
      expect(view().rows[1]?.membership.roomCategoryId).toBe(9);
      expect(overlayIsEmpty()).toBe(true);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("rename and fold with only the field that changed", () =>
    Effect.gen(function* () {
      seed();

      const fake = yield* FakeApi;

      yield* fake.reply("PATCH /room_categories/2", { ...team, name: "People" });
      yield* organize.renameCategory(2, " People ");

      yield* fake.reply("PATCH /room_categories/2", { ...team, name: "People", collapsed: true });
      yield* organize.setCollapsed(2, true);

      const requests = yield* fake.requests;

      expect(requests.map((request) => request.body)).toEqual([
        { name: "People" },
        { collapsed: true },
      ]);
      expect(view().categories.find((category) => category.id === 2)).toMatchObject({
        name: "People",
        collapsed: true,
      });
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("drop a rename reply from before a resync that removed the category", () =>
    Effect.gen(function* () {
      seed();

      const fake = yield* FakeApi;

      yield* fake.route("PATCH /room_categories/2", () => {
        // While the rename is on its way, another tab deletes the category, and a resync
        // (no event left to say so again) installs the sidebar without it.
        mutations.resyncSidebar(
          { ...sidebarFixture([general, design, engineering, ada]), categories: [launch] },
          sidebarRowClock(),
        );

        return Effect.succeed({ ...team, name: "People" });
      });

      yield* organize.renameCategory(2, "People");

      expect(store.getState().sidebar.categories.map((category) => category.id)).toEqual([1]);
      expect(store.getState().rowTouches.categories).toEqual({});
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("drop a fold reply from before sync removed the category", () =>
    Effect.gen(function* () {
      seed();

      const fake = yield* FakeApi;

      yield* fake.route("PATCH /room_categories/2", () => {
        mutations.applyEvents(
          [{ seq: 0, topic: "user:7", type: "sidebar.category.removed", data: { id: 2 } }],
          0,
        );

        return Effect.succeed({ ...team, collapsed: true });
      });

      yield* organize.setCollapsed(2, true);

      expect(store.getState().sidebar.categories.map((category) => category.id)).toEqual([1]);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("keep a category sync renamed over an older reorder reply", () =>
    Effect.gen(function* () {
      seed();

      const fake = yield* FakeApi;
      const renamed = { ...team, name: "People", position: 1 };

      yield* fake.route("PUT /room_categories/order", () => {
        mutations.applyEvents(
          [{ seq: 0, topic: "user:7", type: "sidebar.category.upserted", data: renamed }],
          0,
        );

        return Effect.succeed({
          categories: [
            { ...team, position: 1 },
            { ...launch, position: 2 },
          ],
        });
      });

      yield* organize.reorderCategories([2, 1]);

      expect(store.getState().sidebar.categories).toEqual([renamed, { ...launch, position: 2 }]);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("delete a category, its rooms going back to Channels", () =>
    Effect.gen(function* () {
      seed();

      const fake = yield* FakeApi;
      let during: number | null | undefined;

      yield* fake.route("DELETE /room_categories/1", () => {
        during = view().rows[2]?.membership.roomCategoryId;

        return Effect.succeed(null);
      });

      yield* organize.deleteCategory(1);

      expect(during).toBeNull();
      expect(view().categories.map((category) => category.id)).toEqual([2]);
      expect(view().rows[2]?.membership.roomCategoryId).toBeNull();
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("reorder categories, and refetch the sidebar on a 409", () =>
    Effect.gen(function* () {
      seed();

      const fake = yield* FakeApi;

      yield* fake.reply("PUT /room_categories/order", {
        categories: [
          { ...team, position: 1 },
          { ...launch, position: 2 },
        ],
      });

      yield* organize.reorderCategories([2, 1]);

      expect(view().categories.map((category) => category.id)).toEqual([2, 1]);

      const fresh: RoomCategory = { id: 5, name: "Elsewhere", collapsed: false, position: 3 };

      yield* fake.route("PUT /room_categories/order", () =>
        Effect.fail(new Conflict({ message: "Stale" })),
      );
      yield* fake.reply("GET /sidebar", {
        ...sidebarFixture([general, design, engineering, ada]),
        categories: [{ ...team, position: 1 }, { ...launch, position: 2 }, fresh],
      });

      const exit = yield* Effect.exit(organize.reorderCategories([1, 2]));

      expect(Exit.isFailure(exit)).toBe(true);
      expect((yield* fake.requests).at(-1)?.path).toBe("/sidebar");
      expect(view().categories.map((category) => category.id)).toEqual([2, 1, 5]);
      expect(overlayIsEmpty()).toBe(true);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("keep a newer synced row over the sidebar a 409 refetched from before it", () =>
    Effect.gen(function* () {
      const pinged = { ...general, unreadCount: 3, notificationCount: 1 };

      seed();
      mutations.applyEvents(
        [{ seq: 0, topic: "user:7", type: "sidebar.row.upserted", data: pinged }],
        0,
      );

      const fake = yield* FakeApi;

      yield* fake.route("PUT /room_categories/order", () =>
        Effect.fail(new Conflict({ message: "Stale" })),
      );
      yield* fake.route("GET /sidebar", () => {
        // The snapshot is read with the ping still counted; the read's row lands over sync
        // before the reply does.
        mutations.applyEvents(
          [
            {
              seq: 0,
              topic: "user:7",
              type: "sidebar.row.upserted",
              data: { ...general, unreadCount: 0, notificationCount: 0 },
            },
          ],
          0,
        );

        return Effect.succeed({
          ...sidebarFixture([pinged, design, engineering, ada]),
          categories: [launch, team],
        });
      });

      yield* Effect.exit(organize.reorderCategories([2, 1]));

      expect((yield* fake.requests).at(-1)?.path).toBe("/sidebar");
      expect(store.getState().sidebar.rows[1]?.notificationCount).toBe(0);
      expect(store.getState().sidebar.rows[1]?.unreadCount).toBe(0);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("read a 409 refetch again after a resync outdates it, then give up", () =>
    Effect.gen(function* () {
      const pinged = { ...general, unreadCount: 3, notificationCount: 1 };

      seed();
      mutations.applyEvents(
        [{ seq: 0, topic: "user:7", type: "sidebar.row.upserted", data: pinged }],
        0,
      );

      const fake = yield* FakeApi;

      yield* fake.route("PUT /room_categories/order", () =>
        Effect.fail(new Conflict({ message: "Stale" })),
      );
      yield* fake.route("GET /sidebar", () => {
        // While each read is on its way, a sync resync installs the read row (and no later
        // event says so again); then the read, older than the resync, arrives.
        mutations.resyncSidebar(
          {
            ...sidebarFixture([{ ...general, unreadCount: 0, notificationCount: 0 }, design]),
            categories: [launch, team],
          },
          sidebarRowClock(),
        );

        return Effect.succeed({
          ...sidebarFixture([pinged, design, engineering, ada]),
          categories: [launch, team],
        });
      });

      yield* Effect.exit(organize.reorderCategories([2, 1]));

      const reads = (yield* fake.requests).filter((request) => request.path === "/sidebar");

      expect(reads).toHaveLength(3);
      expect(store.getState().sidebar.rows[1]?.notificationCount).toBe(0);
      // The resync dropped these rooms; the older reads can't bring them back.
      expect(store.getState().sidebar.rows[3]).toBeUndefined();
      expect(store.getState().sidebar.rows[4]).toBeUndefined();
      expect(store.getState().rowTouches.at).toEqual({});
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("land a 409 refetch read again after a resync outdated the first", () =>
    Effect.gen(function* () {
      seed();

      const fake = yield* FakeApi;
      let reads = 0;

      yield* fake.route("PUT /room_categories/order", () =>
        Effect.fail(new Conflict({ message: "Stale" })),
      );
      yield* fake.route("GET /sidebar", () => {
        reads += 1;

        if (reads === 1) {
          mutations.resyncSidebar(
            { ...sidebarFixture([general, design]), categories: [launch, team] },
            sidebarRowClock(),
          );
        }

        return Effect.succeed({
          ...sidebarFixture([general, design, engineering]),
          categories: [
            { ...team, position: 1 },
            { ...launch, position: 2 },
          ],
        });
      });

      yield* Effect.exit(organize.reorderCategories([2, 1]));

      expect(reads).toBe(2);
      expect(store.getState().sidebar.rows[3]).toEqual(engineering);
      expect(store.getState().sidebar.categories.map((category) => category.id)).toEqual([2, 1]);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("reorder with a new category's real id when its create is still in flight", () =>
    Effect.gen(function* () {
      seed();

      const fake = yield* FakeApi;
      const gate = yield* Deferred.make<void>();
      const ops: RoomCategory = { id: 9, name: "Ops", collapsed: false, position: 3 };

      yield* fake.route("POST /room_categories", () => Deferred.await(gate).pipe(Effect.as(ops)));
      yield* fake.reply("PUT /room_categories/order", {
        categories: [
          { ...ops, position: 1 },
          { ...launch, position: 2 },
          { ...team, position: 3 },
        ],
      });

      const created = yield* Effect.forkChild(organize.createCategory("Ops"));

      yield* Effect.yieldNow;

      const temporaryId = view().categories.at(-1)?.id ?? 0;

      expect(temporaryId).toBeLessThan(0);

      const reordered = yield* Effect.forkChild(
        organize.reorderCategories([temporaryId, launch.id, team.id]),
      );

      yield* Effect.yieldNow;
      yield* Deferred.succeed(gate, undefined);
      yield* Fiber.join(created);
      yield* Fiber.join(reordered);

      expect((yield* fake.requests).at(-1)?.body).toEqual({ categoryIds: [9, 1, 2] });
      expect(view().categories.map((category) => category.id)).toEqual([9, 1, 2]);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("set the notification level on the row and the room header", () =>
    Effect.gen(function* () {
      seed();
      mutations.setRoomDetail(roomDetailFixture(1), beginRoomRequest());

      const server = createMockServer({ simulate: false });

      const settings = yield* Effect.promise(() =>
        server.handle({ method: "GET", path: "/api/v1/settings" }),
      );

      server.dispose();

      const fake = yield* FakeApi;
      let during: string | undefined;

      mutations.applyEvents(
        [
          {
            seq: 0,
            topic: "user:7",
            type: "sidebar.row.upserted",
            data: withMembership(general, { unreadAt: "2026-10-05T00:00:00.000Z" }),
          },
        ],
        0,
      );

      let unreadDuring: string | null | undefined;

      yield* fake.route("PUT /rooms/1/involvement", () => {
        during = view().rows[1]?.membership.involvement;
        // Muting marks the room read: it shows read at once too.
        unreadDuring = view().rows[1]?.membership.unreadAt;

        return Effect.succeed({
          membership: { ...general.membership, involvement: "muted" },
          settings: settings.json,
        });
      });

      yield* organize.setInvolvement(1, "muted");

      expect(during).toBe("muted");
      expect(unreadDuring).toBeNull();
      expect(store.getState().sidebar.rows[1]?.membership.involvement).toBe("muted");
      expect(store.getState().rooms[1]?.detail?.membership.involvement).toBe("muted");
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("give up a change the server never answers, so the next one runs", () =>
    Effect.gen(function* () {
      seed();

      const fake = yield* FakeApi;

      yield* fake.route("PUT /rooms/1/category", () => Effect.never);
      yield* fake.reply("PATCH /room_categories/2", { ...team, name: "People" });

      const stuck = yield* Effect.forkChild(
        organize.moveRoom(1, { kind: "category", categoryId: team.id }),
      );

      const renamed = yield* Effect.forkChild(organize.renameCategory(2, "People"));

      yield* Effect.yieldNow;

      expect(view().rows[1]?.membership.roomCategoryId).toBe(team.id);
      expect((yield* fake.requests).map((request) => request.path)).toEqual(["/rooms/1/category"]);

      yield* TestClock.adjust("15 seconds");

      const error = yield* Effect.flip(Fiber.join(stuck));

      // Undone, with a reason to show, and the rename that waited behind it went through.
      expect(error).toBeInstanceOf(NetworkError);
      expect(error.message).toContain("didn't answer");
      expect(view().rows[1]?.membership.roomCategoryId).toBeNull();

      yield* Fiber.join(renamed);

      expect(view().categories.find((category) => category.id === 2)?.name).toBe("People");
      expect(overlayIsEmpty()).toBe(true);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("leave a room where it is when it is already in the slot", () =>
    Effect.gen(function* () {
      seed();

      const fake = yield* FakeApi;

      yield* organize.moveRoom(2, { kind: "category", categoryId: launch.id });
      yield* organize.moveRoom(4, { kind: "unfavorite" });

      expect(yield* fake.requests).toEqual([]);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );
});
