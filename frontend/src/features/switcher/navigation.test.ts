import { describe, expect, it } from "vitest";
import { sidebarRowFixture } from "../../api/testing.ts";
import { SHORTCUTS } from "../../lib/shortcuts.ts";
import type { SidebarRow } from "../../store/model.ts";
import { initialState, type SidebarState } from "../../store/state.ts";
import { globalShortcut, type KeyPress } from "./global-keys.ts";
import { adjacentRoom, isUnreadStop, sidebarOrder } from "./room-navigation.ts";
import { groupShortcuts } from "./shortcut-groups.ts";

function withUnread(row: SidebarRow, muted = false): SidebarRow {
  return {
    ...row,
    unreadCount: 1,
    membership: {
      ...row.membership,
      unreadAt: "2026-10-05T00:00:00.000Z",
      involvement: muted ? "muted" : row.membership.involvement,
    },
  };
}

function favorite(row: SidebarRow): SidebarRow {
  return { ...row, membership: { ...row.membership, favoritePosition: 1 } };
}

function sidebarOf(rows: readonly SidebarRow[]): SidebarState {
  return {
    ...initialState.sidebar,
    status: "ready",
    order: rows.map((row) => row.room.id),
    rows: Object.fromEntries(rows.map((row) => [row.room.id, row])),
  };
}

// Server order is by name; the sidebar shows Favourites first, then Channels, then DMs.
const SIDEBAR = sidebarOf([
  sidebarRowFixture(1, "alpha"),
  withUnread(sidebarRowFixture(2, "beta")),
  favorite(sidebarRowFixture(3, "gamma")),
  withUnread(sidebarRowFixture(4, "noise"), true),
  withUnread(sidebarRowFixture(9, "Maya", "direct", [2])),
]);

const ROWS = sidebarOrder(SIDEBAR);

describe("sidebarOrder", () => {
  it("follows the sidebar's sections, not the server's order", () => {
    expect(ROWS.map((row) => row.room.id)).toEqual([3, 1, 2, 4, 9]);
  });

  it("can keep only some sections (the DMs destination)", () => {
    expect(
      sidebarOrder(SIDEBAR, (section) => section.key === "direct").map((row) => row.room.id),
    ).toEqual([9]);
  });
});

describe("adjacentRoom", () => {
  it("steps down and up, wrapping at the ends", () => {
    expect(adjacentRoom(ROWS, 1, 1)).toBe(2);
    expect(adjacentRoom(ROWS, 1, -1)).toBe(3);
    expect(adjacentRoom(ROWS, 9, 1)).toBe(3);
    expect(adjacentRoom(ROWS, 3, -1)).toBe(9);
  });

  it("starts at the top going down and the bottom going up from no room", () => {
    expect(adjacentRoom(ROWS, null, 1)).toBe(3);
    expect(adjacentRoom(ROWS, null, -1)).toBe(9);
  });

  it("stops only at unread rooms that aren't muted", () => {
    expect(adjacentRoom(ROWS, 3, 1, isUnreadStop)).toBe(2);
    expect(adjacentRoom(ROWS, 2, 1, isUnreadStop)).toBe(9);
    expect(adjacentRoom(ROWS, 9, 1, isUnreadStop)).toBe(2);
    expect(adjacentRoom(ROWS, 2, -1, isUnreadStop)).toBe(9);
  });

  it("answers null when no other room qualifies", () => {
    expect(adjacentRoom([], null, 1)).toBeNull();
    expect(adjacentRoom(ROWS.slice(0, 1), 3, 1)).toBeNull();
    expect(adjacentRoom(ROWS, 2, 1, (row) => row.room.id === 2)).toBeNull();
  });
});

function press(key: string, change: Partial<KeyPress> = {}): KeyPress {
  return {
    key,
    code: "",
    metaKey: false,
    ctrlKey: false,
    altKey: false,
    shiftKey: false,
    ...change,
  };
}

describe("globalShortcut", () => {
  it("reads ⌘ on Apple platforms and Ctrl elsewhere", () => {
    expect(globalShortcut(press("k", { metaKey: true }), true)).toBe("switcher");
    expect(globalShortcut(press("k", { ctrlKey: true }), false)).toBe("switcher");
    expect(globalShortcut(press("k", { ctrlKey: true }), true)).toBeNull();
    expect(globalShortcut(press("k", { metaKey: true }), false)).toBeNull();
  });

  it("maps ⌘⇧K, ⌘/ and a non-Latin layout's K key", () => {
    expect(globalShortcut(press("K", { ctrlKey: true, shiftKey: true }), false)).toBe("new-direct");
    expect(globalShortcut(press("/", { ctrlKey: true }), false)).toBe("shortcuts");
    expect(globalShortcut(press("л", { ctrlKey: true, code: "KeyK" }), false)).toBe("switcher");
  });

  it("maps Alt+arrows to rooms and Alt+Shift+arrows to unread rooms", () => {
    expect(globalShortcut(press("ArrowDown", { altKey: true }), false)).toBe("next-room");
    expect(globalShortcut(press("ArrowUp", { altKey: true }), false)).toBe("previous-room");
    expect(globalShortcut(press("ArrowDown", { altKey: true, shiftKey: true }), false)).toBe(
      "next-unread",
    );
    expect(globalShortcut(press("ArrowUp", { altKey: true, shiftKey: true }), false)).toBe(
      "previous-unread",
    );
  });

  it("leaves everything else alone, including the composer's ⌘U and ⌘⇧U", () => {
    expect(globalShortcut(press("k"), false)).toBeNull();
    expect(globalShortcut(press("ArrowDown"), false)).toBeNull();
    expect(globalShortcut(press("u", { ctrlKey: true }), false)).toBeNull();
    expect(globalShortcut(press("U", { ctrlKey: true, shiftKey: true }), false)).toBeNull();
    expect(globalShortcut(press("ArrowDown", { altKey: true, ctrlKey: true }), false)).toBeNull();
  });
});

describe("groupShortcuts", () => {
  it("lists every shortcut once, in the dialog's group order", () => {
    const sections = groupShortcuts(SHORTCUTS, "");

    expect(sections.map((section) => section.group)).toEqual([
      "Navigation",
      "Messages",
      "Composer",
      "Formatting",
    ]);
    expect(sections.flatMap((section) => section.shortcuts)).toHaveLength(SHORTCUTS.length);
    expect(sections[0]?.shortcuts[0]?.id).toBe("switcher");
  });

  it("filters by label, by key name and by group, every word matching", () => {
    const ids = (query: string) =>
      groupShortcuts(SHORTCUTS, query).flatMap((section) =>
        section.shortcuts.map((shortcut) => shortcut.id),
      );

    expect(ids("bold")).toEqual(["bold"]);
    expect(ids("shift enter")).toEqual(["newline"]);
    expect(ids("unread next")).toEqual(["next-unread"]);
    expect(ids("formatting")).toEqual(["bold", "italic", "strike", "code", "link"]);
    expect(groupShortcuts(SHORTCUTS, "nothing like this")).toEqual([]);
  });
});
