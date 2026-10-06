import { describe, expect, it } from "vitest";
import type { RoomKind, SidebarRow } from "../../store/model.ts";
import { initialState, type SidebarState } from "../../store/state.ts";
import { sidebarSections, sidebarTotals } from "./sections.ts";

interface RowOptions {
  readonly kind?: RoomKind;
  readonly favorite?: number;
  readonly category?: number;
  readonly updatedAt?: string;
  readonly unread?: number;
  readonly mentions?: number;
  readonly muted?: boolean;
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
      involvement: options.muted === true ? "muted" : "mentions",
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
  it("counts unread rooms and mentions, DMs by message, skipping muted rooms", () => {
    const totals = sidebarTotals(
      sidebarOf([
        row(1, "alpha", { unread: 3, mentions: 1 }),
        row(2, "Ada", { kind: "direct", unread: 2 }),
        row(3, "noise", { unread: 9, mentions: 4, muted: true }),
      ]),
    );

    expect(totals).toEqual({ unreadRooms: 2, mentions: 3 });
  });
});
