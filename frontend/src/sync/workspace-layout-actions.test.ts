import { afterEach, describe, expect, it } from "@effect/vitest";
import { Effect, Exit } from "effect";
import { Conflict, Validation } from "../api/errors.ts";
import { FakeApi, meFixture, sidebarFixture, sidebarRowFixture } from "../api/testing.ts";
import type { WorkspaceCategory } from "../gen/WorkspaceCategory.ts";
import type { WorkspaceLayout } from "../gen/WorkspaceLayout.ts";
import { mutations, sidebarRowClock, store } from "../store/store.ts";
import { groupRooms, sharedCategories, sharedLayout } from "../store/workspace-layout.ts";
import * as workspaceLayout from "./workspace-layout-actions.ts";

const product: WorkspaceCategory = { id: 30, name: "Product", position: 0 };

const ops: WorkspaceCategory = { id: 31, name: "Ops", position: 1 };

const rows = [
  sidebarRowFixture(1, "general"),
  sidebarRowFixture(2, "design"),
  sidebarRowFixture(3, "engineering"),
];

/** #design and #general in Product; #engineering in no category. */
const layout: WorkspaceLayout = {
  categories: [product, ops],
  rooms: [
    { roomId: 2, workspaceCategoryId: 30, position: 0 },
    { roomId: 1, workspaceCategoryId: 30, position: 1 },
    { roomId: 3, workspaceCategoryId: null, position: null },
  ],
};

function seed(): void {
  mutations.reset();
  mutations.setMe(meFixture);
  mutations.loadSidebar({ ...sidebarFixture(rows), workspaceLayout: layout }, sidebarRowClock());
}

/** The layout as the administrator sees it, pending changes included. */
const view = () => sharedLayout(store.getState().sidebar);

const ids = (categoryId: number | null) =>
  groupRooms(view(), categoryId).map((room) => room.roomId);

const names = () => sharedCategories(view()).map((category) => category.name);

const overlay = () => store.getState().sidebar.workspaceOverlay;

describe("shared layout actions", () => {
  afterEach(() => mutations.reset());

  it.effect("show a moved room at once and keep the layout the server answers", () =>
    Effect.gen(function* () {
      seed();

      const fake = yield* FakeApi;
      let during: readonly number[] = [];

      const answer: WorkspaceLayout = {
        categories: [product, ops],
        rooms: [
          { roomId: 2, workspaceCategoryId: 30, position: 0 },
          { roomId: 3, workspaceCategoryId: 30, position: 1 },
          { roomId: 1, workspaceCategoryId: 30, position: 2 },
        ],
      };

      yield* fake.route("PUT /rooms/3/workspace_category", () => {
        during = ids(30);

        return Effect.succeed(answer);
      });

      yield* workspaceLayout.moveRoom(3, 30, 1);

      expect(during).toEqual([2, 3, 1]);
      expect((yield* fake.requests).at(-1)?.body).toEqual({ workspaceCategoryId: 30, position: 1 });
      expect(store.getState().sidebar.workspaceLayout).toEqual(answer);
      expect(overlay()).toBeNull();
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("put the room back when the server refuses the move", () =>
    Effect.gen(function* () {
      seed();

      const fake = yield* FakeApi;
      let during: readonly number[] = [];

      yield* fake.route("PUT /rooms/2/workspace_category", () => {
        during = ids(null);

        return Effect.fail(new Validation({ message: "Nope", fields: {} }));
      });

      const exit = yield* Effect.exit(workspaceLayout.moveRoom(2, null, null));

      expect(Exit.isFailure(exit)).toBe(true);
      expect(during).toEqual([3, 2]);
      expect(ids(30)).toEqual([2, 1]);
      expect(ids(null)).toEqual([3]);
      expect(overlay()).toBeNull();
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("send nothing for a room dropped where it already is", () =>
    Effect.gen(function* () {
      seed();

      const fake = yield* FakeApi;

      yield* workspaceLayout.moveRoom(2, 30, 1);

      expect(yield* fake.requests).toEqual([]);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("show a new category under a temporary id until the server names it", () =>
    Effect.gen(function* () {
      seed();

      const fake = yield* FakeApi;
      let during: readonly WorkspaceCategory[] = [];
      const created: WorkspaceCategory = { id: 32, name: "Support", position: 2 };

      yield* fake.route("POST /workspace_categories", () => {
        during = sharedCategories(view());

        return Effect.succeed(created);
      });

      const result = yield* workspaceLayout.createCategory("  Support ");

      expect(result).toEqual(created);
      expect(during.at(-1)).toMatchObject({ name: "Support", position: 2 });
      expect(during.at(-1)?.id).toBeLessThan(0);
      expect(sharedCategories(view())).toEqual([product, ops, created]);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("bring a deleted category back when the server refuses", () =>
    Effect.gen(function* () {
      seed();

      const fake = yield* FakeApi;
      let during: readonly string[] = [];

      yield* fake.route("DELETE /workspace_categories/30", () => {
        during = names();

        return Effect.fail(new Validation({ message: "Nope", fields: {} }));
      });

      yield* Effect.exit(workspaceLayout.deleteCategory(30));

      expect(during).toEqual(["Ops"]);
      expect(names()).toEqual(["Product", "Ops"]);
      expect(ids(30)).toEqual([2, 1]);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("send a deleted category's rooms after the uncategorized ones", () =>
    Effect.gen(function* () {
      seed();

      const fake = yield* FakeApi;

      yield* fake.reply("DELETE /workspace_categories/30", null);
      yield* workspaceLayout.deleteCategory(30);

      expect(names()).toEqual(["Ops"]);
      expect(sharedCategories(view())[0]?.position).toBe(0);
      expect(ids(null)).toEqual([3, 2, 1]);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("refetch the sidebar and fail when the category order was stale", () =>
    Effect.gen(function* () {
      seed();

      const fake = yield* FakeApi;
      const support: WorkspaceCategory = { id: 32, name: "Support", position: 2 };

      yield* fake.route("PUT /workspace_categories/order", () =>
        Effect.fail(new Conflict({ message: "Stale" })),
      );
      yield* fake.reply("GET /sidebar", {
        ...sidebarFixture(rows),
        workspaceLayout: { ...layout, categories: [product, ops, support] },
      });

      const exit = yield* Effect.exit(workspaceLayout.reorderCategories([31, 30]));

      expect(Exit.isFailure(exit)).toBe(true);
      expect((yield* fake.requests).at(-1)?.path).toBe("/sidebar");
      expect(names()).toEqual(["Product", "Ops", "Support"]);
      expect(overlay()).toBeNull();
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("keep a newer layout a sync event brought while the reply was on its way", () =>
    Effect.gen(function* () {
      seed();

      const fake = yield* FakeApi;
      const synced: WorkspaceLayout = { categories: [ops], rooms: layout.rooms };

      yield* fake.route("PATCH /workspace_categories/30", () => {
        mutations.applyEvents(
          [{ seq: 0, topic: "user:7", type: "workspace.layout.updated", data: synced }],
          0,
        );

        return Effect.succeed({ ...product, name: "Make" });
      });

      yield* workspaceLayout.renameCategory(30, "Make");

      expect(names()).toEqual(["Ops"]);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );
});
