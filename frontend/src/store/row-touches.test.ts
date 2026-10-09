import { describe, expect, it } from "vitest";
import { sidebarFixture, sidebarRowFixture } from "../api/testing.ts";
import type { SidebarRow, SyncEvent } from "./model.ts";
import { setMembership } from "./organize.ts";
import { applyEvents, loadSidebar, markRoomRead, markUnreadFrom } from "./reducers.ts";
import { rowClock, touchedSince, touchRows, untouchedReplyEvents } from "./row-touches.ts";
import { initialState, type State } from "./state.ts";

const general = sidebarRowFixture(1, "general");

const design = sidebarRowFixture(2, "design");

const pinged = (row: SidebarRow): SidebarRow => ({
  ...row,
  unreadCount: 3,
  notificationCount: 1,
  membership: { ...row.membership, unreadAt: "2026-10-09T00:00:00.000Z" },
});

const read = (row: SidebarRow): SidebarRow => ({
  ...row,
  unreadCount: 0,
  notificationCount: 0,
  membership: { ...row.membership, unreadAt: null },
});

const upserted = (row: SidebarRow): SyncEvent => ({
  seq: 1,
  topic: "user:7",
  type: "sidebar.row.upserted",
  data: row,
});

const removed = (roomId: number): SyncEvent => ({
  seq: 1,
  topic: "user:7",
  type: "sidebar.row.removed",
  data: { roomId },
});

const loaded = (rows: readonly SidebarRow[]): State =>
  loadSidebar(initialState, sidebarFixture(rows), rowClock(initialState));

describe("sidebar row touches", () => {
  it("a whole-sidebar snapshot from before a newer synced row leaves that row be", () => {
    // The snapshot request goes out; a read's row lands over sync; the stale reply arrives.
    const before = loaded([pinged(general), design]);
    const since = rowClock(before);
    const synced = applyEvents(before, [upserted(read(general))], 0);
    const replied = loadSidebar(synced, sidebarFixture([pinged(general), design]), since);

    expect(replied.sidebar.rows[1]?.notificationCount).toBe(0);
    expect(replied.sidebar.rows[1]?.unreadCount).toBe(0);
    expect(replied.sidebar.rows[1]?.membership.unreadAt).toBeNull();
  });

  it("a snapshot still lands every row sync left alone", () => {
    const before = loaded([general, design]);
    const since = rowClock(before);
    const synced = applyEvents(before, [upserted(read(general))], 0);
    const replied = loadSidebar(synced, sidebarFixture([general, pinged(design)]), since);

    expect(replied.sidebar.rows[2]?.notificationCount).toBe(1);
  });

  it("a snapshot from before sync removed or added a room keeps sync's word", () => {
    const before = loaded([general, design]);
    const since = rowClock(before);
    const third = sidebarRowFixture(3, "announcements");
    const synced = applyEvents(before, [removed(2), upserted(third)], 0);
    const replied = loadSidebar(synced, sidebarFixture([general, design]), since);

    expect(replied.sidebar.rows[2]).toBeUndefined();
    expect(replied.sidebar.rows[3]).toBe(third);
    expect(replied.sidebar.order).toEqual([3, 1]);
  });

  it("a snapshot taken after the synced row lands in full", () => {
    const synced = applyEvents(loaded([general]), [upserted(read(general))], 0);
    const replied = loadSidebar(synced, sidebarFixture([pinged(general)]), rowClock(synced));

    expect(replied.sidebar.rows[1]?.notificationCount).toBe(1);
  });

  it("a read made here outranks a snapshot already on its way", () => {
    const before = loaded([pinged(general)]);
    const since = rowClock(before);
    const readHere = touchRows(markRoomRead(before, 1), [1]);
    const replied = loadSidebar(readHere, sidebarFixture([pinged(general)]), since);

    expect(replied.sidebar.rows[1]?.unreadCount).toBe(0);
  });

  it("a reply's rows put through the sync reducers touch nothing", () => {
    const before = loaded([general]);
    const after = applyEvents(before, [upserted(pinged(general))], 0, "reply");

    expect(rowClock(after)).toBe(rowClock(before));
    expect(touchedSince(after, 1, 0)).toBe(false);
  });

  it("a join or room reply from before sync removed the room doesn't bring it back", () => {
    const before = loaded([general, design]);
    const since = rowClock(before);
    const synced = applyEvents(before, [removed(2)], 0);

    expect(untouchedReplyEvents(synced, [upserted(design), upserted(general)], since)).toEqual([
      upserted(general),
    ]);
  });

  it("an involvement reply older than a synced row takes only the involvement", () => {
    const before = loaded([pinged(general)]);
    const since = rowClock(before);
    const synced = applyEvents(before, [upserted(read(general))], 0);

    const replied = setMembership(
      synced,
      { ...pinged(general).membership, involvement: "mentions" },
      since,
    );

    expect(replied.sidebar.rows[1]?.membership.involvement).toBe("mentions");
    expect(replied.sidebar.rows[1]?.membership.unreadAt).toBeNull();
  });

  it("a mark-unread reply older than a synced row leaves the row", () => {
    const before = loaded([general]);
    const since = rowClock(before);
    const synced = applyEvents(before, [upserted(read(general))], 0);
    const replied = markUnreadFrom(synced, 1, 40, 0, since);

    expect(replied.sidebar.rows[1]).toBe(synced.sidebar.rows[1]);
  });
});
