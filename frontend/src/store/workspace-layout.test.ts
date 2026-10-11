import { describe, expect, it } from "vitest";

import { decodeSidebar, sidebarFixture, sidebarRowFixture } from "../api/testing.ts";
import { applyEvents, loadSidebar, setRoomUnavailable } from "./reducers.ts";
import { rowClock } from "./row-touches.ts";
import { initialState } from "./state.ts";

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
