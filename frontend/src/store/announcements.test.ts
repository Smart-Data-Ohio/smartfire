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
});
