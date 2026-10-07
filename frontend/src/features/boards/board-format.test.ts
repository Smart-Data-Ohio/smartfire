import { describe, expect, it } from "vitest";
import { userFixture } from "../../api/testing.ts";
import type { User } from "../../gen/User.ts";
import type { WorkFacts } from "../../gen/WorkFacts.ts";
import {
  activeFilterCount,
  boardQuery,
  boardSearch,
  digestDate,
  ownerLabel,
  parseTags,
  safeHttpsUrl,
  tagsProblem,
} from "./board-format.ts";

const MAYA: User = { ...userFixture(2), name: "Maya Okafor" };

function facts(owner: User | null, ownerActive: boolean): WorkFacts {
  return {
    status: "planned",
    owner,
    ownerActive,
    runUrl: null,
    resultUpdatedAt: null,
    links: [],
    tags: [],
  };
}

describe("board queries", () => {
  it("fills in the classic defaults and lists every status in the column view", () => {
    expect(boardQuery({})).toEqual({ view: "list", status: "open", owner: "anyone", tag: "" });
    expect(boardQuery({ view: "board", status: "done" })).toEqual({
      view: "board",
      status: "all",
      owner: "anyone",
      tag: "",
    });
  });

  it("leaves defaults out of the URL", () => {
    expect(boardSearch(boardQuery({}))).toEqual({});
    expect(boardSearch({ view: "list", status: "done", owner: "7", tag: "bug" })).toEqual({
      status: "done",
      owner: "7",
      tag: "bug",
    });
    expect(boardSearch({ view: "board", status: "all", owner: "me", tag: "" })).toEqual({
      view: "board",
      owner: "me",
    });
  });

  it("counts the status filter only in the list", () => {
    expect(activeFilterCount(boardQuery({ status: "done", tag: "api" }))).toBe(2);
    expect(activeFilterCount(boardQuery({ view: "board", status: "done", tag: "api" }))).toBe(1);
  });
});

describe("tags", () => {
  it("normalises like the server: stripped, lower-cased, blanks and repeats dropped", () => {
    expect(parseTags(" Bug, api,, BUG ,infra ")).toEqual(["bug", "api", "infra"]);
  });

  it("says what the server would refuse", () => {
    expect(tagsProblem(["a", "b", "c", "d", "e"])).toBeUndefined();
    expect(tagsProblem(["a", "b", "c", "d", "e", "f"])).toBe("Use up to 5 tags.");
    expect(tagsProblem(["x".repeat(31)])).toBe("Keep each tag to 30 characters.");
    expect(tagsProblem(["-api"])).toMatch(/letters, numbers and dashes/);
  });
});

describe("words", () => {
  it("names the owner, or says they can't act on it", () => {
    expect(ownerLabel(facts(null, false))).toBe("Unassigned");
    expect(ownerLabel(facts(MAYA, true))).toBe("Maya Okafor");
    expect(ownerLabel(facts(MAYA, false))).toBe("Owner unavailable (Maya Okafor)");
  });

  it("reads the digest's UTC day", () => {
    expect(digestDate("2026-10-07")).toBe(
      new Intl.DateTimeFormat(undefined, {
        month: "long",
        day: "numeric",
        year: "numeric",
        timeZone: "UTC",
      }).format(Date.UTC(2026, 9, 7)),
    );
  });

  it("only links https run URLs", () => {
    expect(safeHttpsUrl("https://ci.example.com/1")).toBe("https://ci.example.com/1");
    expect(safeHttpsUrl("javascript:alert(1)")).toBeNull();
    expect(safeHttpsUrl(null)).toBeNull();
  });
});
