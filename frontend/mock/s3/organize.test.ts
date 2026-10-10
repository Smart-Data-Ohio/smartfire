import { Schema } from "effect";
import { describe, expect, it } from "vitest";
import {
  FavoriteList,
  InvolvementChange as InvolvementChangeSchema,
  RoomCategoryList,
} from "../../src/api/schema/organize.ts";
import { RoomCategory, Sidebar, SidebarRow } from "../../src/api/schema/sidebar.ts";
import { SyncEvent } from "../../src/api/schema/sync.ts";
import type { FavoriteList as FavoriteListType } from "../../src/gen/FavoriteList.ts";
import type { InvolvementChange } from "../../src/gen/InvolvementChange.ts";
import type { RoomCategory as RoomCategoryType } from "../../src/gen/RoomCategory.ts";
import type { RoomCategoryList as RoomCategoryListType } from "../../src/gen/RoomCategoryList.ts";
import type { Sidebar as SidebarType } from "../../src/gen/Sidebar.ts";
import type { SidebarRow as SidebarRowType } from "../../src/gen/SidebarRow.ts";
import { type Json, parseJson } from "../json.ts";
import { collect, errorOf, expectStatus, get, harness, send } from "../s2/testing.ts";
import { CATEGORY_IDS, ROOM_IDS } from "../seed.ts";

const sidebarOf = (server: ReturnType<typeof harness>["server"]) =>
  get<SidebarType>(server, "/api/v1/sidebar");

const rowOf = async (server: ReturnType<typeof harness>["server"], roomId: number) =>
  (await sidebarOf(server)).rows.find((row) => row.room.id === roomId);

const favoriteIds = async (server: ReturnType<typeof harness>["server"]) =>
  (await sidebarOf(server)).rows
    .filter((row) => row.membership.favoritePosition !== null)
    .sort(
      (left, right) =>
        (left.membership.favoritePosition ?? 0) - (right.membership.favoritePosition ?? 0) ||
        left.membership.id - right.membership.id,
    )
    .map((row) => row.room.id);

const decodes = (schema: Schema.Decoder<unknown>, json: Json) => {
  expect(() => Schema.decodeUnknownSync(schema)(json)).not.toThrow();
};

describe("mock sidebar organisation", () => {
  describe("categories", () => {
    it("seeds Launch and Team in order", async () => {
      const { server } = harness();
      const sidebar = await sidebarOf(server);

      expect(sidebar.categories.map((category) => [category.id, category.position])).toEqual([
        [CATEGORY_IDS.launch, 1],
        [CATEGORY_IDS.team, 2],
      ]);
    });

    it("creates at the end and publishes the category", async () => {
      const { server } = harness();
      const events = collect(server);

      const created = await expectStatus<RoomCategoryType>(
        server,
        "POST",
        "/api/v1/room_categories",
        { name: "Ops" },
        201,
      );

      expect(created).toMatchObject({ name: "Ops", collapsed: false, position: 3 });
      expect(events.map((event) => event.type)).toEqual(["sidebar.category.upserted"]);
      expect((await sidebarOf(server)).categories.at(-1)).toEqual(created);
    });

    it("refuses a blank or long name with 422", async () => {
      const { server } = harness();

      const blank = await send(server, "POST", "/api/v1/room_categories", { name: "   " });
      const long = await send(server, "POST", "/api/v1/room_categories", { name: "x".repeat(51) });

      expect(blank.status).toBe(422);
      expect(long.status).toBe(422);
      expect(errorOf(long.json).tag).toBe("Validation");

      await expectStatus(server, "POST", "/api/v1/room_categories", { name: "x".repeat(50) }, 201);
    });

    it("updates only the fields sent", async () => {
      const { server } = harness();
      const path = `/api/v1/room_categories/${CATEGORY_IDS.team}`;

      const folded = await expectStatus<RoomCategoryType>(
        server,
        "PATCH",
        path,
        { collapsed: true },
        200,
      );

      expect(folded).toMatchObject({ name: "Team", collapsed: true });

      const renamed = await expectStatus<RoomCategoryType>(
        server,
        "PATCH",
        path,
        { name: "People" },
        200,
      );

      expect(renamed).toMatchObject({ name: "People", collapsed: true, position: 2 });

      const missing = await send(server, "PATCH", "/api/v1/room_categories/99", { name: "A" });

      expect(missing.status).toBe(404);
    });

    it("deletes: rooms go back to Channels, their rows first, then the removal", async () => {
      const { server } = harness();
      const events = collect(server);

      await expectStatus(
        server,
        "DELETE",
        `/api/v1/room_categories/${CATEGORY_IDS.launch}`,
        null,
        204,
      );

      expect(events.map((event) => event.type)).toEqual([
        "sidebar.row.upserted",
        "sidebar.row.upserted",
        "sidebar.category.removed",
      ]);

      expect((await rowOf(server, ROOM_IDS.design))?.membership.roomCategoryId).toBeNull();
      expect((await sidebarOf(server)).categories.map((category) => category.name)).toEqual([
        "Team",
      ]);
    });

    it("reorders an exact permutation and answers 409 for a stale list", async () => {
      const { server } = harness();
      const events = collect(server);

      const list = await expectStatus<RoomCategoryListType>(
        server,
        "PUT",
        "/api/v1/room_categories/order",
        { categoryIds: [CATEGORY_IDS.team, CATEGORY_IDS.launch] },
        200,
      );

      expect(list.categories.map((category) => [category.id, category.position])).toEqual([
        [CATEGORY_IDS.team, 1],
        [CATEGORY_IDS.launch, 2],
      ]);
      expect(events.map((event) => event.type)).toEqual([
        "sidebar.category.upserted",
        "sidebar.category.upserted",
      ]);

      for (const categoryIds of [[CATEGORY_IDS.team], [1, 2, 3], [1, 1], [2, 99]]) {
        const stale = await send(server, "PUT", "/api/v1/room_categories/order", { categoryIds });

        expect(stale.status, JSON.stringify(categoryIds)).toBe(409);
        expect(errorOf(stale.json).tag).toBe("Conflict");
      }
    });
  });

  describe("placement", () => {
    it("assigns a channel to a category and back", async () => {
      const { server } = harness();
      const path = `/api/v1/rooms/${ROOM_IDS.general}/category`;

      const row = await expectStatus<SidebarRowType>(
        server,
        "PUT",
        path,
        { roomCategoryId: CATEGORY_IDS.team },
        200,
      );

      expect(row.membership.roomCategoryId).toBe(CATEGORY_IDS.team);

      const out = await expectStatus<SidebarRowType>(
        server,
        "PUT",
        path,
        { roomCategoryId: null },
        200,
      );

      expect(out.membership.roomCategoryId).toBeNull();
    });

    it("refuses direct and voice rooms (422) and unknown categories (404)", async () => {
      const { server } = harness();

      const direct = await send(server, "PUT", `/api/v1/rooms/${ROOM_IDS.dmMaya}/category`, {
        roomCategoryId: CATEGORY_IDS.team,
      });

      const voice = await send(server, "PUT", `/api/v1/rooms/${ROOM_IDS.lounge}/category`, {
        roomCategoryId: CATEGORY_IDS.team,
      });

      const unknown = await send(server, "PUT", `/api/v1/rooms/${ROOM_IDS.general}/category`, {
        roomCategoryId: 99,
      });

      expect([direct.status, voice.status, unknown.status]).toEqual([422, 422, 404]);
    });

    it("favourites append, unfavourites leave a gap, a move renumbers from 0", async () => {
      const { server } = harness();

      expect(await favoriteIds(server)).toEqual([ROOM_IDS.engineering, ROOM_IDS.dmMaya]);

      const starred = await expectStatus<SidebarRowType>(
        server,
        "POST",
        `/api/v1/rooms/${ROOM_IDS.general}/favorite`,
        null,
        200,
      );

      expect(starred.membership.favoritePosition).toBe(3);

      // Idempotent.
      await expectStatus(server, "POST", `/api/v1/rooms/${ROOM_IDS.general}/favorite`, null, 200);
      expect((await rowOf(server, ROOM_IDS.general))?.membership.favoritePosition).toBe(3);

      await expectStatus(
        server,
        "DELETE",
        `/api/v1/rooms/${ROOM_IDS.engineering}/favorite`,
        null,
        200,
      );

      const gapped = await sidebarOf(server);

      expect(
        gapped.rows.find((row) => row.room.id === ROOM_IDS.dmMaya)?.membership.favoritePosition,
      ).toBe(2);

      const events = collect(server);

      const moved = await expectStatus<FavoriteListType>(
        server,
        "PATCH",
        `/api/v1/rooms/${ROOM_IDS.general}/favorite`,
        { position: 0 },
        200,
      );

      expect(moved.rows.map((row) => [row.room.id, row.membership.favoritePosition])).toEqual([
        [ROOM_IDS.general, 0],
        [ROOM_IDS.dmMaya, 1],
      ]);
      expect(events.every((event) => event.type === "sidebar.row.upserted")).toBe(true);
      expect(events).toHaveLength(2);
    });

    it("clamps a move and leaves a non-favourite alone", async () => {
      const { server } = harness();

      const clamped = await expectStatus<FavoriteListType>(
        server,
        "PATCH",
        `/api/v1/rooms/${ROOM_IDS.engineering}/favorite`,
        { position: 40 },
        200,
      );

      expect(clamped.rows.map((row) => row.room.id)).toEqual([
        ROOM_IDS.dmMaya,
        ROOM_IDS.engineering,
      ]);

      const events = collect(server);

      const untouched = await expectStatus<FavoriteListType>(
        server,
        "PATCH",
        `/api/v1/rooms/${ROOM_IDS.general}/favorite`,
        { position: 0 },
        200,
      );

      expect(untouched.rows.map((row) => row.room.id)).toEqual([
        ROOM_IDS.dmMaya,
        ROOM_IDS.engineering,
      ]);
      expect(events).toEqual([]);
    });

    it("organises a hidden room without a row; a move counts shown favourites only", async () => {
      const { server } = harness();

      const hide = (roomId: number, involvement: string) =>
        expectStatus(server, "PUT", `/api/v1/rooms/${roomId}/involvement`, { involvement }, 200);

      await hide(ROOM_IDS.dmMaya, "invisible");
      await hide(ROOM_IDS.quiet, "invisible");

      const events = collect(server);

      await expectStatus(
        server,
        "PUT",
        `/api/v1/rooms/${ROOM_IDS.quiet}/category`,
        { roomCategoryId: CATEGORY_IDS.team },
        200,
      );
      expect(events).toEqual([]);

      await expectStatus(server, "POST", `/api/v1/rooms/${ROOM_IDS.general}/favorite`, null, 200);

      // Engineering 0, Maya 1 (hidden), General 2: General to shown index 0 goes before
      // Engineering, and Maya keeps its place after it.
      const moved = await expectStatus<FavoriteListType>(
        server,
        "PATCH",
        `/api/v1/rooms/${ROOM_IDS.general}/favorite`,
        { position: 0 },
        200,
      );

      expect(moved.rows.map((row) => row.room.id)).toEqual([
        ROOM_IDS.general,
        ROOM_IDS.engineering,
      ]);

      // Unfavouriting the hidden room succeeds and publishes nothing for it.
      events.length = 0;

      await expectStatus(server, "DELETE", `/api/v1/rooms/${ROOM_IDS.dmMaya}/favorite`, null, 200);
      expect(events).toEqual([]);

      await hide(ROOM_IDS.dmMaya, "everything");
      expect((await rowOf(server, ROOM_IDS.dmMaya))?.membership.favoritePosition).toBeNull();
    });
  });

  describe("involvement", () => {
    it("muting marks the room read and publishes room.read with the row", async () => {
      const { server } = harness();
      const events = collect(server);

      const { membership } = await expectStatus<InvolvementChange>(
        server,
        "PUT",
        `/api/v1/rooms/${ROOM_IDS.design}/involvement`,
        { involvement: "muted" },
        200,
      );

      expect(membership.involvement).toBe("muted");
      expect(membership.unreadAt).toBeNull();
      expect(events.map((event) => event.type)).toEqual(["room.read", "sidebar.row.upserted"]);
      expect((await rowOf(server, ROOM_IDS.design))?.unreadCount).toBe(0);
    });

    it("invisible removes the row; any other level brings it back", async () => {
      const { server } = harness();
      const events = collect(server);
      const path = `/api/v1/rooms/${ROOM_IDS.quiet}/involvement`;

      await expectStatus(server, "PUT", path, { involvement: "invisible" }, 200);

      expect(events.map((event) => event.type)).toEqual(["sidebar.row.removed"]);
      expect(await rowOf(server, ROOM_IDS.quiet)).toBeUndefined();

      await expectStatus(server, "PUT", path, { involvement: "everything" }, 200);

      expect(events.map((event) => event.type)).toEqual([
        "sidebar.row.removed",
        "sidebar.row.upserted",
      ]);
      expect((await rowOf(server, ROOM_IDS.quiet))?.membership.involvement).toBe("everything");
    });

    it("refuses an unknown level", async () => {
      const { server } = harness();

      const response = await send(server, "PUT", `/api/v1/rooms/${ROOM_IDS.general}/involvement`, {
        involvement: "loud",
      });

      expect(response.status).toBe(422);
    });
  });

  it("answers and publishes what the pinned schemas decode", async () => {
    const { server } = harness();
    const events = collect(server);

    const created = await send(server, "POST", "/api/v1/room_categories", { name: "Ops" });

    decodes(RoomCategory, created.json);

    decodes(
      RoomCategoryList,
      (
        await send(server, "PUT", "/api/v1/room_categories/order", {
          categoryIds: [3, CATEGORY_IDS.team, CATEGORY_IDS.launch],
        })
      ).json,
    );

    decodes(
      SidebarRow,
      (
        await send(server, "PUT", `/api/v1/rooms/${ROOM_IDS.general}/category`, {
          roomCategoryId: 3,
        })
      ).json,
    );

    decodes(
      SidebarRow,
      (await send(server, "POST", `/api/v1/rooms/${ROOM_IDS.quiet}/favorite`)).json,
    );

    decodes(
      FavoriteList,
      (await send(server, "PATCH", `/api/v1/rooms/${ROOM_IDS.quiet}/favorite`, { position: 0 }))
        .json,
    );

    decodes(
      InvolvementChangeSchema,
      (
        await send(server, "PUT", `/api/v1/rooms/${ROOM_IDS.general}/involvement`, {
          involvement: "muted",
        })
      ).json,
    );

    await send(server, "DELETE", "/api/v1/room_categories/3");
    decodes(Sidebar, (await server.handle({ method: "GET", path: "/api/v1/sidebar" })).json);

    expect(new Set(events.map((event) => event.type))).toEqual(
      new Set([
        "sidebar.category.upserted",
        "sidebar.category.removed",
        "sidebar.row.upserted",
        "room.read",
      ]),
    );

    for (const event of events) {
      decodes(SyncEvent, parseJson(JSON.stringify(event)) ?? null);
    }
  });
});
