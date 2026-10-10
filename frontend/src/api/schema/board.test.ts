import { describe, expect, it } from "@effect/vitest";
import { Schema } from "effect";
import { boardListing } from "../../test/board-fixtures.ts";
import {
  BoardListing,
  BoardPostForm,
  BoardTagCatalog,
  ReorderBoardTags,
  SaveBoardTag,
  UpdateBoardTagPolicy,
} from "./board.ts";

describe("board catalog contract", () => {
  it("preserves catalog labels, emoji and required/default policy", () => {
    const form = {
      ownerCandidates: [],
      tagSuggestions: ["legacy"],
      users: [],
      tags: [{ id: 12, name: "Bug Report", emoji: "🐛", position: 0 }],
      tagsRequired: true,
      defaultBoardTagId: 12,
    };

    expect(Schema.decodeUnknownSync(BoardPostForm)(form)).toEqual(form);
  });
  it("decodes catalog, listing and mutation contracts", () => {
    const catalog = {
      roomId: 900,
      tags: [{ id: 12, name: "Bug Report", emoji: null, position: 0 }],
      tagsRequired: false,
      defaultBoardTagId: null,
    };

    const decode = Schema.decodeUnknownSync;
    expect(decode(BoardTagCatalog)(catalog)).toEqual(catalog);
    expect(decode(BoardListing)({ ...boardListing(), ...catalog }).tags).toEqual(catalog.tags);
    expect(decode(SaveBoardTag)({ name: "Bug Report", emoji: "🐛" })).toEqual({
      name: "Bug Report",
      emoji: "🐛",
    });
    expect(decode(ReorderBoardTags)({ tagIds: [12, 13] })).toEqual({ tagIds: [12, 13] });
    expect(decode(UpdateBoardTagPolicy)({ tagsRequired: true, defaultBoardTagId: 12 })).toEqual({
      tagsRequired: true,
      defaultBoardTagId: 12,
    });
  });
  it("rejects malformed tag and policy fields", () => {
    const decode = Schema.decodeUnknownSync;
    expect(() =>
      decode(BoardTagCatalog)({
        roomId: 900,
        tags: [],
        tagsRequired: "yes",
        defaultBoardTagId: null,
      }),
    ).toThrow();
    expect(() => decode(ReorderBoardTags)({ tagIds: [1.5] })).toThrow();
    expect(() => decode(SaveBoardTag)({ name: "Bug", emoji: 42 })).toThrow();
    expect(() =>
      decode(UpdateBoardTagPolicy)({ tagsRequired: true, defaultBoardTagId: "12" }),
    ).toThrow();
  });
});
