import { afterEach, describe, expect, it } from "@effect/vitest";
import { Deferred, Effect, Exit, Fiber } from "effect";
import { Conflict, Validation } from "../api/errors.ts";
import {
  FakeApi,
  meFixture,
  roomDetailFixture,
  sidebarFixture,
  sidebarRowFixture,
} from "../api/testing.ts";
import type { RoomCategory, SidebarRow } from "../store/model.ts";
import { favoriteRows, organizedSidebar } from "../store/organize.ts";
import { mutations, store } from "../store/store.ts";
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
  mutations.loadSidebar({
    ...sidebarFixture([general, design, engineering, ada]),
    categories: [launch, team],
  });
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

      yield* fake.reply("PUT /rooms/1/category", withMembership(general, { roomCategoryId: 9 }));

      const created = yield* organize.createCategory("  Ops ", 1);
      const requests = yield* fake.requests;

      expect(shown.at(-1)).toMatchObject({ name: "Ops", position: 3 });
      expect(shown.at(-1)?.id).toBeLessThan(0);
      expect(placedIn).toBe(shown.at(-1)?.id);
      expect(created.id).toBe(9);
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
      mutations.setRoomDetail(roomDetailFixture(1));

      const fake = yield* FakeApi;
      let during: string | undefined;

      yield* fake.route("PUT /rooms/1/involvement", () => {
        during = view().rows[1]?.membership.involvement;

        return Effect.succeed({ ...general.membership, involvement: "muted" });
      });

      yield* organize.setInvolvement(1, "muted");

      expect(during).toBe("muted");
      expect(store.getState().sidebar.rows[1]?.membership.involvement).toBe("muted");
      expect(store.getState().rooms[1]?.detail?.membership.involvement).toBe("muted");
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
