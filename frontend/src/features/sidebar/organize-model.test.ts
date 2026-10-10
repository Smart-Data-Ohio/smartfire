import { describe, expect, it } from "vitest";
import type { RoomCategory, RoomKind, SidebarRow } from "../../store/model.ts";
import {
  categoryTargets,
  insertionIndex,
  involvementChoices,
  reorderedIds,
  roomTargets,
  slotForSection,
} from "./organize-model.ts";
import type { SidebarSection } from "./sections.ts";

function row(id: number, name: string, kind: RoomKind = "open", favorite: number | null = null) {
  const entry: SidebarRow = {
    revision: 0,
    evaluatedAt: "2026-10-10T12:00:00.000000000Z",
    room: {
      id,
      kind,
      name,
      iconName: null,
      creatorId: 1,
      createdAt: "2026-10-01T00:00:00.000Z",
      updatedAt: "2026-10-01T00:00:00.000Z",
      topic: null,
    },
    membership: {
      id: id * 10,
      roomId: id,
      userId: 1,
      involvement: kind === "direct" ? "everything" : "mentions",
      unreadAt: null,
      lastReadMessageId: null,
      roomCategoryId: null,
      favoritePosition: favorite,
      stageRole: null,
    },
    displayName: name,
    directMemberIds: [],
    unreadCount: 0,
    mentionCount: 0,
    notificationCount: 0,
    threadNotificationCount: 0,
  };

  return entry;
}

const LAUNCH: RoomCategory = { id: 1, name: "Launch", collapsed: false, position: 0 };

const TEAM: RoomCategory = { id: 2, name: "Team", collapsed: false, position: 1 };

function section(
  kind: SidebarSection["kind"],
  title: string,
  rows: readonly SidebarRow[] = [],
  category: RoomCategory | null = null,
): SidebarSection {
  return {
    key: category === null ? kind : `category-${category.id}`,
    kind,
    title,
    rows,
    category,
    collapsed: false,
  };
}

const general = row(1, "general");

const engineering = row(2, "engineering", "open", 0);

const maya = row(3, "Maya Okafor", "direct", 1);

const lounge = row(4, "Lounge", "voice");

const favorites = section("favorites", "Favourites", [engineering, maya]);

const launch = section("category", "Launch", [], LAUNCH);

const team = section("category", "Team", [], TEAM);

const channels = section("channels", "Channels", [general]);

const voice = section("voice", "Voice", [lounge]);

const direct = section("direct", "Direct messages");

const sections = [favorites, launch, team, channels, voice, direct];

describe("slotForSection", () => {
  it("takes channels into categories and Channels, and stars anything", () => {
    expect(slotForSection(launch, general, 0)).toEqual({
      kind: "category",
      categoryId: 1,
    });
    expect(slotForSection(channels, general, 0)).toEqual({ kind: "channels" });
    expect(slotForSection(favorites, lounge, 2)).toEqual({
      kind: "favorite",
      index: 2,
    });
  });

  it("keeps direct and voice rooms out of categories; a favourite one goes home", () => {
    expect(slotForSection(launch, maya, 0)).toBeNull();
    expect(slotForSection(channels, maya, 0)).toBeNull();
    expect(slotForSection(direct, maya, 0)).toEqual({ kind: "unfavorite" });
    expect(slotForSection(voice, maya, 0)).toBeNull();
    expect(slotForSection(voice, lounge, 0)).toBeNull();
  });
});

describe("roomTargets", () => {
  it("lists the favourites' gaps, then each section that takes the room", () => {
    expect(roomTargets(sections, general).map((target) => target.label)).toEqual([
      "Top of Favourites",
      "Favourites, after engineering",
      "Favourites, after Maya Okafor",
      "Launch",
      "Team",
      "Channels",
    ]);
  });

  it("doesn't count the dragged favourite among the gaps, and names the way home", () => {
    expect(roomTargets(sections, maya).map((target) => target.label)).toEqual([
      "Top of Favourites",
      "Favourites, after engineering",
      "Direct messages, out of Favourites",
    ]);
  });

  it("offers the top of Favourites when there are none yet", () => {
    expect(roomTargets(sections.slice(1), general)[0]).toEqual({
      target: { kind: "room", slot: { kind: "favorite", index: 0 } },
      label: "Top of Favourites",
    });
  });
});

describe("category moves", () => {
  it("reads each place among the others", () => {
    expect(categoryTargets([LAUNCH, TEAM], TEAM).map((target) => target.label)).toEqual([
      "First category",
      "After Launch, position 2 of 2",
    ]);
  });

  it("moves an id to an index among the others, clamped", () => {
    const third: RoomCategory = { id: 3, name: "Ops", collapsed: false, position: 2 };

    expect(reorderedIds([LAUNCH, TEAM, third], 3, 0)).toEqual([3, 1, 2]);
    expect(reorderedIds([LAUNCH, TEAM, third], 1, 9)).toEqual([2, 3, 1]);
    expect(reorderedIds([LAUNCH, TEAM, third], 2, -1)).toEqual([2, 1, 3]);
  });

  it("counts the midpoints above the pointer", () => {
    expect(insertionIndex([10, 30, 50], 5)).toBe(0);
    expect(insertionIndex([10, 30, 50], 31)).toBe(2);
    expect(insertionIndex([10, 30, 50], 99)).toBe(3);
  });
});

describe("involvementChoices", () => {
  it("offers direct messages all, none or muted; rooms every level", () => {
    expect(involvementChoices(maya).map((choice) => choice.level)).toEqual([
      "everything",
      "nothing",
      "muted",
    ]);
    expect(involvementChoices(general).map((choice) => choice.label)).toEqual([
      "All messages",
      "Mentions",
      "No notifications",
      "Muted",
      "Hidden",
    ]);
  });
});
