import { describe, expect, it } from "vitest";
import type { RecentSearchList } from "../../src/gen/RecentSearchList.ts";
import type { SearchResults } from "../../src/gen/SearchResults.ts";
import { THREAD_IDS } from "../s2/seed.ts";
import { errorOf, expectStatus, get, harness, send } from "../s2/testing.ts";
import { ROOM_IDS, USER_IDS } from "../seed.ts";
import {
  decodeCursor,
  encodeCursor,
  parseSearchQuery,
  plainText,
  SEARCH_PAGE_SIZE,
  SEARCH_SEED,
  stem,
  textTokens,
} from "./search.ts";

function searchPath(q: string, before: string | null = null): string {
  const query = new URLSearchParams(before === null ? { q } : { q, before });

  return `/api/v1/search?${query.toString()}`;
}

describe("parseSearchQuery", () => {
  it("turns operators into chips with what removing each searches for", () => {
    const parsed = parseSearchQuery("deploy  from:@maya, in:#engineering has:FILE is:Thread");

    expect(parsed.text).toBe("deploy");

    expect(parsed.chips).toEqual([
      {
        operator: "from",
        value: "maya",
        token: "from:@maya,",
        label: "from: maya",
        removeQuery: "deploy in:#engineering has:FILE is:Thread",
      },
      {
        operator: "in",
        value: "engineering",
        token: "in:#engineering",
        label: "in: engineering",
        removeQuery: "deploy from:@maya, has:FILE is:Thread",
      },
      {
        operator: "has",
        value: "file",
        token: "has:FILE",
        label: "has: file",
        removeQuery: "deploy from:@maya, in:#engineering is:Thread",
      },
      {
        operator: "is",
        value: "true",
        token: "is:Thread",
        label: "is: true",
        removeQuery: "deploy from:@maya, in:#engineering has:FILE",
      },
    ]);

    expect(parsed.filters.fromNames).toEqual(["maya"]);
    expect(parsed.filters.threadOnly).toBe(true);
  });

  it("leaves operators that don't parse in the text", () => {
    const parsed = parseSearchQuery("has:bogus on:2026-02-30 From:maya from:@ before:yesterday");

    expect(parsed.chips).toEqual([]);
    expect(parsed.text).toBe("has:bogus on:2026-02-30 From:maya from:@ before:yesterday");
  });

  it("needs whitespace before an operator, and keeps the last of each date", () => {
    const parsed = parseSearchQuery("xfrom:maya after:2026-10-01 after:2026-10-03 has:pin has:pin");

    expect(parsed.filters.fromNames).toEqual([]);
    expect(parsed.filters.afterDate).toBe("2026-10-03");
    expect(parsed.filters.hasValues).toEqual(["pin"]);
    expect(parsed.chips.map((chip) => chip.label)).toEqual([
      "after: 2026-10-01",
      "after: 2026-10-03",
      "has: pin",
      "has: pin",
    ]);
    expect(parsed.text).toBe("xfrom:maya");
  });

  it("removes the first copy of a repeated token only", () => {
    const parsed = parseSearchQuery("has:pin launch has:pin");

    expect(parsed.chips[0]?.removeQuery).toBe("launch has:pin");
    expect(parsed.text).toBe("launch");
  });
});

describe("matching helpers", () => {
  it("splits words and stems them roughly as Porter would", () => {
    expect(textTokens("rate-limiter, v2_beta! café")).toEqual([
      "rate",
      "limiter",
      "v2_beta",
      "café",
    ]);
    expect(stem("Invites")).toBe(stem("invited"));
    expect(stem("onboarding")).toBe("onboard");
    expect(stem("merged")).toBe(stem("merge"));
  });

  it("reads a body's plain text", () => {
    expect(plainText("<p>Fish &amp; <strong>chips</strong></p>")).toBe(" Fish &  chips  ");
  });

  it("round-trips a cursor and refuses junk", () => {
    const cursor = encodeCursor({ createdAt: "2026-10-06T12:00:00.000Z", id: 42 });

    expect(cursor).toMatch(/^[A-Za-z0-9_-]+$/u);
    expect(decodeCursor(cursor)).toEqual({ createdAt: "2026-10-06T12:00:00.000Z", id: 42 });
    expect(decodeCursor("not a cursor")).toBeNull();
    expect(decodeCursor(btoa("yesterday|42"))).toBeNull();
  });
});

describe("GET /search", () => {
  it("finds the seeded messages for a word, oldest first on the page", async () => {
    const { server } = harness();
    const reply = await get<SearchResults>(server, searchPath("  onboarding  "));

    expect(reply.query).toBe("onboarding");
    expect(reply.messages.length).toBeGreaterThan(5);

    for (const message of reply.messages) {
      expect(plainText(message.bodyHtml).toLowerCase()).toContain("onboarding");
    }

    const times = reply.messages.map((message) => message.createdAt);

    expect([...times].sort()).toEqual(times);
    expect(reply.nextCursor).toBeNull();

    const creators = new Set(reply.messages.map((message) => message.creatorId));

    expect(new Set(reply.users.map((user) => user.id))).toEqual(creators);

    for (const message of reply.messages) {
      const name = reply.conversations.find(
        (candidate) =>
          candidate.roomId === message.roomId && candidate.threadId === message.threadId,
      );

      expect(name, `conversation for ${message.id}`).toBeDefined();
    }
  });

  it("lists board posts, work threads and events whose names hold every word", async () => {
    const { server } = harness();
    const reply = await get<SearchResults>(server, searchPath("onboarding"));

    expect(reply.sections.map((section) => section.kind)).toEqual([
      "board_posts",
      "work_threads",
      "events",
    ]);

    const [boards, work, events] = reply.sections;

    expect(boards?.rows.map((row) => row.id)).toEqual([
      SEARCH_SEED.boardPostIds.onboardingChecklist,
    ]);
    expect(work?.rows.map((row) => [row.id, row.workStatus])).toEqual([
      [THREAD_IDS.design, "planned"],
    ]);
    expect(events?.rows.map((row) => row.id)).toEqual([SEARCH_SEED.eventIds.onboardingReview]);

    const launch = await get<SearchResults>(server, searchPath("launch"));
    const launchEvents = launch.sections.find((section) => section.kind === "events");

    expect(launchEvents?.rows.map((row) => [row.title, row.cancelled])).toEqual([
      ["Launch party", true],
      ["Launch dry run", false],
    ]);

    const board = launch.conversations.find(
      (name) => name.roomId === SEARCH_SEED.boardRoomId && name.threadId !== null,
    );

    expect(board?.roomKind).toBe("board");
    expect(board?.threadName).toBe("Launch week plan");
  });

  it("narrows sections by in: only, and leaves them out without words", async () => {
    const { server } = harness();
    const narrowed = await get<SearchResults>(server, searchPath("launch in:launch-planning"));

    expect(narrowed.sections.map((section) => section.kind)).toEqual(["events"]);

    const operatorsOnly = await get<SearchResults>(server, searchPath("has:image"));

    expect(operatorsOnly.sections).toEqual([]);
    expect(operatorsOnly.messages.length).toBeGreaterThan(0);
  });

  it("pages through older matches with the cursor", async () => {
    const { server } = harness();
    const first = await get<SearchResults>(server, searchPath("the"));

    expect(first.messages).toHaveLength(SEARCH_PAGE_SIZE);
    expect(first.nextCursor).not.toBeNull();

    const seen = new Set(first.messages.map((message) => message.id));
    const oldestFirst = first.messages[0]?.createdAt ?? "";
    let cursor = first.nextCursor;
    let pages = 1;

    while (cursor !== null) {
      const page = await get<SearchResults>(server, searchPath("the", cursor));

      expect(page.sections).toEqual([]);

      for (const message of page.messages) {
        expect(seen.has(message.id)).toBe(false);
        expect(message.createdAt <= oldestFirst).toBe(true);
        seen.add(message.id);
      }

      cursor = page.nextCursor;
      pages += 1;
    }

    expect(pages).toBeGreaterThan(1);
  });

  it("refuses a cursor it can't read", async () => {
    const { server } = harness();
    const response = await server.handle({ method: "GET", path: searchPath("the", "%%%") });

    expect(response.status).toBe(422);
    expect(errorOf(response.json).tag).toBe("Validation");
  });

  it("answers a blank query with an empty page", async () => {
    const { server } = harness();
    const reply = await get<SearchResults>(server, searchPath("   "));

    expect(reply).toEqual({
      query: "",
      chips: [],
      messages: [],
      users: [],
      conversations: [],
      nextCursor: null,
      sections: [],
    });
  });

  it("applies from:, in:, has: and is:", async () => {
    const { server } = harness();
    const files = await get<SearchResults>(server, searchPath("from:@maya has:file"));

    expect(files.messages.length).toBeGreaterThan(0);

    for (const message of files.messages) {
      expect(message.creatorId).toBe(USER_IDS.maya);
      // A file is an attachment or a Drive card.
      expect(
        message.attachment !== null || message.cards.some((card) => card.kind === "drive"),
      ).toBe(true);
    }

    const inRoom = await get<SearchResults>(server, searchPath("in:#engineering"));

    expect(new Set(inRoom.messages.map((message) => message.roomId))).toEqual(
      new Set([ROOM_IDS.engineering]),
    );

    const images = await get<SearchResults>(server, searchPath("has:image"));

    for (const message of images.messages) {
      expect(message.attachment?.contentType).toMatch(/^image\//u);
    }

    const pins = await get<SearchResults>(server, searchPath("has:pin launch"));

    expect(pins.messages.length).toBeGreaterThan(0);

    expect(pins.messages.map((message) => message.pinned)).toEqual(pins.messages.map(() => true));

    const replies = await get<SearchResults>(server, searchPath("is:thread invite"));

    expect(replies.messages.length).toBeGreaterThan(0);

    for (const message of replies.messages) {
      expect(message.threadId).toBe(THREAD_IDS.generalActive);
    }

    const direct = await get<SearchResults>(server, searchPath("in:maya"));

    expect(direct.messages).toEqual([]);
  });

  it("reads dates as days in the viewer's zone", async () => {
    const { server } = harness();
    const on = await get<SearchResults>(server, searchPath("on:2026-10-05"));

    expect(on.messages.length).toBeGreaterThan(0);

    for (const message of on.messages) {
      expect(message.createdAt >= "2026-10-05T04:00:00.000Z").toBe(true);
      expect(message.createdAt < "2026-10-06T04:00:00.000Z").toBe(true);
    }

    const after = await get<SearchResults>(server, searchPath("after:2026-10-05 launch"));

    for (const message of after.messages) {
      expect(message.createdAt >= "2026-10-06T04:00:00.000Z").toBe(true);
    }

    const before = await get<SearchResults>(server, searchPath("before:2026-09-26 thermostat"));

    expect(before.messages.length).toBeGreaterThan(0);

    for (const message of before.messages) {
      expect(message.createdAt < "2026-09-26T04:00:00.000Z").toBe(true);
    }
  });
});

describe("recent searches", () => {
  it("start with the seeded searches", async () => {
    const { server } = harness();
    const reply = await get<RecentSearchList>(server, "/api/v1/search/recents");

    expect(reply.searches.map((search) => search.query)).toEqual([...SEARCH_SEED.recents]);
  });

  it("move a repeat to the top and keep ten", async () => {
    const { server } = harness();

    const repeat = await expectStatus<RecentSearchList>(
      server,
      "POST",
      "/api/v1/search/recents",
      { query: " in:#engineering   rate limiter " },
      200,
    );

    expect(repeat.searches.map((search) => search.query)).toEqual([
      "in:#engineering rate limiter",
      "launch checklist",
      "from:@maya has:file",
    ]);

    for (let index = 0; index < 12; index += 1) {
      await send(server, "POST", "/api/v1/search/recents", { query: `query ${index}` });
    }

    const reply = await get<RecentSearchList>(server, "/api/v1/search/recents");

    expect(reply.searches).toHaveLength(10);
    expect(reply.searches[0]?.query).toBe("query 11");
    expect(new Set(reply.searches.map((search) => search.id)).size).toBe(10);
  });

  it("refuse a blank query", async () => {
    const { server } = harness();
    const response = await send(server, "POST", "/api/v1/search/recents", { query: "  " });

    expect(response.status).toBe(422);
    expect(errorOf(response.json).message).toContain("Enter a word to search for.");
  });

  it("clear with a 204 and come back on reset", async () => {
    const { server } = harness();
    const cleared = await send(server, "DELETE", "/api/v1/search/recents");

    expect(cleared.status).toBe(204);
    expect((await get<RecentSearchList>(server, "/api/v1/search/recents")).searches).toEqual([]);

    server.reset();

    expect((await get<RecentSearchList>(server, "/api/v1/search/recents")).searches).toHaveLength(
      SEARCH_SEED.recents.length,
    );
  });
});
