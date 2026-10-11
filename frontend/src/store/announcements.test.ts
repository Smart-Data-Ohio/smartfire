import { afterEach, describe, expect, it } from "vitest";
import { roomDetailFixture, sidebarFixture, sidebarRowFixture } from "../api/testing.ts";
import { isAnnouncementRoom } from "./announcements.ts";
import { beginRoomRequest } from "./join-state.ts";
import { mutations, sidebarRowClock, store } from "./store.ts";

const NEWS = 5;

const AT = "2026-10-11T10:00:00.000Z";

afterEach(() => mutations.reset());

describe("the stored announcement flag", () => {
  it("follows the sidebar row when the detail ties it to the millisecond", () => {
    const row = sidebarRowFixture(NEWS, "news");
    const detail = roomDetailFixture(NEWS);

    // A settings save's older detail lands after sync's row, stamped the same millisecond.
    mutations.loadSidebar(
      sidebarFixture([{ ...row, room: { ...row.room, announcement: true, updatedAt: AT } }]),
      sidebarRowClock(),
    );
    mutations.setRoomDetail(
      { ...detail, room: { ...detail.room, announcement: false, updatedAt: AT } },
      beginRoomRequest(),
    );

    expect(isAnnouncementRoom(store.getState(), NEWS)).toBe(true);
  });

  it("reads the detail for a room with no sidebar row", () => {
    const detail = roomDetailFixture(NEWS);

    mutations.setRoomDetail(
      { ...detail, room: { ...detail.room, announcement: true } },
      beginRoomRequest(),
    );

    expect(store.getState().sidebar.rows[NEWS]).toBeUndefined();
    expect(isAnnouncementRoom(store.getState(), NEWS)).toBe(true);
  });

  it("keeps the row's flag once the row is removed, even if the room's refresh then fails", () => {
    const row = sidebarRowFixture(NEWS, "news");
    const detail = roomDetailFixture(NEWS);

    mutations.loadSidebar(
      sidebarFixture([{ ...row, room: { ...row.room, announcement: false, updatedAt: AT } }]),
      sidebarRowClock(),
    );
    mutations.setRoomDetail(
      { ...detail, room: { ...detail.room, announcement: true, updatedAt: AT } },
      beginRoomRequest(),
    );
    expect(isAnnouncementRoom(store.getState(), NEWS)).toBe(false);

    mutations.applyEvents(
      [
        {
          seq: 0,
          topic: "user:7",
          type: "sidebar.row.removed",
          data: { roomId: NEWS, refreshRoom: true },
        },
      ],
      Date.now(),
    );
    expect(store.getState().sidebar.rows[NEWS]).toBeUndefined();
    expect(isAnnouncementRoom(store.getState(), NEWS)).toBe(false);

    // The refresh that follows fails; the detail it leaves still says what the row said.
    const started = beginRoomRequest();

    mutations.setRoomError(NEWS, "The server didn't answer", started);
    expect(store.getState().rooms[NEWS]?.status).toBe("error");
    expect(isAnnouncementRoom(store.getState(), NEWS)).toBe(false);
  });

  it("keeps the row's flag when a sidebar snapshot drops the row", () => {
    const row = sidebarRowFixture(NEWS, "news");
    const detail = roomDetailFixture(NEWS);

    mutations.loadSidebar(
      sidebarFixture([{ ...row, room: { ...row.room, announcement: false, updatedAt: AT } }]),
      sidebarRowClock(),
    );
    mutations.setRoomDetail(
      { ...detail, room: { ...detail.room, announcement: true, updatedAt: AT } },
      beginRoomRequest(),
    );
    mutations.resyncSidebar(sidebarFixture([]), sidebarRowClock());

    expect(store.getState().sidebar.rows[NEWS]).toBeUndefined();
    expect(isAnnouncementRoom(store.getState(), NEWS)).toBe(false);
  });

  describe("a detail newer than the row keeps its own flag when the row leaves", () => {
    const ROW_AT = "2026-10-11T10:00:00.000Z";
    const DETAIL_AT = "2026-10-11T10:01:00.000Z";

    const seed = () => {
      const row = sidebarRowFixture(NEWS, "news");
      const detail = roomDetailFixture(NEWS);

      mutations.loadSidebar(
        sidebarFixture([{ ...row, room: { ...row.room, announcement: true, updatedAt: ROW_AT } }]),
        sidebarRowClock(),
      );
      mutations.setRoomDetail(
        { ...detail, room: { ...detail.room, announcement: false, updatedAt: DETAIL_AT } },
        beginRoomRequest(),
      );
    };

    const expectDetailKept = () => {
      expect(store.getState().sidebar.rows[NEWS]).toBeUndefined();
      expect(store.getState().rooms[NEWS]?.detail?.room).toMatchObject({
        announcement: false,
        updatedAt: DETAIL_AT,
      });
      expect(isAnnouncementRoom(store.getState(), NEWS)).toBe(false);
    };

    it("on sidebar.row.removed", () => {
      seed();
      mutations.applyEvents(
        [
          {
            seq: 0,
            topic: "user:7",
            type: "sidebar.row.removed",
            data: { roomId: NEWS, refreshRoom: true },
          },
        ],
        Date.now(),
      );
      expectDetailKept();
    });

    it("when a loaded sidebar drops the row", () => {
      seed();
      mutations.loadSidebar(sidebarFixture([]), sidebarRowClock());
      expectDetailKept();
    });

    it("when a resync drops the row", () => {
      seed();
      mutations.resyncSidebar(sidebarFixture([]), sidebarRowClock());
      expectDetailKept();
    });
  });
});
