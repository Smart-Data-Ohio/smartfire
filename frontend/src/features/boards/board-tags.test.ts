import { describe, expect, it } from "vitest";
import type { BoardTag } from "../../gen/BoardTag.ts";
import {
  addedDefault,
  deleteConsequences,
  EMPTY_CATALOG,
  initialSelection,
  keptSelection,
  moveId,
  orderedPostTags,
  postTagNames,
  postTagsProblem,
  REQUIRED_TAG_MESSAGE,
  tagNameProblem,
  toggleSelection,
} from "./board-tags.ts";

const bug: BoardTag = { id: 1, name: "Bug report", emoji: "🐞", position: 0 };

const idea: BoardTag = { id: 2, name: "Idea", emoji: null, position: 1 };

const tags = [bug, idea];

const catalog = (over: Partial<typeof EMPTY_CATALOG> = {}) => ({ ...EMPTY_CATALOG, tags, ...over });

describe("board tag selection", () => {
  it("starts with the default picked and keeps picks in catalog order", () => {
    expect(initialSelection(catalog())).toEqual([]);
    expect(initialSelection(catalog({ defaultBoardTagId: 2 }))).toEqual([2]);
    expect(initialSelection(catalog({ defaultBoardTagId: 99 }))).toEqual([]);
    expect(toggleSelection([2], 1, tags)).toEqual([1, 2]);
    expect(toggleSelection([1, 2], 1, tags)).toEqual([2]);
    expect(keptSelection([1, 2, 3], [idea])).toEqual([2]);
  });

  it("sends picked labels first and maps free text onto catalog labels", () => {
    expect(postTagNames(tags, [2], ["api", "bug REPORT", "api"])).toEqual([
      "Idea",
      "api",
      "Bug report",
    ]);
    expect(postTagNames(tags, [1], ["bug report"])).toEqual(["Bug report"]);
  });

  it("matches names under the server's full case folding", () => {
    const street: BoardTag = { id: 3, name: "Straße", emoji: null, position: 2 };
    const sigma: BoardTag = { id: 4, name: "ΟΔΟΣ", emoji: null, position: 3 };

    expect(postTagNames([street, sigma], [], ["STRASSE", "οδος"])).toEqual(["Straße", "ΟΔΟΣ"]);
    expect(tagNameProblem([street], "strasse")).toBe("“Straße” is already a tag here.");
    expect(tagNameProblem([sigma], "οδος")).toBe("“ΟΔΟΣ” is already a tag here.");
    expect(tagNameProblem([{ ...street, name: "ı" }], "I")).toBeUndefined();
  });
});

describe("postTagsProblem", () => {
  it("asks for a curated tag on a board that requires one and has no default", () => {
    expect(postTagsProblem(catalog({ tagsRequired: true }), ["api"])).toEqual({
      field: "catalog",
      message: REQUIRED_TAG_MESSAGE,
    });
    expect(postTagsProblem(catalog({ tagsRequired: true }), ["idea"])).toBeUndefined();
    expect(postTagsProblem({ ...EMPTY_CATALOG, tagsRequired: true }, [])).toBeUndefined();
  });

  it("lets the default stand in, as the server adds it, and counts it toward five", () => {
    const required = catalog({ tagsRequired: true, defaultBoardTagId: 2 });

    expect(postTagsProblem(required, [])).toBeUndefined();
    expect(postTagsProblem(required, ["api"])).toBeUndefined();
    expect(addedDefault(required, ["api"])).toBe(idea);
    expect(addedDefault(required, ["Bug report"])).toBeUndefined();
    expect(addedDefault(catalog({ defaultBoardTagId: 2 }), [])).toBeUndefined();
    expect(postTagsProblem(required, ["a", "b", "c", "d", "e"])).toEqual({
      field: "tags",
      message: "Use up to 4 tags: Idea is added as well.",
    });
  });

  it("counts every tag toward five and checks only free text against the grammar", () => {
    expect(postTagsProblem(catalog(), ["Bug report", "Idea", "a", "b", "c", "d"])).toEqual({
      field: "tags",
      message: "Use up to 5 tags in all.",
    });
    expect(postTagsProblem(catalog(), ["Bug report", "ok-tag"])).toBeUndefined();
    expect(postTagsProblem(catalog(), ["Not OK"])?.field).toBe("tags");
  });
});

describe("tagNameProblem", () => {
  it("checks names the way the server will", () => {
    expect(tagNameProblem(tags, "  ")).toBe("Give the tag a name.");
    expect(tagNameProblem(tags, "x".repeat(21))).toBe("Keep tag names to 20 characters.");
    expect(tagNameProblem(tags, "IDEA")).toBe("“Idea” is already a tag here.");
    expect(tagNameProblem(tags, "idea", 2)).toBeUndefined();
    expect(tagNameProblem(tags, "Question")).toBeUndefined();

    const full = Array.from({ length: 20 }, (_, index) => ({
      ...idea,
      id: index,
      name: `T${index}`,
    }));

    expect(tagNameProblem(full, "Another")).toBe("A board holds up to 20 tags.");
    expect(tagNameProblem(full, "Renamed", 3)).toBeUndefined();
  });
});

describe("moveId", () => {
  it("moves within the list and clamps the target", () => {
    expect(moveId([1, 2, 3], 0, 2)).toEqual([2, 3, 1]);
    expect(moveId([1, 2, 3], 2, -5)).toEqual([3, 1, 2]);
    expect(moveId([1, 2, 3], 1, Number.POSITIVE_INFINITY)).toEqual([1, 3, 2]);
    expect(moveId([1, 2, 3], 4, 0)).toEqual([1, 2, 3]);
  });
});

describe("deleteConsequences", () => {
  it("names the posts, the default and a requirement nothing could satisfy", () => {
    expect(deleteConsequences(catalog(), idea, 3)).toEqual({
      lines: ["3 posts lose it."],
      liftsRequirement: false,
    });
    expect(deleteConsequences(catalog({ defaultBoardTagId: 2 }), idea, 1).lines).toEqual([
      "1 post loses it.",
      "New posts stop starting with it.",
    ]);
    expect(deleteConsequences(catalog(), idea, undefined).lines).toEqual([
      "Every post that has it loses it.",
    ]);
    expect(
      deleteConsequences({ ...EMPTY_CATALOG, tags: [idea], tagsRequired: true }, idea, 0),
    ).toEqual({
      lines: ["No posts have it yet.", "It's the last tag, so new posts stop requiring one."],
      liftsRequirement: true,
    });
  });
});

describe("orderedPostTags", () => {
  it("puts curated tags first in catalog order", () => {
    expect(orderedPostTags(tags, ["api", "Idea", "Bug report"])).toEqual([
      { name: "Bug report", tag: bug },
      { name: "Idea", tag: idea },
      { name: "api", tag: undefined },
    ]);
  });
});
