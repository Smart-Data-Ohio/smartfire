import { describe, expect, it } from "vitest";
import { roomDetailFixture, sidebarFixture, sidebarRowFixture } from "../api/testing.ts";
import type { SidebarRow, SyncEvent } from "./model.ts";
import { applyEvents, loadSidebar, setRoomDetail } from "./reducers.ts";
import { initialState } from "./state.ts";

const rowEvent = (data: SidebarRow): SyncEvent => ({
  seq: 1,
  topic: "user:7",
  type: "sidebar.row.upserted",
  data,
});

describe("room metadata follows sidebar sync", () => {
  it("updates a loaded room's type/name/icon and membership, preserving detail-only facts", () => {
    const detail = roomDetailFixture(20, { firstUnreadMessageId: 41, count: 2 });
    const row = sidebarRowFixture(20, "Launch crew", "closed");

    const edited = {
      ...row,
      room: { ...row.room, iconName: "fire" },
      membership: { ...row.membership, involvement: "mentions" as const },
    };

    const loaded = setRoomDetail(loadSidebar(initialState, sidebarFixture([row])), detail);
    const next = applyEvents(loaded, [rowEvent(edited)], 0);

    expect(next.rooms[20]?.detail).toEqual({
      ...detail,
      room: edited.room,
      membership: edited.membership,
      displayName: edited.displayName,
      directMemberIds: edited.directMemberIds,
    });
    expect(next.timelines[20]).toBe(loaded.timelines[20]);
  });

  it("updates direct display names and avatar ids, including a fresh sidebar snapshot", () => {
    const row = sidebarRowFixture(20, "Ada and Grace", "direct", [8, 9]);
    const loaded = setRoomDetail(initialState, roomDetailFixture(20));
    const synced = applyEvents(loaded, [rowEvent(row)], 0);

    expect(synced.rooms[20]?.detail?.directMemberIds).toEqual([8, 9]);
    expect(synced.rooms[20]?.detail?.displayName).toBe("Ada and Grace");
    expect(loadSidebar(loaded, sidebarFixture([row])).rooms[20]?.detail?.displayName).toBe(
      "Ada and Grace",
    );
  });

  it("does not create an unloaded detail or revoke access for a hidden row removal", () => {
    const row = sidebarRowFixture(20, "Room");

    expect(applyEvents(initialState, [rowEvent(row)], 0).rooms[20]).toBeUndefined();

    const loaded = setRoomDetail(
      loadSidebar(initialState, sidebarFixture([row])),
      roomDetailFixture(20),
    );

    const removed = applyEvents(
      loaded,
      [{ seq: 1, topic: "user:7", type: "sidebar.row.removed", data: { roomId: 20 } }],
      0,
    );

    expect(removed.sidebar.rows[20]).toBeUndefined();
    expect(removed.rooms[20]).toBe(loaded.rooms[20]);
    expect(removed.timelines[20]).toBe(loaded.timelines[20]);
  });
});
