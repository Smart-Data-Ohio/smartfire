import { beforeEach, describe, expect, it } from "vitest";
import { sidebarFixture, sidebarRowFixture } from "../api/testing.ts";
import { beginRoomRequest } from "./join-state.ts";
import type { SidebarRow, SyncEvent } from "./model.ts";
import {
  landCategories,
  landCategory,
  landCategoryRemoval,
  mergeOrganization,
  setMembership,
} from "./organize.ts";
import {
  applyEvents,
  loadSidebar,
  markRoomRead,
  markUnreadFrom,
  resyncSidebar,
} from "./reducers.ts";
import {
  pruneTouches,
  rowClock,
  touchedSince,
  touchRows,
  untouchedReplyEvents,
} from "./row-touches.ts";
import { emptyTimeline, initialState, type State } from "./state.ts";
import { mutations, store } from "./store.ts";

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

  it("a room sync added and removed in one batch stays gone under an older snapshot", () => {
    const before = loaded([general]);
    const since = rowClock(before);
    const synced = applyEvents(before, [upserted(design), removed(2)], 0);
    const replied = loadSidebar(synced, sidebarFixture([general, design]), since);

    expect(replied.sidebar.rows[2]).toBeUndefined();
    expect(replied.sidebar.order).toEqual([1]);
  });

  it("a removal of a row already gone still outranks an older snapshot listing it", () => {
    const before = loaded([general]);
    const since = rowClock(before);
    const synced = applyEvents(before, [removed(2)], 0);
    const replied = loadSidebar(synced, sidebarFixture([general, design]), since);

    expect(replied.sidebar.rows[2]).toBeUndefined();
  });

  it("a gap's resync outranks a refetch that started before it", () => {
    // A refetch goes out; the sync engine's resync lands the read row; the refetch arrives.
    const before = loaded([pinged(general), design]);
    const since = rowClock(before);
    const resynced = resyncSidebar(before, sidebarFixture([read(general)]), rowClock(before));
    const replied = loadSidebar(resynced, sidebarFixture([pinged(general), design]), since);

    expect(replied.sidebar.rows[1]?.notificationCount).toBe(0);
    expect(replied.sidebar.rows[2]).toBeUndefined();
  });

  it("an unseen room created and deleted during a gap stays gone under an older reply", () => {
    // A refetch goes out and its snapshot lists room 9; the socket misses room 9's creation and
    // deletion; the resync (without it) lands; then the refetch, and a join's row, arrive.
    const nine = sidebarRowFixture(9, "pop-up");
    const before = loaded([general]);
    const since = rowClock(before);
    const resynced = resyncSidebar(before, sidebarFixture([general]), rowClock(before));

    expect(loadSidebar(resynced, sidebarFixture([general, nine]), since)).toBe(resynced);
    expect(untouchedReplyEvents(resynced, [upserted(nine)], since)).toEqual([]);
  });

  it("a reply that started before a resync is stale as a whole", () => {
    const before = loaded([pinged(general)]);
    const since = rowClock(before);
    const resynced = resyncSidebar(before, sidebarFixture([read(general)]), rowClock(before));
    const membership = { ...pinged(general).membership, involvement: "mentions" as const };

    expect(setMembership(resynced, membership, since)).toBe(resynced);
    expect(mergeOrganization(resynced, [{ ...general, membership }], since)).toBe(resynced);
    expect(markUnreadFrom(resynced, 1, 40, 0, since).sidebar).toBe(resynced.sidebar);
    // A request that began after the resync lands as usual.
    expect(
      setMembership(resynced, membership, rowClock(resynced)).sidebar.rows[1]?.membership
        .involvement,
    ).toBe("mentions");
  });

  it("an old mark-unread reply after a resync leaves the divider be", () => {
    const withTimeline = (state: State, unreadFromId: number | null): State => ({
      ...state,
      timelines: {
        ...state.timelines,
        1: { ...emptyTimeline, ids: [40, 41, 42], unreadFromId, unreadCount: 0 },
      },
    });

    // The old mark-unread (from 40) goes out; a resync lands; a newer one marks from 42.
    const before = withTimeline(loaded([general]), null);
    const old = rowClock(before);
    const resynced = resyncSidebar(before, sidebarFixture([general]), rowClock(before));
    const newer = markUnreadFrom(resynced, 1, 42, 0, rowClock(resynced));

    expect(newer.timelines[1]?.unreadFromId).toBe(42);
    expect(markUnreadFrom(newer, 1, 40, 0, old)).toBe(newer);
  });

  it("an old mark-unread reply after a newer one leaves the divider be", () => {
    const state: State = {
      ...loaded([general]),
      timelines: { 1: { ...emptyTimeline, ids: [40, 41, 42], unreadFromId: null, unreadCount: 0 } },
    };

    const old = rowClock(state);
    const newer = markUnreadFrom(state, 1, 42, 0, rowClock(state));

    expect(markUnreadFrom(newer, 1, 40, 0, old)).toBe(newer);
  });

  it("the viewer's own room.unread landing first still lets the divider move", () => {
    const state: State = {
      ...loaded([general]),
      timelines: { 1: { ...emptyTimeline, ids: [40, 41, 42], unreadFromId: null, unreadCount: 0 } },
    };

    const since = rowClock(state);

    const own: SyncEvent = {
      seq: 1,
      topic: "user:7",
      type: "room.unread",
      data: { roomId: 1, messageId: null, mentioned: false },
    };

    const replied = markUnreadFrom(applyEvents(state, [own], 0), 1, 41, 0, since);

    expect(replied.timelines[1]?.unreadFromId).toBe(41);
  });

  it("a mark-unread reply from before another tab read the room changes nothing", () => {
    // Here: mark unread from 41 goes out. Another tab reads the room; its room.read lands first.
    const state: State = {
      ...loaded([general]),
      timelines: { 1: { ...emptyTimeline, ids: [40, 41, 42], unreadFromId: null, unreadCount: 0 } },
    };

    const since = rowClock(state);

    const readElsewhere: SyncEvent = {
      seq: 1,
      topic: "user:7",
      type: "room.read",
      data: { roomId: 1 },
    };

    const synced = applyEvents(state, [readElsewhere], 0);
    const replied = markUnreadFrom(synced, 1, 41, 0, since);

    expect(replied).toBe(synced);
    expect(replied.timelines[1]?.unreadFromId).toBeNull();
    expect(replied.sidebar.rows[1]?.unreadCount).toBe(0);

    // A mark-unread sent after that read lands as usual.
    expect(markUnreadFrom(synced, 1, 41, 0, rowClock(synced)).timelines[1]?.unreadFromId).toBe(41);
  });

  it("a category reply older than sync's word on it changes nothing", () => {
    const team = { id: 2, name: "Team", collapsed: false, position: 1 };

    const before: State = {
      ...loaded([general]),
      sidebar: { ...loaded([general]).sidebar, categories: [team] },
    };

    const since = rowClock(before);

    const gone = applyEvents(
      before,
      [{ seq: 1, topic: "user:7", type: "sidebar.category.removed", data: { id: 2 } }],
      0,
    );

    expect(landCategory(gone, { ...team, name: "People" }, since)).toBe(gone);
    expect(landCategories(gone, [team], since).sidebar.categories).toEqual([]);

    const resynced = resyncSidebar(before, sidebarFixture([general]), rowClock(before));

    expect(landCategory(resynced, { ...team, name: "People" }, since)).toBe(resynced);
    expect(landCategories(resynced, [team], since)).toBe(resynced);
    expect(landCategoryRemoval(before, 2, since).sidebar.categories).toEqual([]);
  });

  it("forgets the touches no request in flight is older than", () => {
    const touched = touchRows(touchRows(loaded([general, design]), [1]), [2]);

    expect(Object.keys(pruneTouches(touched, 1).rowTouches.at)).toEqual(["2"]);
    expect(pruneTouches(touched, undefined).rowTouches.at).toEqual({});
    expect(pruneTouches(touched, 0)).toBe(touched);
  });

  it("a gap's resync still keeps rows sync changed while it was on its way", () => {
    const before = loaded([pinged(general)]);
    const since = rowClock(before);
    const synced = applyEvents(before, [upserted(read(general))], 0);
    const resynced = resyncSidebar(synced, sidebarFixture([pinged(general)]), since);

    expect(resynced.sidebar.rows[1]?.notificationCount).toBe(0);
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

describe("sidebar row tickets in the store", () => {
  beforeEach(() => mutations.reset());

  const rows = () => store.getState().sidebar.rows;

  it("a 404 from before access came back doesn't block the restored row", () => {
    // Room 2 was lost. Its 404 read goes out; access comes back during a gap; a resync reads.
    mutations.loadSidebar(sidebarFixture([general]), rowClock(store.getState()));

    const notFound = mutations.openRowTicket();
    const resync = mutations.openRowTicket();

    // The 404 lands while the resync is on its way: the store already agrees the room is gone.
    expect(mutations.setRoomUnavailable(2, beginRoomRequest(), notFound)).toBe(true);
    mutations.resyncSidebar(sidebarFixture([general, design]), resync);
    mutations.closeRowTicket(resync);

    expect(rows()[2]).toEqual(design);

    // Read again, it lands after the resync: stale, so the restored row stays.
    expect(mutations.setRoomUnavailable(2, beginRoomRequest(), notFound)).toBe(false);
    mutations.closeRowTicket(notFound);

    expect(rows()[2]).toEqual(design);
  });

  it("a 404 from before sync restored the row leaves it", () => {
    mutations.loadSidebar(sidebarFixture([general]), rowClock(store.getState()));

    const notFound = mutations.openRowTicket();

    mutations.applyEvents([upserted(design)], 0);

    expect(mutations.setRoomUnavailable(2, beginRoomRequest(), notFound)).toBe(false);
    expect(rows()[2]).toEqual(design);
    mutations.closeRowTicket(notFound);
  });

  it("keeps no touches once every request has settled", () => {
    mutations.loadSidebar(sidebarFixture([general]), rowClock(store.getState()));
    mutations.applyEvents([upserted(read(general))], 0);

    // Nothing in flight: nothing to outrank, so nothing is kept.
    expect(store.getState().rowTouches.at).toEqual({});

    const first = mutations.openRowTicket();

    mutations.applyEvents([upserted(pinged(general))], 0);

    const second = mutations.openRowTicket();

    mutations.applyEvents([upserted(design)], 0);
    mutations.applyEvents([{ seq: 1, topic: "user:7", type: "room.read", data: { roomId: 2 } }], 0);

    expect(Object.keys(store.getState().rowTouches.at).sort()).toEqual(["1", "2"]);
    expect(Object.keys(store.getState().rowTouches.reads)).toEqual(["2"]);

    // Only the second request is older than room 2's touch now.
    mutations.closeRowTicket(first);

    expect(Object.keys(store.getState().rowTouches.at)).toEqual(["2"]);

    mutations.closeRowTicket(second);

    expect(store.getState().rowTouches.at).toEqual({});
    expect(store.getState().rowTouches.reads).toEqual({});
  });

  it("stays bounded while rooms come and go around short requests", () => {
    mutations.loadSidebar(sidebarFixture([general]), rowClock(store.getState()));

    for (let roomId = 100; roomId < 600; roomId++) {
      const since = mutations.openRowTicket();

      mutations.applyEvents([upserted(sidebarRowFixture(roomId, `pop-up ${roomId}`))], 0);
      mutations.applyEvents([removed(roomId)], 0);

      expect(Object.keys(store.getState().rowTouches.at).length).toBeLessThanOrEqual(1);
      mutations.closeRowTicket(since);
    }

    expect(store.getState().rowTouches.at).toEqual({});
  });
});
