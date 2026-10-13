import { describe, expect, it } from "vitest";
import { meFixture, sidebarFixture, sidebarRowFixture, userFixture } from "../../api/testing.ts";
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

const self = sidebarRowFixture(SELF_ROOM, "Ada", "direct", [VIEWER]);

const grace = {
  ...sidebarRowFixture(41, "Grace", "direct", [3]),
  room: {
    ...sidebarRowFixture(41, "Grace", "direct", [3]).room,
    updatedAt: "2026-10-12T00:00:00Z",
  },
};

function signedIn(rows: Parameters<typeof sidebarFixture>[0], placeholders: number[]): State {
  return loadSidebar(setMe(initialState, meFixture), sidebarFixture(rows, placeholders), 0);
}

describe("the note-to-self", () => {
  it("is the direct room whose only member is you", () => {
    expect(isNoteToSelf(self, VIEWER)).toBe(true);
    expect(isNoteToSelf(grace, VIEWER)).toBe(false);
    expect(isNoteToSelf(self, null)).toBe(false);
    expect(isNoteToSelf(sidebarRowFixture(42, "general"), VIEWER)).toBe(false);
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
    const before = signedIn([grace], [VIEWER, 3]);

    expect(offersNoteToSelf(before)).toBe(true);
    expect(noteToSelfRoomId(before)).toBeNull();
    expect(viewerNameOf(before)).toBe("Ada Lovelace");

    const after = signedIn([grace, self], [3]);

    expect(offersNoteToSelf(after)).toBe(false);
    expect(noteToSelfRoomId(after)).toBe(SELF_ROOM);
    expect(roomIsNoteToSelf(after, SELF_ROOM)).toBe(true);
    expect(roomIsNoteToSelf(after, 41)).toBe(false);
  });

  it("is offered again once its row leaves the sidebar", () => {
    const state = applyEvents(
      signedIn([grace, self], [3]),
      [{ seq: 1, topic: "user", type: "sidebar.row.removed", data: { roomId: SELF_ROOM } }],
      0,
    );

    expect(state.sidebar.placeholderUserIds).toEqual([VIEWER, 3]);
    expect(offersNoteToSelf(state)).toBe(true);
  });

  it("leads Direct messages, ahead of newer conversations", () => {
    const state = signedIn([grace, self], [3]);

    const directs = sidebarSections(state.sidebar, { viewerId: VIEWER }).find(
      (section) => section.kind === "direct",
    );

    expect(directs?.rows.map((row) => row.room.id)).toEqual([SELF_ROOM, 41]);
  });

  it("is you in the switcher, before and after it exists", () => {
    const before = localItems(signedIn([grace], [VIEWER, 3]).sidebar, VIEWER);
    const offered = before.find((item) => item.key === `person:${VIEWER}`);

    expect(offered).toMatchObject({ kind: "person", userId: VIEWER, roomId: null });

    const after = localItems(signedIn([grace, self], [3]).sidebar, VIEWER);

    expect(after.filter((item) => item.key === `person:${VIEWER}`)).toEqual([
      expect.objectContaining({ roomId: SELF_ROOM }),
    ]);
  });
});
