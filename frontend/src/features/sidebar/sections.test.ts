import { describe, expect, it } from "vitest";
import type { Involvement } from "../../gen/Involvement.ts";
import type { RoomKind, SidebarRow } from "../../store/model.ts";
import { initialState, type SidebarState } from "../../store/state.ts";
import {
  notificationLabel,
  peekingRows,
  rowPillCount,
  rowState,
  rowUnread,
  sectionStatus,
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
  /** The server's `notificationCount`; by default what its policy gives for the options. */
  readonly notifications?: number;
}

/** `crates/api/src/dto.rs` `notification_count` for a row with no thread or inbox-only pings. */
function policyCount(involvement: Involvement, unread: number, mentions: number): number {
  switch (involvement) {
    case "everything":
      return unread;
    case "mentions":
    case "muted":
      return mentions;
    default:
      return 0;
  }
}

function row(id: number, name: string, options: RowOptions = {}): SidebarRow {
  const unread = options.unread ?? 0;
  const mentions = options.mentions ?? 0;

  // A DM notifies for everything by default, a channel for mentions (the server's defaults).
  const involvement: Involvement =
    options.involvement ??
    (options.muted === true ? "muted" : options.kind === "direct" ? "everything" : "mentions");

  return {
    revision: 0,
    evaluatedAt: "2026-10-10T12:00:00.000000000Z",
    room: {
      id,
      kind: options.kind ?? "open",
      name,
      iconName: null,
      creatorId: 1,
      createdAt: "2026-10-01T00:00:00.000Z",
      updatedAt: options.updatedAt ?? "2026-10-01T00:00:00.000Z",
      topic: null,
    },
    membership: {
      id: id * 10,
      roomId: id,
      userId: 1,
      involvement,
      unreadAt: unread > 0 ? "2026-10-05T00:00:00.000Z" : null,
      lastReadMessageId: null,
      roomCategoryId: options.category ?? null,
      favoritePosition: options.favorite ?? null,
      stageRole: null,
    },
    displayName: name,
    directMemberIds: [],
    unreadCount: unread,
    mentionCount: mentions,
    notificationCount: options.notifications ?? policyCount(involvement, unread, mentions),
    threadNotificationCount: 0,
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
  it("counts unread rooms, and notifications as each row's policy count", () => {
    const totals = sidebarTotals(
      sidebarOf([
        row(1, "alpha", { unread: 3, mentions: 1 }),
        row(2, "Ada", { kind: "direct", unread: 2 }),
        // Muted: unread only because of the mention, which counts like any other.
        row(3, "noise", { unread: 9, mentions: 4, muted: true }),
        row(4, "hush", { muted: true }),
        row(5, "Bo", { kind: "direct", unread: 6, mentions: 1, muted: true }),
        // A thread ping in a read room still counts (the server's count, not the unread roots).
        row(6, "ops", { involvement: "everything", notifications: 1 }),
      ]),
    );

    // "ops" reads as unread too: its ping bolds it, so the rail's dot agrees with the row.
    expect(totals).toEqual({ unreadRooms: 5, mentions: 1 + 2 + 4 + 1 + 1 });
  });
});

describe("sidebarTotals through pending changes", () => {
  it("stops counting a room at once when it is being hidden, muted or set to nothing", () => {
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

    const quiet: SidebarState = {
      ...base,
      overlay: { memberships: { 3: { involvement: "nothing" } }, categories: {} },
    };

    expect(sidebarTotals(base)).toEqual({ unreadRooms: 3, mentions: 5 });
    expect(sidebarTotals(pending)).toEqual({ unreadRooms: 1, mentions: 2 });
    expect(sidebarTotals(quiet).mentions).toBe(3);
  });
});

describe("rows", () => {
  it("read a muted room as unread once mentioned, and count its mentions", () => {
    const mentioned = row(3, "noise", { unread: 9, mentions: 4, muted: true });
    const quiet = row(4, "hush", { muted: true });

    expect(rowPillCount(mentioned)).toBe(4);
    expect(rowState(mentioned, false)).toBe("unread");
    expect(rowState(quiet, false)).toBe("muted");
    expect(rowState(mentioned, true)).toBe("selected");
    expect(rowState(row(1, "alpha"), false)).toBeNull();
  });
});

describe("rowUnread", () => {
  it("bolds a row with a ping even when its root timeline is read", () => {
    // A thread @mention: the membership stays read, but classic would have notified.
    const pinged = row(4, "alerts", { involvement: "everything", notifications: 1 });
    // A muted room's thread mention counts and bolds too, as classic still pushes it.
    const mutedPing = row(5, "noise", { muted: true, mentions: 1 });

    expect(rowUnread(pinged)).toBe(true);
    expect(rowState(pinged, false)).toBe("unread");
    expect(rowPillCount(mutedPing)).toBe(1);
    expect(rowState(mutedPing, false)).toBe("unread");
    expect(sectionUnread([pinged])).toEqual({ unread: true, count: 1 });
    expect(sidebarTotals(sidebarOf([pinged, mutedPing]))).toEqual({ unreadRooms: 2, mentions: 2 });
    // "nothing" never notifies, so a stale server count neither bolds nor counts.
    expect(rowUnread(row(6, "quiet", { involvement: "nothing", notifications: 2 }))).toBe(false);
    expect(rowUnread(row(1, "general"))).toBe(false);
  });
});

describe("peekingRows", () => {
  it("keeps a folded section's pinged rows on show, even with the room timeline read", () => {
    const pinged = row(4, "alerts", { involvement: "everything", notifications: 1 });
    const unreadRoom = row(5, "design", { unread: 2 });
    const selected = row(6, "general");
    const quiet = row(7, "random");

    expect(
      peekingRows([pinged, unreadRoom, selected, quiet], 6).map((entry) => entry.room.id),
    ).toEqual([4, 5, 6]);
  });
});

describe("the red pill: what would have notified you", () => {
  it("leaves plain unread activity in a channel to the bold name, with no count", () => {
    const busy = row(1, "general", { unread: 12 });

    expect(rowState(busy, false)).toBe("unread");
    expect(rowPillCount(busy)).toBe(0);
    expect(rowPillCount(row(2, "private", { kind: "closed", unread: 3 }))).toBe(0);
  });

  it("is the server's notification count: mentions, replies, thread pings", () => {
    expect(rowPillCount(row(1, "general", { unread: 12, mentions: 2 }))).toBe(2);
    // A reply and a thread mention the mention count leaves out.
    expect(rowPillCount(row(1, "general", { unread: 12, mentions: 1, notifications: 3 }))).toBe(3);
  });

  it("counts every unread message where every message notifies (a DM, an everything room)", () => {
    expect(rowPillCount(row(2, "Ada", { kind: "direct", unread: 3 }))).toBe(3);
    expect(rowPillCount(row(4, "alerts", { unread: 5, involvement: "everything" }))).toBe(5);
    // A thread @mention in a read "everything" room still counts.
    expect(rowPillCount(row(4, "alerts", { involvement: "everything", notifications: 1 }))).toBe(1);
  });

  it("follows the policy for a DM set to mentions, and counts nothing for nothing", () => {
    expect(
      rowPillCount(row(2, "Ada", { kind: "direct", unread: 3, involvement: "mentions" })),
    ).toBe(0);
    // Overlaid involvement wins at once over a count the server made for the old one.
    expect(rowPillCount(row(5, "quiet", { involvement: "nothing", notifications: 4 }))).toBe(0);
    expect(rowPillCount(row(5, "noise", { muted: true, mentions: 1, notifications: 6 }))).toBe(1);
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

describe("notificationLabel", () => {
  it("names the red count for screen readers", () => {
    expect(notificationLabel(1)).toBe("1 notification");
    expect(notificationLabel(4)).toBe("4 notifications");
  });
});

describe("sectionStatus", () => {
  it("says what a folded category hides, or nothing when it is all read", () => {
    expect(sectionStatus({ unread: false, count: 0 })).toBeNull();
    expect(sectionStatus({ unread: true, count: 0 })).toBe("Unread");
    expect(sectionStatus({ unread: true, count: 2 })).toBe("Unread, 2 notifications");
    expect(sectionStatus({ unread: false, count: 1 })).toBe("1 notification");
  });
});
