import { describe, expect, it } from "vitest";

import { decodeSidebar, sidebarFixture, sidebarRowFixture } from "../api/testing.ts";
import type { WorkspaceLayout } from "../gen/WorkspaceLayout.ts";
import { applyEvents, loadSidebar, setRoomUnavailable } from "./reducers.ts";
import { markResynced, rowClock } from "./row-touches.ts";
import { initialState } from "./state.ts";
import {
  addWorkspaceOverlay,
  dropWorkspaceOverlay,
  END_OF_GROUP,
  groupRooms,
  isPlacedAt,
  landWorkspaceChange,
  moveRequest,
  sharedCategories,
  sharedLayout,
  withCategoriesOrdered,
  withCategory,
  withoutCategory,
  withRoomMoved,
} from "./workspace-layout.ts";

describe("shared workspace layout", () => {
  it("decodes the snapshot and keeps the shared layout separately from personal categories", () => {
    const layout = {
      categories: [{ id: 9, name: "Team", position: 0 }],
      rooms: [{ roomId: 12, workspaceCategoryId: 9, position: 0 }],
    };

    const decoded = decodeSidebar({ ...sidebarFixture([]), workspaceLayout: layout });
    const state = loadSidebar(initialState, decoded, 0);
    expect(state.sidebar.workspaceLayout).toEqual(layout);
    expect(state.sidebar.categories).toEqual([]);
  });
  it("consumes live changes without overwriting personal categories and ignores older snapshots", () => {
    const personal = { id: 1, name: "Mine", collapsed: true, position: 1 };
    const snapshot = { ...sidebarFixture([]), categories: [personal] };
    const loaded = loadSidebar(initialState, snapshot, 0);
    const since = rowClock(loaded);
    const layout = { categories: [{ id: 9, name: "Team", position: 0 }], rooms: [] };

    const live = applyEvents(
      loaded,
      [{ seq: 1, topic: "user:7", type: "workspace.layout.updated", data: layout }],
      0,
    );

    expect(live.sidebar.workspaceLayout).toEqual(layout);
    expect(live.sidebar.categories).toEqual([personal]);
    expect(loadSidebar(live, snapshot, since).sidebar.workspaceLayout).toEqual(layout);
    expect(loadSidebar(live, snapshot, rowClock(live)).sidebar.workspaceLayout).toEqual(
      snapshot.workspaceLayout,
    );
  });
  it("drops shared assignments when room access or sidebar visibility is lost", () => {
    const layout = {
      categories: [{ id: 9, name: "Team", position: 0 }],
      rooms: [{ roomId: 12, workspaceCategoryId: 9, position: 0 }],
    };

    const state = loadSidebar(
      initialState,
      { ...sidebarFixture([sidebarRowFixture(12, "private")]), workspaceLayout: layout },
      0,
    );

    const removed = applyEvents(
      state,
      [{ seq: 1, topic: "user:7", type: "sidebar.row.removed", data: { roomId: 12 } }],
      0,
    );

    expect(removed.sidebar.workspaceLayout.rooms).toEqual([]);
  });
  it("accepts a new shared snapshot after local access loss", () => {
    const snapshot = {
      ...sidebarFixture([sidebarRowFixture(12, "room")]),
      workspaceLayout: {
        categories: [],
        rooms: [{ roomId: 12, workspaceCategoryId: null, position: 0 }],
      },
    };

    const unavailable = setRoomUnavailable(loadSidebar(initialState, snapshot, 0), 12);

    expect(
      loadSidebar(unavailable, snapshot, rowClock(unavailable)).sidebar.workspaceLayout,
    ).toEqual(snapshot.workspaceLayout);
  });
});

const team = { id: 9, name: "Team", position: 0 };

const ops = { id: 10, name: "Ops", position: 1 };

/** Team holds 1, 2, 3 (organized); 4 and 5 are uncategorized, 5 never organized. */
const organized: WorkspaceLayout = {
  categories: [team, ops],
  rooms: [
    { roomId: 4, workspaceCategoryId: null, position: 0 },
    { roomId: 5, workspaceCategoryId: null, position: null },
    { roomId: 1, workspaceCategoryId: 9, position: 0 },
    { roomId: 2, workspaceCategoryId: 9, position: 1 },
    { roomId: 3, workspaceCategoryId: 9, position: 2 },
  ],
};

const ids = (layout: WorkspaceLayout, categoryId: number | null) =>
  groupRooms(layout, categoryId).map((room) => room.roomId);

describe("shared layout transforms", () => {
  it("orders a group by position, with rooms never organized last in the server's order", () => {
    expect(ids(organized, null)).toEqual([4, 5]);
    expect(ids(organized, 9)).toEqual([1, 2, 3]);
    expect(sharedCategories({ ...organized, categories: [ops, team] })).toEqual([team, ops]);
  });

  it("moves a room within its group and between groups, renumbering both", () => {
    const within = withRoomMoved(organized, 3, 9, 1);

    expect(ids(within, 9)).toEqual([3, 1, 2]);
    expect(groupRooms(within, 9).map((room) => room.position)).toEqual([0, 1, 2]);

    const across = withRoomMoved(organized, 2, null, 5);

    expect(ids(across, 9)).toEqual([1, 3]);
    expect(groupRooms(across, 9).map((room) => room.position)).toEqual([0, 1]);
    expect(ids(across, null)).toEqual([4, 2, 5]);
    expect(ids(withRoomMoved(organized, 4, 10, null), 10)).toEqual([4]);
  });

  it("asks the server to place a room before its neighbour, or at the very end", () => {
    // Room 7 at position 1 is in Team but not in this administrator's layout: the server
    // places by room, so it keeps its place.
    const gapped: WorkspaceLayout = {
      categories: [team],
      rooms: [
        { roomId: 1, workspaceCategoryId: 9, position: 0 },
        { roomId: 2, workspaceCategoryId: 9, position: 2 },
        { roomId: 4, workspaceCategoryId: null, position: null },
      ],
    };

    expect(moveRequest(gapped, 4, 9, 2)).toEqual({
      workspaceCategoryId: 9,
      position: 0,
      beforeRoomId: 2,
    });
    expect(moveRequest(gapped, 4, 9, null)).toEqual({
      workspaceCategoryId: 9,
      position: END_OF_GROUP,
    });
    // A neighbour that has left the group since the drop: the end rather than a refusal.
    expect(moveRequest(gapped, 1, null, 2)).toEqual({
      workspaceCategoryId: null,
      position: END_OF_GROUP,
    });
  });

  it("knows a drop that leaves a room in place", () => {
    expect(isPlacedAt(organized, 2, 9, 3)).toBe(true);
    expect(isPlacedAt(organized, 3, 9, null)).toBe(true);
    expect(isPlacedAt(organized, 3, 9, 2)).toBe(false);
    expect(isPlacedAt(organized, 3, null, null)).toBe(false);
  });

  it("reorders, renames and deletes categories as the server does", () => {
    expect(sharedCategories(withCategoriesOrdered(organized, [10, 9]))).toEqual([
      { ...ops, position: 0 },
      { ...team, position: 1 },
    ]);
    expect(withCategory(organized, { ...team, name: "People" }).categories).toEqual([
      { ...team, name: "People" },
      ops,
    ]);

    const deleted = withoutCategory(organized, 9);

    expect(deleted.categories).toEqual([{ ...ops, position: 0 }]);
    expect(ids(deleted, null)).toEqual([4, 5, 1, 2, 3]);
    expect(groupRooms(deleted, null).map((room) => room.position)).toEqual([0, 1, 2, 3, 4]);
  });
});

describe("shared layout overlay and replies", () => {
  const loaded = () =>
    loadSidebar(initialState, { ...sidebarFixture([]), workspaceLayout: organized }, 0);

  it("draws a pending change and drops only its own overlay", () => {
    const first = withRoomMoved(organized, 3, 9, 1);
    const second = withRoomMoved(first, 4, 9, null);
    const both = addWorkspaceOverlay(addWorkspaceOverlay(loaded(), first), second);

    expect(sharedLayout(both.sidebar)).toBe(second);
    expect(sharedLayout(dropWorkspaceOverlay(both, first).sidebar)).toBe(second);
    expect(sharedLayout(dropWorkspaceOverlay(both, second).sidebar)).toBe(organized);
  });

  it("lands a reply unless a layout event or a resync came after the request began", () => {
    const state = loaded();
    const since = rowClock(state);
    const moved = withRoomMoved(organized, 3, 9, 1);

    const landed = landWorkspaceChange(state, () => moved, since);

    expect(landed.sidebar.workspaceLayout).toBe(moved);
    // A sidebar snapshot read before the reply landed doesn't put the old layout back.
    expect(
      loadSidebar(landed, { ...sidebarFixture([]), workspaceLayout: organized }, since).sidebar
        .workspaceLayout,
    ).toBe(moved);

    const live = applyEvents(
      state,
      [{ seq: 1, topic: "user:7", type: "workspace.layout.updated", data: moved }],
      0,
    );

    expect(landWorkspaceChange(live, () => organized, since).sidebar.workspaceLayout).toBe(moved);
    expect(
      landWorkspaceChange(markResynced(state), () => moved, since).sidebar.workspaceLayout,
    ).toBe(organized);
  });
});
