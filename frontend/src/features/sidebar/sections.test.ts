import { describe, expect, it } from "vitest";
import type { Involvement } from "../../gen/Involvement.ts";
import type { RoomKind, SidebarRow } from "../../store/model.ts";
import { initialState, type SidebarState } from "../../store/state.ts";
import {
  rowPillCount,
  rowPillNoun,
  rowState,
  sectionUnread,
  sidebarSections,
  sidebarTotals,
} from "./sections.ts";

interface RowOptions {
  readonly kind?: RoomKind;
  readonly favorite?: number;
  readonly category?: number;
  readonly updatedAt?: string;
  readonly unread?: number;
  readonly mentions?: number;
  readonly muted?: boolean;
  readonly involvement?: Involvement;
}

function row(id: number, name: string, options: RowOptions = {}): SidebarRow {
  const unread = options.unread ?? 0;

  return {
    room: {
      id,
      kind: options.kind ?? "open",
      name,
      iconName: null,
      creatorId: 1,
      createdAt: "2026-10-01T00:00:00.000Z",
      updatedAt: options.updatedAt ?? "2026-10-01T00:00:00.000Z",
    },
    membership: {
      id: id * 10,
      roomId: id,
      userId: 1,
      involvement: options.involvement ?? (options.muted === true ? "muted" : "mentions"),
      unreadAt: unread > 0 ? "2026-10-05T00:00:00.000Z" : null,
      lastReadMessageId: null,
      roomCategoryId: options.category ?? null,
      favoritePosition: options.favorite ?? null,
      stageRole: null,
    },
    displayName: name,
    directMemberIds: [],
    unreadCount: unread,
    mentionCount: options.mentions ?? 0,
  };
}

function sidebarOf(rows: readonly SidebarRow[]): SidebarState {
  return {
    ...initialState.sidebar,
    status: "ready",
    order: rows.map((entry) => entry.room.id),
    rows: Object.fromEntries(rows.map((entry) => [entry.room.id, entry])),
    categories: [{ id: 7, name: "Projects", collapsed: false, position: 0 }],
  };
}

describe("sidebarSections", () => {
  it("partitions rows like the classic sidebar", () => {
    const sections = sidebarSections(
      sidebarOf([
        row(1, "alpha"),
        row(2, "beta", { favorite: 2 }),
        row(3, "gamma", { favorite: 1 }),
        row(4, "launch", { category: 7 }),
        row(5, "lounge", { kind: "voice" }),
        row(6, "Ada", { kind: "direct", updatedAt: "2026-10-02T00:00:00.000Z" }),
        row(8, "Grace", { kind: "direct", updatedAt: "2026-10-04T00:00:00.000Z" }),
        row(9, "orphan", { category: 99 }),
      ]),
    );

    expect(
      sections.map((section) => [section.key, section.rows.map((entry) => entry.room.id)]),
    ).toEqual([
      ["favorites", [3, 2]],
      ["category-7", [4]],
      ["channels", [1, 9]],
      ["voice", [5]],
      ["direct", [8, 6]],
    ]);
  });

  it("keeps every category, Channels and Direct messages even when empty", () => {
    expect(sidebarSections(sidebarOf([])).map((section) => section.key)).toEqual([
      "category-7",
      "channels",
      "direct",
    ]);
  });

  it("leaves invisible rooms out", () => {
    const hidden = row(2, "hidden");

    const sections = sidebarSections(
      sidebarOf([
        row(1, "alpha"),
        { ...hidden, membership: { ...hidden.membership, involvement: "invisible" } },
      ]),
    );

    expect(
      sections.find((section) => section.key === "channels")?.rows.map((entry) => entry.room.id),
    ).toEqual([1]);
  });

  it("draws pending organising changes over the server's rows", () => {
    const base = sidebarOf([row(1, "alpha"), row(2, "beta", { favorite: 0 })]);

    const sections = sidebarSections({
      ...base,
      overlay: {
        memberships: { 1: { roomCategoryId: -1 }, 2: { favoritePosition: null } },
        categories: { [-1]: { id: -1, name: "Drafts", collapsed: false, position: 1 }, 7: null },
      },
    });

    expect(
      sections.map((section) => [section.key, section.rows.map((entry) => entry.room.id)]),
    ).toEqual([
      ["category--1", [1]],
      ["channels", [2]],
      ["direct", []],
    ]);
  });
});

describe("sidebarTotals", () => {
  it("counts unread rooms and mentions, DMs by message, a muted room by its mentions", () => {
    const totals = sidebarTotals(
      sidebarOf([
        row(1, "alpha", { unread: 3, mentions: 1 }),
        row(2, "Ada", { kind: "direct", unread: 2 }),
        // Muted: unread only because of the mention, which counts like any other.
        row(3, "noise", { unread: 9, mentions: 4, muted: true }),
        row(4, "hush", { muted: true }),
        row(5, "Bo", { kind: "direct", unread: 6, mentions: 1, muted: true }),
      ]),
    );

    expect(totals).toEqual({ unreadRooms: 4, mentions: 1 + 2 + 4 + 1 });
  });
});

describe("sidebarTotals through pending changes", () => {
  it("stops counting a room at once when it is being hidden or muted", () => {
    const base = sidebarOf([
      row(1, "alpha", { unread: 3, mentions: 1 }),
      row(2, "Ada", { kind: "direct", unread: 2 }),
      row(3, "beta", { unread: 1, mentions: 2 }),
    ]);

    const pending: SidebarState = {
      ...base,
      overlay: {
        memberships: {
          1: { involvement: "invisible" },
          2: { involvement: "muted", unreadAt: null },
        },
        categories: {},
      },
    };

    expect(sidebarTotals(base)).toEqual({ unreadRooms: 3, mentions: 5 });
    expect(sidebarTotals(pending)).toEqual({ unreadRooms: 1, mentions: 2 });
  });
});

describe("rows", () => {
  it("count a muted room's mentions only, and read it as unread once mentioned", () => {
    const mentioned = row(3, "noise", { unread: 9, mentions: 4, muted: true });
    const quiet = row(4, "hush", { muted: true });
    const direct = row(5, "Bo", { kind: "direct", unread: 6, mentions: 1, muted: true });

    expect(rowPillCount(mentioned)).toBe(4);
    expect(rowPillCount(direct)).toBe(1);
    expect(rowPillCount(row(2, "Ada", { kind: "direct", unread: 2 }))).toBe(2);
    expect(rowState(mentioned, false)).toBe("unread");
    expect(rowState(quiet, false)).toBe("muted");
    // Read again, the mention no longer shows.
    expect(
      rowPillCount({ ...mentioned, membership: { ...mentioned.membership, unreadAt: null } }),
    ).toBe(0);
    expect(rowState(mentioned, true)).toBe("selected");
    expect(rowState(row(1, "alpha"), false)).toBeNull();
  });
});

describe("the red pill counts notifications only", () => {
  it("leaves plain unread activity in a channel to the bold name, with no count", () => {
    const busy = row(1, "general", { unread: 12 });

    expect(rowState(busy, false)).toBe("unread");
    expect(rowPillCount(busy)).toBe(0);
    expect(rowPillCount(row(2, "private", { kind: "closed", unread: 3 }))).toBe(0);
  });

  it("counts a channel's mentions, not its unread messages", () => {
    expect(rowPillCount(row(1, "general", { unread: 12, mentions: 2 }))).toBe(2);
  });

  it("counts every unread direct message, each one being addressed to you", () => {
    expect(rowPillCount(row(2, "Ada", { kind: "direct", unread: 3 }))).toBe(3);
    expect(
      rowPillCount(row(3, "Ada, Bo", { kind: "direct", unread: 2, involvement: "everything" })),
    ).toBe(2);
  });

  it("counts every unread message in a room set to notify for everything", () => {
    expect(rowPillCount(row(4, "alerts", { unread: 5, involvement: "everything" }))).toBe(5);
  });

  it("still counts mentions in a room set to nothing, as the inbox does", () => {
    expect(rowPillCount(row(5, "quiet", { unread: 5, mentions: 1, involvement: "nothing" }))).toBe(
      1,
    );
    expect(rowPillCount(row(5, "quiet", { unread: 5, involvement: "nothing" }))).toBe(0);
  });

  it("totals the rail from the same counts: unread rooms apart from notifications", () => {
    const sidebar = sidebarOf([
      row(1, "general", { unread: 12 }),
      row(2, "design", { unread: 4, mentions: 1 }),
      row(3, "Ada", { kind: "direct", unread: 2 }),
      row(4, "alerts", { unread: 3, involvement: "everything" }),
    ]);

    expect(sidebarTotals(sidebar)).toEqual({ unreadRooms: 4, mentions: 6 });
  });
});

describe("a folded section's summary", () => {
  it("marks plain unread without a count, and totals the notifications inside", () => {
    expect(sectionUnread([row(1, "general", { unread: 12 }), row(2, "design")])).toEqual({
      unread: true,
      count: 0,
    });
    expect(
      sectionUnread([
        row(1, "general", { unread: 12, mentions: 1 }),
        row(2, "Ada", { kind: "direct", unread: 2 }),
      ]),
    ).toEqual({ unread: true, count: 3 });
    expect(sectionUnread([row(1, "general"), row(2, "design")])).toEqual({
      unread: false,
      count: 0,
    });
  });
});

describe("rowPillNoun", () => {
  it("says what the pill counts", () => {
    expect(rowPillNoun(row(1, "general"))).toBe("mentions");
    expect(rowPillNoun(row(2, "Ada", { kind: "direct" }))).toBe("unread");
    expect(rowPillNoun(row(3, "alerts", { involvement: "everything" }))).toBe("unread");
    expect(rowPillNoun(row(4, "Bo", { kind: "direct", muted: true }))).toBe("mentions");
  });
});
