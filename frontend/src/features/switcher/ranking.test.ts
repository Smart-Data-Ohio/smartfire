import { describe, expect, it } from "vitest";
import { sidebarFixture, sidebarRowFixture } from "../../api/testing.ts";
import type { SidebarRow } from "../../store/model.ts";
import { initialState, type SidebarState } from "../../store/state.ts";
import { matchRange, matchScore, normalizeQuery } from "./match.ts";
import {
  flattenSections,
  localItems,
  mergeItems,
  type RemoteCatalogue,
  rankItems,
  readsUnread,
  remoteItems,
} from "./ranking.ts";
import { withRecent } from "./recents.ts";

const VIEWER = 7;

/** Unread in a room that notifies for mentions only, so its count is the mentions (the default). */
function unread(row: SidebarRow, mentions = 0): SidebarRow {
  return {
    ...row,
    unreadCount: 3,
    mentionCount: mentions,
    notificationCount: mentions,
    threadNotificationCount: 0,
    membership: {
      ...row.membership,
      involvement: "mentions",
      unreadAt: "2026-10-05T00:00:00.000Z",
    },
  };
}

function touched(row: SidebarRow, updatedAt: string): SidebarRow {
  return { ...row, room: { ...row.room, updatedAt } };
}

function sidebarOf(rows: readonly SidebarRow[]): SidebarState {
  const loaded = sidebarFixture(rows);

  return {
    ...initialState.sidebar,
    status: "ready",
    order: loaded.rows.map((row) => row.room.id),
    rows: Object.fromEntries(loaded.rows.map((row) => [row.room.id, row])),
  };
}

const SIDEBAR = sidebarOf([
  touched(sidebarRowFixture(1, "general"), "2026-10-04T00:00:00.000Z"),
  unread(sidebarRowFixture(2, "design"), 2),
  touched(sidebarRowFixture(3, "engineering"), "2026-10-05T00:00:00.000Z"),
  sidebarRowFixture(9, "Maya Chen", "direct", [2]),
  sidebarRowFixture(11, "Jonah and Priya", "direct", [3, 4]),
]);

const NAMES = new Map([
  [2, "Maya Chen"],
  [3, "Jonah Park"],
  [4, "Priya Shah"],
  [5, "Sam Ortiz"],
]);

const nameOf = (userId: number) => NAMES.get(userId);

const CATALOGUE: RemoteCatalogue = {
  rooms: [
    {
      roomId: 1,
      name: "general",
      kind: "channel",
      unread: false,
      muted: false,
      favorite: false,
    },
    { roomId: 9, name: "Maya Chen", kind: "dm", unread: false, muted: false, favorite: false },
    { roomId: 40, name: "archive", kind: "channel", unread: false, muted: true, favorite: false },
  ],
  people: [
    { userId: 2, directRoomId: 9 },
    { userId: 5, directRoomId: null },
  ],
  threads: [{ threadId: 70, name: "Launch checklist", roomId: 1, roomName: "general" }],
};

describe("matchScore", () => {
  it("prefers exact, then prefix, then word start, then substring, then subsequence", () => {
    const query = normalizeQuery("des");

    expect(matchScore("des", query)).toBeGreaterThan(matchScore("design", query) ?? 0);
    expect(matchScore("design", query)).toBeGreaterThan(matchScore("ux-design", query) ?? 0);
    expect(matchScore("ux-design", query)).toBeGreaterThan(matchScore("undesigned", query) ?? 0);
    expect(matchScore("undesigned", query)).toBeGreaterThan(
      matchScore("data engineering stuff", query) ?? 0,
    );
    expect(matchScore("general", query)).toBeNull();
  });

  it("ignores case, accents and a leading # or @", () => {
    expect(matchScore("Lúcia Ferreira", normalizeQuery("@luc"))).not.toBeNull();
    expect(matchScore("general", normalizeQuery("#GEN"))).not.toBeNull();
  });

  it("matches initials across words as a subsequence", () => {
    expect(matchScore("launch-planning", normalizeQuery("lp"))).not.toBeNull();
  });

  it("matches one or two letters only as a run or as initials", () => {
    expect(matchScore("Jonah Lindqvist", normalizeQuery("an"))).toBeNull();
    expect(matchScore("Jonah Lindqvist", normalizeQuery("jl"))).not.toBeNull();
  });
});

describe("matchRange", () => {
  it("finds the typed run with accents folded, in the label's own indices", () => {
    expect(matchRange("Lucía Fernández", normalizeQuery("an"))).toEqual([10, 12]);
    expect(matchRange("Quick poll 👍 standup", normalizeQuery("stand"))).toEqual([14, 19]);
    expect(matchRange("general", normalizeQuery("xyz"))).toBeNull();
    expect(matchRange("general", "")).toBeNull();
  });
});

describe("localItems", () => {
  it("turns a one-to-one DM into its person and keeps group DMs as rooms", () => {
    const items = localItems(SIDEBAR, VIEWER);

    expect(items.map((item) => item.key)).toEqual([
      "room:1",
      "room:2",
      "room:3",
      "room:11",
      "person:2",
    ]);
    expect(items.find((item) => item.key === "person:2")?.roomId).toBe(9);
    expect(items.find((item) => item.key === "room:11")?.memberIds).toEqual([3, 4]);
    expect(items.find((item) => item.key === "room:2")?.count).toBe(2);
  });
});

describe("readsUnread", () => {
  const muted = (row: SidebarRow): SidebarRow => ({
    ...row,
    membership: { ...row.membership, involvement: "muted" },
  });

  it("bolds a muted room only for a ping, as the sidebar's muted row does", () => {
    const items = localItems(
      sidebarOf([
        unread(sidebarRowFixture(2, "design"), 2),
        muted(unread(sidebarRowFixture(4, "noise"), 1)),
        muted(unread(sidebarRowFixture(5, "chatter"))),
        sidebarRowFixture(1, "general"),
      ]),
      VIEWER,
    );

    const reads = (key: string) => {
      const item = items.find((entry) => entry.key === key);

      return item === undefined ? undefined : readsUnread(item);
    };

    expect(reads("room:2")).toBe(true);
    expect(reads("room:4")).toBe(true);
    expect(reads("room:5")).toBe(false);
    expect(reads("room:1")).toBe(false);
  });
});

describe("mergeItems", () => {
  const merged = mergeItems(localItems(SIDEBAR, VIEWER), remoteItems(CATALOGUE), nameOf);
  const keys = merged.map((item) => item.key);

  it("dedupes rooms the sidebar and server both list, the sidebar's copy winning", () => {
    expect(keys.filter((key) => key === "room:1")).toHaveLength(1);
    expect(keys.filter((key) => key === "person:2")).toHaveLength(1);
    expect(merged.find((item) => item.key === "room:1")?.updatedAt).toBe(
      "2026-10-04T00:00:00.000Z",
    );
  });

  it("doesn't list a DM room beside the person it belongs to", () => {
    expect(keys).not.toContain("room:9");
  });

  it("adds what only the server knows: other rooms, people without a DM, threads", () => {
    expect(keys).toContain("room:40");
    expect(merged.find((item) => item.key === "person:5")).toMatchObject({
      label: "Sam Ortiz",
      roomId: null,
    });
    expect(merged.find((item) => item.key === "thread:70")).toMatchObject({
      label: "Launch checklist",
      detail: "general",
      roomId: 1,
    });
  });

  it("drops a person whose name isn't known yet", () => {
    const anonymous = mergeItems(
      [],
      remoteItems({ ...CATALOGUE, people: [{ userId: 99, directRoomId: null }] }),
      nameOf,
    );

    expect(anonymous.map((item) => item.key)).not.toContain("person:99");
  });
});

describe("rankItems", () => {
  const items = mergeItems(localItems(SIDEBAR, VIEWER), remoteItems(CATALOGUE), nameOf);

  it("opens on recent picks, topped up with the latest conversations, then unread", () => {
    const recents = withRecent(withRecent([], "room:11"), "person:5");
    const sections = rankItems(items, "", recents);

    expect(sections.map((section) => section.key)).toEqual(["recent", "unread", "threads"]);
    expect(sections[0]?.items.map((item) => item.key)).toEqual([
      "person:5",
      "room:11",
      "room:3",
      "room:1",
      "person:2",
      "room:40",
    ]);
    expect(sections[1]?.items.map((item) => item.key)).toEqual(["room:2"]);
    expect(sections[2]?.items.map((item) => item.key)).toEqual(["thread:70"]);
  });

  it("groups matches into sections, best section first", () => {
    const sections = rankItems(items, "maya", []);

    expect(sections[0]?.key).toBe("people");
    expect(flattenSections(sections)[0]?.key).toBe("person:2");
  });

  it("ranks a prefix above a scattered match and nudges recent picks up", () => {
    const plain = flattenSections(rankItems(items, "gen", [])).map((item) => item.key);

    expect(plain[0]).toBe("room:1");

    const twins = localItems(
      sidebarOf([sidebarRowFixture(51, "launch-one"), sidebarRowFixture(52, "launch-two")]),
      VIEWER,
    );

    const keys = (recents: readonly string[]) =>
      flattenSections(rankItems(twins, "launch", recents)).map((item) => item.key);

    expect(keys([])).toEqual(["room:51", "room:52"]);
    expect(keys(["room:52"])).toEqual(["room:52", "room:51"]);
  });

  it("finds threads by name and says nothing when nothing matches", () => {
    expect(flattenSections(rankItems(items, "checklist", [])).map((item) => item.key)).toEqual([
      "thread:70",
    ]);
    expect(rankItems(items, "zzz", [])).toEqual([]);
  });
});

describe("withRecent", () => {
  it("moves a pick to the front without duplicates and caps the list", () => {
    let recents: readonly string[] = [];

    for (let index = 0; index < 20; index += 1) {
      recents = withRecent(recents, `room:${index}`);
    }

    recents = withRecent(recents, "room:15");

    expect(recents[0]).toBe("room:15");
    expect(new Set(recents).size).toBe(recents.length);
    expect(recents.length).toBe(12);
  });
});
