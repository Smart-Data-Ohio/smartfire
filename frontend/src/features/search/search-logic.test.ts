import { describe, expect, it } from "vitest";
import type { SearchChip } from "../../gen/SearchChip.ts";
import type { SwitcherItem } from "../switcher/ranking.ts";
import { messageFixture } from "../threads/test-fixtures.ts";
import { chipLabel, hitDay, messageCount, resultsAnnouncement } from "./format.ts";
import { highlightHtml, markRuns, wordMatcher } from "./highlight.ts";
import {
  appendToken,
  isoDay,
  lastToken,
  operatorPrefixes,
  partialOperator,
  replaceLastToken,
  setFilter,
  textWords,
} from "./query.ts";
import { isEditable, searchShortcut } from "./search-keys.ts";
import { flattenTypeahead, fromHandle, typeaheadSections } from "./typeahead.ts";

const NOW = new Date(2026, 9, 6, 12, 0).getTime();

function item(extra: Partial<SwitcherItem> & Pick<SwitcherItem, "key" | "kind" | "label">) {
  return {
    detail: null,
    roomId: null,
    roomKind: null,
    userId: null,
    memberIds: [],
    threadId: null,
    unread: false,
    count: 0,
    muted: false,
    favorite: false,
    updatedAt: null,
    ...extra,
  } satisfies SwitcherItem;
}

const ITEMS = [
  item({ key: "person:2", kind: "person", label: "Maya Okafor", userId: 2, roomId: 40 }),
  item({ key: "person:6", kind: "person", label: "Priya Raman", userId: 6 }),
  item({ key: "room:7", kind: "room", label: "launch-planning", roomId: 7, roomKind: "closed" }),
  item({ key: "room:3", kind: "room", label: "engineering", roomId: 3, roomKind: "open" }),
  item({ key: "room:40", kind: "room", label: "Maya Okafor", roomId: 40, roomKind: "direct" }),
];

const RECENTS = [
  { id: 1, query: "launch checklist", searchedAt: "2026-10-06T10:00:00.000Z" },
  { id: 2, query: "from:@maya has:file", searchedAt: "2026-10-05T10:00:00.000Z" },
];

const SOURCE = { recents: RECENTS, items: ITEMS, now: NOW };

function key(extra: Partial<Parameters<typeof searchShortcut>[0]>) {
  return {
    key: "",
    code: "",
    metaKey: false,
    ctrlKey: false,
    altKey: false,
    shiftKey: false,
    ...extra,
  };
}

describe("query", () => {
  it("splits the free text into words and leaves the operators out", () => {
    expect(textWords("launch from:@maya  rate-limiter has:file")).toEqual([
      "launch",
      "rate",
      "limiter",
    ]);

    expect(textWords("from:@maya")).toEqual([]);
    // An operator only counts at a word's start, as on the server.
    expect(textWords("xfrom:maya")).toEqual(["xfrom", "maya"]);
  });

  it("reads the operator being typed at the end of the field", () => {
    expect(lastToken("launch from:ma")).toBe("from:ma");
    expect(lastToken("launch ")).toBe("");
    expect(partialOperator("launch from:ma")).toEqual({ operator: "from", partial: "ma" });
    expect(partialOperator("launch in:")).toEqual({ operator: "in", partial: "" });
    expect(partialOperator("launch from:ma ")).toBeNull();
    expect(partialOperator("launch")).toBeNull();
  });

  it("offers operators by prefix from two letters", () => {
    expect(operatorPrefixes("f")).toEqual([]);
    expect(operatorPrefixes("launch fr")).toEqual(["from", "from_id"]);
    expect(operatorPrefixes("ha")).toEqual(["has"]);
    expect(operatorPrefixes("from:")).toEqual([]);
  });

  it("replaces the last token and appends filters", () => {
    expect(replaceLastToken("launch from:ma", "from:@maya")).toBe("launch from:@maya ");
    expect(replaceLastToken("launch ", "has:")).toBe("launch has: ");
    expect(appendToken("launch  ", "has:file")).toBe("launch has:file");
    expect(appendToken("", "has:file")).toBe("has:file");
  });

  it("writes a local day as YYYY-MM-DD", () => {
    expect(isoDay(new Date(2026, 0, 5, 23, 30).getTime())).toBe("2026-01-05");
  });
});

describe("highlight", () => {
  it("matches a word by prefix or a rough stem", () => {
    const matches = wordMatcher(["launch", "reviews"]);

    expect(matches?.("Launching")).toBe(true);
    expect(matches?.("launched")).toBe(true);
    expect(matches?.("review")).toBe(true);
    expect(matches?.("lunch")).toBe(false);
    expect(wordMatcher([])).toBeNull();
  });

  it("splits text into marked and unmarked runs", () => {
    const matches = wordMatcher(["doc"]);

    expect(matches === null ? [] : markRuns("the doc, the docs", matches)).toEqual([
      { text: "the ", marked: false },
      { text: "doc", marked: true },
      { text: ", the ", marked: false },
      { text: "docs", marked: true },
    ]);
  });

  it("marks words in text nodes only, never in tags or attributes", () => {
    const html = '<p>See the <a href="https://x.example/launch">launch doc</a> now</p>';

    expect(highlightHtml(html, ["launch"])).toBe(
      '<p>See the <a href="https://x.example/launch"><mark class="search-mark">launch</mark> doc</a> now</p>',
    );

    expect(highlightHtml(html, [])).toBe(html);
  });

  it("keeps markup in the body as markup", () => {
    expect(highlightHtml("<p>a &lt;b&gt; tag</p>", ["tag"])).toBe(
      '<p>a &lt;b&gt; <mark class="search-mark">tag</mark></p>',
    );
  });
});

describe("typeahead", () => {
  it("shows recent searches and the operators while the field is empty", () => {
    const sections = typeaheadSections("", SOURCE);

    expect(sections.map((entry) => entry.title)).toEqual(["Recent searches", "Narrow your search"]);
    expect(sections[0]?.items.map((entry) => entry.action)).toEqual([
      { kind: "search", query: "launch checklist" },
      { kind: "search", query: "from:@maya has:file" },
    ]);

    expect(sections[1]?.items.map((entry) => entry.label)).toEqual([
      "from:",
      "in:",
      "from_id:",
      "in_id:",
      "mentions:me",
      "sort:",
      "has:",
      "is:thread",
      "before:",
      "after:",
      "on:",
    ]);
  });

  it("suggests the search, matching recents, people and channels as you type", () => {
    const sections = typeaheadSections("ma", SOURCE);

    expect(sections.map((entry) => entry.key)).toEqual(["query", "recent", "people"]);
    expect(sections[0]?.items[0]?.action).toEqual({ kind: "search", query: "ma" });

    const people = sections.find((entry) => entry.key === "people")?.items ?? [];

    expect(people.map((entry) => [entry.label, entry.detail, entry.action])).toEqual([
      ["Maya Okafor", null, { kind: "open", roomId: 40, userId: 2 }],
      ["Priya Raman", "New message", { kind: "open", roomId: null, userId: 6 }],
    ]);

    const channels = typeaheadSections("launch", SOURCE).find((entry) => entry.key === "channels");

    expect(channels?.items.map((entry) => entry.label)).toEqual(["launch-planning"]);
  });

  it("finishes an operator's value: people, channels, kinds and days", () => {
    const from = flattenTypeahead(typeaheadSections("launch from:ma", SOURCE));

    expect(from.map((entry) => entry.action)).toEqual([
      { kind: "complete", value: "launch from:@maya " },
      { kind: "complete", value: "launch from:@priya " },
    ]);

    const into = flattenTypeahead(typeaheadSections("in:lau", SOURCE));

    expect(into.map((entry) => entry.detail)).toEqual(["in:#launch-planning"]);

    const has = flattenTypeahead(typeaheadSections("has:f", SOURCE));

    expect(has.map((entry) => entry.detail)).toEqual(["has:file"]);

    const on = flattenTypeahead(typeaheadSections("on:", SOURCE));

    expect(on.map((entry) => entry.detail)).toEqual([
      "on:2026-10-06",
      "on:2026-10-05",
      "on:2026-09-29",
      "on:2026-09-06",
    ]);

    expect(flattenTypeahead(typeaheadSections("is:", SOURCE))[0]?.detail).toBe("is:thread");
  });

  it("names people by their first name for from:", () => {
    expect(fromHandle("Lucía Fernández")).toBe("lucia");
  });
});

describe("search shortcut", () => {
  it("takes Ctrl+Shift+F (⌘⇧F on Apple) and leaves Ctrl+F to the browser", () => {
    expect(
      searchShortcut(key({ key: "F", code: "KeyF", ctrlKey: true, shiftKey: true }), false, true),
    ).toBe("search");
    expect(
      searchShortcut(key({ key: "f", code: "KeyF", metaKey: true, shiftKey: true }), true, false),
    ).toBe("search");
    expect(searchShortcut(key({ key: "f", code: "KeyF", ctrlKey: true }), false, false)).toBeNull();
    expect(searchShortcut(key({ key: "f", code: "KeyF", metaKey: true }), true, false)).toBeNull();
    expect(
      searchShortcut(key({ key: "F", code: "KeyF", metaKey: true, shiftKey: true }), false, false),
    ).toBeNull();
  });

  it("takes / only outside a text field", () => {
    expect(searchShortcut(key({ key: "/", code: "Slash" }), false, false)).toBe("search-slash");
    expect(searchShortcut(key({ key: "/", code: "Slash" }), false, true)).toBeNull();
    expect(
      searchShortcut(key({ key: "/", code: "Slash", ctrlKey: true }), false, false),
    ).toBeNull();
  });

  it("knows which elements take typing", () => {
    const text = document.createElement("input");
    const box = document.createElement("input");
    const editor = document.createElement("div");

    box.type = "checkbox";
    editor.contentEditable = "true";

    expect(isEditable(text)).toBe(true);
    expect(isEditable(box)).toBe(false);
    expect(isEditable(document.createElement("textarea"))).toBe(true);
    expect(isEditable(document.createElement("button"))).toBe(false);
    expect(isEditable(null)).toBe(false);
  });
});

describe("format", () => {
  it("names a hit's day", () => {
    expect(hitDay(new Date(2026, 9, 6, 8, 0).toISOString(), NOW)).toBe("Today");
    expect(hitDay(new Date(2026, 9, 5, 23, 0).toISOString(), NOW)).toBe("Yesterday");
    expect(hitDay(new Date(2026, 9, 7, 9, 0).toISOString(), NOW)).toBe("Tomorrow");
    expect(hitDay(new Date(2025, 2, 1, 9, 0).toISOString(), NOW)).toMatch(/2025/);
  });

  it("counts messages, with a + while older pages remain", () => {
    expect(messageCount(14, false)).toBe("14");
    expect(messageCount(40, true)).toBe("40+");
  });

  it("labels is:thread by what it filters", () => {
    const chip: SearchChip = {
      operator: "is",
      value: "true",
      token: "is:thread",
      label: "is: true",
      removeQuery: "launch",
    };

    expect(chipLabel(chip)).toBe("is: thread");
    expect(chipLabel({ ...chip, operator: "has", value: "file", label: "has: file" })).toBe(
      "has: file",
    );
  });

  it("says what the results hold as they load, land and fail", () => {
    const base = {
      query: "launch",
      status: "ready" as const,
      error: null,
      messages: [],
      sections: [],
      hasMore: false,
      loadingMore: false,
      moreError: null,
    };

    const one = [messageFixture(1)];
    const forty = Array.from({ length: 40 }, (_, index) => messageFixture(index + 1));

    expect(resultsAnnouncement({ ...base, query: "" })).toBe("");
    expect(resultsAnnouncement({ ...base, status: "loading" })).toBe("Searching…");
    expect(resultsAnnouncement({ ...base, status: "error", error: "Down" })).toBe(
      "Search failed. Down",
    );
    expect(resultsAnnouncement(base)).toBe("No results for “launch”");
    expect(resultsAnnouncement({ ...base, messages: one })).toBe("1 message");
    expect(resultsAnnouncement({ ...base, messages: forty, hasMore: true })).toBe("40+ messages");
    expect(resultsAnnouncement({ ...base, messages: forty, loadingMore: true })).toBe(
      "Loading more messages…",
    );
    expect(resultsAnnouncement({ ...base, messages: forty, moreError: "Down" })).toBe(
      "Couldn't load more results.",
    );
  });
});

describe("stable search filters", () => {
  it("uses IDs when choosing people and channels", () => {
    expect(flattenTypeahead(typeaheadSections("from_id:ma", SOURCE))[0]?.action).toEqual({
      kind: "complete",
      value: "from_id:2 ",
    });
    expect(flattenTypeahead(typeaheadSections("in_id:lau", SOURCE))[0]?.action).toEqual({
      kind: "complete",
      value: "in_id:7 ",
    });
  });
  it("keeps filter values out of highlighted words", () => {
    expect(textWords("launch from_id:2 in_id:7 mentions:me has:mention sort:oldest")).toEqual([
      "launch",
    ]);
  });
});

it("replaces sort without changing ID filters or free text", () => {
  expect(setFilter("launch from_id:2 in_id:7 sort:newest", "sort", "oldest")).toBe(
    "launch from_id:2 in_id:7 sort:oldest",
  );
  expect(setFilter("from_id:2 in_id:7", "from_id", "")).toBe("in_id:7");
});
