import { describe, expect, it } from "vitest";
import { meFixture, sidebarFixture, sidebarRowFixture, userFixture } from "../../api/testing.ts";
import type { Sidebar } from "../../gen/Sidebar.ts";
import type { SidebarRow } from "../../store/model.ts";
import { applyEvents, loadSidebar, setMe } from "../../store/reducers.ts";
import { initialState, type State } from "../../store/state.ts";
import { sidebarSections } from "../sidebar/sections.ts";
import { localItems } from "../switcher/ranking.ts";
import {
  isNoteToSelf,
  namesWithYou,
  noteToSelfRoomId,
  offersNoteToSelf,
  roomIsNoteToSelf,
  viewerNameOf,
  withYou,
} from "./note-to-self.ts";

const VIEWER = meFixture.user.id;

const SELF_ROOM = 40;

const TRIP = 43;

/** The note-to-self, as the server marks it. */
const self: SidebarRow = {
  ...sidebarRowFixture(SELF_ROOM, "Ada", "direct", [VIEWER]),
  noteToSelf: true,
};

const graceRow = sidebarRowFixture(41, "Grace", "direct", [3]);

const grace: SidebarRow = {
  ...graceRow,
  room: { ...graceRow.room, updatedAt: "2026-10-12T00:00:00Z" },
};

/** A named group everyone else left: only the viewer is in it, but it is not their notes. */
const tripRow = sidebarRowFixture(TRIP, "Road trip", "direct", [VIEWER]);

const trip: SidebarRow = {
  ...tripRow,
  room: { ...tripRow.room, name: "Road trip", updatedAt: "2026-10-13T00:00:00Z" },
  displayName: "Road trip",
};

function signedIn(
  rows: readonly SidebarRow[],
  placeholders: number[],
  noteToSelf: number | null = null,
): State {
  const sidebar: Sidebar = sidebarFixture(rows, placeholders);

  if (noteToSelf !== null) {
    sidebar.noteToSelfRoomId = noteToSelf;
  }

  return loadSidebar(setMe(initialState, meFixture), sidebar, 0);
}

const removed = (roomId: number) =>
  [{ seq: 1, topic: "user", type: "sidebar.row.removed", data: { roomId } }] as const;

describe("the note-to-self", () => {
  it("is the room the server marks, never one worked out from its members", () => {
    expect(isNoteToSelf(self)).toBe(true);
    expect(isNoteToSelf(grace)).toBe(false);
    expect(isNoteToSelf(trip)).toBe(false);
    expect(isNoteToSelf(sidebarRowFixture(42, "general"))).toBe(false);
  });

  it("names you with (you), and nobody else", () => {
    const users = { [VIEWER]: userFixture(VIEWER, "Ada Lovelace"), 3: userFixture(3, "Grace") };
    const name = namesWithYou(users, VIEWER);

    expect(withYou("Ada Lovelace")).toBe("Ada Lovelace (you)");
    expect(name(VIEWER)).toBe("Ada Lovelace (you)");
    expect(name(3)).toBe("Grace");
    expect(name(99)).toBeUndefined();
  });

  it("is offered while the server leads its placeholders with you and there is no row", () => {
    const before = signedIn([grace, trip], [VIEWER, 3]);

    expect(offersNoteToSelf(before)).toBe(true);
    expect(noteToSelfRoomId(before)).toBeNull();
    expect(roomIsNoteToSelf(before, TRIP)).toBe(false);
    expect(viewerNameOf(before)).toBe("Ada Lovelace");

    const after = signedIn([grace, trip, self], [3], SELF_ROOM);

    expect(offersNoteToSelf(after)).toBe(false);
    expect(noteToSelfRoomId(after)).toBe(SELF_ROOM);
    expect(roomIsNoteToSelf(after, SELF_ROOM)).toBe(true);
    expect(roomIsNoteToSelf(after, TRIP)).toBe(false);
  });

  it("learns its room from the row that opens it", () => {
    const state = applyEvents(
      signedIn([grace], [VIEWER, 3]),
      [{ seq: 1, topic: "user", type: "sidebar.row.upserted", data: self }],
      0,
    );

    expect(noteToSelfRoomId(state)).toBe(SELF_ROOM);
    expect(offersNoteToSelf(state)).toBe(false);
  });

  it("is offered again once hidden, and still labelled yours everywhere", () => {
    const state = applyEvents(signedIn([grace, self], [3], SELF_ROOM), removed(SELF_ROOM), 0);

    expect(state.sidebar.placeholderUserIds).toEqual([VIEWER, 3]);
    expect(offersNoteToSelf(state)).toBe(true);
    // Saved, scheduled, search and forward labels go by this, so they keep "(you)".
    expect(noteToSelfRoomId(state)).toBe(SELF_ROOM);
    expect(roomIsNoteToSelf(state, SELF_ROOM)).toBe(true);
  });

  it("leads Direct messages, ahead of newer conversations and an emptied group", () => {
    const state = signedIn([grace, trip, self], [3], SELF_ROOM);
    const directs = sidebarSections(state.sidebar).find((section) => section.kind === "direct");

    expect(directs?.rows.map((row) => row.room.id)).toEqual([SELF_ROOM, TRIP, 41]);
  });

  it("is you in the switcher, before and after it exists; an emptied group stays a room", () => {
    const before = localItems(signedIn([grace, trip], [VIEWER, 3]).sidebar, VIEWER);

    expect(before.filter((item) => item.key === `person:${VIEWER}`)).toEqual([
      expect.objectContaining({ kind: "person", userId: VIEWER, roomId: null }),
    ]);
    expect(before.find((item) => item.roomId === TRIP)).toMatchObject({
      key: `room:${TRIP}`,
      kind: "room",
      label: "Road trip",
    });

    const after = localItems(signedIn([grace, trip, self], [3], SELF_ROOM).sidebar, VIEWER);

    expect(after.filter((item) => item.key === `person:${VIEWER}`)).toEqual([
      expect.objectContaining({ roomId: SELF_ROOM }),
    ]);
    expect(after.find((item) => item.roomId === TRIP)?.kind).toBe("room");
  });
});
