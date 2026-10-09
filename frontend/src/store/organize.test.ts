import { describe, expect, it } from "vitest";
import { roomDetailFixture, sidebarFixture, sidebarRowFixture } from "../api/testing.ts";
import type { RoomCategory, SidebarRow } from "./model.ts";
import {
  addOverlay,
  dropOverlay,
  favoriteRows,
  isInSlot,
  organizedSidebar,
  placementPatches,
  removeCategory,
  type SidebarOverlay,
  setMembership,
} from "./organize.ts";
import { applyEvents, loadSidebar, setRoomDetail } from "./reducers.ts";
import { rowClock } from "./row-touches.ts";
import { initialState, type State } from "./state.ts";

const launch: RoomCategory = { id: 1, name: "Launch", collapsed: false, position: 1 };

const team: RoomCategory = { id: 2, name: "Team", collapsed: false, position: 2 };

function withMembership(row: SidebarRow, change: Partial<SidebarRow["membership"]>): SidebarRow {
  return { ...row, membership: { ...row.membership, ...change } };
}

function seeded(): State {
  return loadSidebar(
    initialState,
    {
      ...sidebarFixture([
        sidebarRowFixture(1, "general"),
        withMembership(sidebarRowFixture(2, "design"), { roomCategoryId: launch.id }),
        withMembership(sidebarRowFixture(3, "engineering"), { favoritePosition: 4 }),
        withMembership(sidebarRowFixture(4, "Ada", "direct"), { favoritePosition: 4 }),
      ]),
      categories: [team, launch],
    },
    0,
  );
}

const event = <T extends Parameters<typeof applyEvents>[1][number]["type"]>(
  type: T,
  data: Extract<Parameters<typeof applyEvents>[1][number], { type: T }>["data"],
) => ({ seq: 1, topic: "user:7", type, data });

describe("sidebar organisation reducers", () => {
  it("orders favourites by position, then membership id", () => {
    expect(favoriteRows(seeded().sidebar).map((row) => row.room.id)).toEqual([3, 4]);
  });

  it("lands category events, keeping (position, id) order", () => {
    const state = applyEvents(
      seeded(),
      [
        event("sidebar.category.upserted", { id: 3, name: "Ops", collapsed: true, position: 0 }),
        event("sidebar.category.upserted", { ...team, name: "People" }),
      ],
      0,
    );

    expect(state.sidebar.categories.map((category) => category.name)).toEqual([
      "Ops",
      "Launch",
      "People",
    ]);
  });

  it("sends a removed category's rooms back to Channels", () => {
    const state = applyEvents(seeded(), [event("sidebar.category.removed", { id: 1 })], 0);

    expect(state.sidebar.categories.map((category) => category.id)).toEqual([2]);
    expect(state.sidebar.rows[2]?.membership.roomCategoryId).toBeNull();
    expect(removeCategory(seeded(), 9).sidebar.rows[2]?.membership.roomCategoryId).toBe(1);
  });

  it("copies a row's membership onto the room header", () => {
    const state = setRoomDetail(seeded(), roomDetailFixture(1));
    const row = withMembership(sidebarRowFixture(1, "general"), { involvement: "muted" });
    const next = applyEvents(state, [event("sidebar.row.upserted", row)], 0);

    expect(next.rooms[1]?.detail?.membership.involvement).toBe("muted");

    const replied = setMembership(
      state,
      { ...row.membership, involvement: "nothing" },
      rowClock(state),
    );

    expect(replied.sidebar.rows[1]?.membership.involvement).toBe("nothing");
    expect(replied.rooms[1]?.detail?.membership.involvement).toBe("nothing");
  });

  it("keeps the overlay across a sidebar reload", () => {
    const entry: SidebarOverlay = { memberships: { 1: { favoritePosition: 0 } }, categories: {} };
    const state = addOverlay(seeded(), entry);

    const reloaded = loadSidebar(
      state,
      sidebarFixture([sidebarRowFixture(1, "general")]),
      rowClock(state),
    );

    expect(organizedSidebar(reloaded.sidebar).rows[1]?.membership.favoritePosition).toBe(0);
  });

  it("draws the overlay and drops only the values an entry put there", () => {
    const first: SidebarOverlay = {
      memberships: { 1: { favoritePosition: 0 } },
      categories: { [-1]: { id: -1, name: "Draft", collapsed: false, position: 3 } },
    };

    const second: SidebarOverlay = { memberships: { 1: { roomCategoryId: 2 } }, categories: {} };
    const both = addOverlay(addOverlay(seeded(), first), second);
    const view = organizedSidebar(both.sidebar);

    expect(view.rows[1]?.membership).toMatchObject({ favoritePosition: null, roomCategoryId: 2 });
    expect(view.categories.map((category) => category.id)).toEqual([1, 2, -1]);
    expect(organizedSidebar(both.sidebar)).toBe(view);

    const afterFirst = dropOverlay(both, first);

    expect(afterFirst.sidebar.overlay.memberships).toEqual(second.memberships);
    expect(afterFirst.sidebar.overlay.categories).toEqual({});

    const settled = dropOverlay(afterFirst, second);

    expect(organizedSidebar(settled.sidebar)).toBe(settled.sidebar);
  });

  it("renumbers every favourite for a favourite slot, as the server's move does", () => {
    const view = seeded().sidebar;

    expect(placementPatches(view, 1, { kind: "favorite", index: 1 })).toEqual({
      3: { favoritePosition: 0 },
      1: { favoritePosition: 1 },
      4: { favoritePosition: 2 },
    });
    expect(placementPatches(view, 4, { kind: "favorite", index: 9 })).toEqual({
      3: { favoritePosition: 0 },
      4: { favoritePosition: 1 },
    });
    expect(placementPatches(view, 3, { kind: "channels" })).toEqual({
      3: { favoritePosition: null, roomCategoryId: null },
    });
  });

  it("knows when a room already sits in a slot", () => {
    const view = seeded().sidebar;
    const rows = view.rows;

    expect(rows[3] && isInSlot(view, rows[3], { kind: "favorite", index: 0 })).toBe(true);
    expect(rows[3] && isInSlot(view, rows[3], { kind: "favorite", index: 1 })).toBe(false);
    expect(rows[2] && isInSlot(view, rows[2], { kind: "category", categoryId: 1 })).toBe(true);
    expect(rows[1] && isInSlot(view, rows[1], { kind: "channels" })).toBe(true);
    expect(rows[1] && isInSlot(view, rows[1], { kind: "category", categoryId: 1 })).toBe(false);
  });
});
