import { describe, expect, it } from "@effect/vitest";
import { Schema } from "effect";
import { boardDetail, boardListing } from "../../test/board-fixtures.ts";
import { userFixture } from "../testing.ts";
import { BoardListing, BoardPostForm, BoardStatusFilter, CreateBoardPost } from "./board.ts";
import { SyncEvent } from "./sync.ts";
import { ThreadDetail } from "./thread.ts";
import { UpdateWork, WorkFacts } from "./work.ts";

const decode = Schema.decodeUnknownSync;

describe("S6 board schemas", () => {
  it("decodes listings, empty boards, form candidates and metadata", () => {
    const listing = decode(BoardListing)(boardListing());
    expect(listing.posts[0]?.thread.parentMessageId).toBeNull();
    expect(listing.digest?.date).toBe("2026-10-07");
    expect(listing.ownerOptions[0]?.userId).toBe(7);
    expect(
      decode(BoardListing)({ ...boardListing([]), anyPosts: false, digest: null }).posts,
    ).toEqual([]);

    const form = decode(BoardPostForm)({
      ownerCandidates: [{ userId: 7, provider: null, description: null }],
      tagSuggestions: ["api"],
      users: [userFixture(7)],
    });

    expect(form.tagSuggestions).toEqual(["api"]);
  });
  it("requires tags on facts and reads them on thread detail and sync", () => {
    const detail = boardDetail();
    expect(decode(ThreadDetail)(detail).thread.work?.tags).toEqual([]);
    expect(
      decode(SyncEvent)({ seq: 1, topic: "room:900", type: "thread.created", data: detail.thread })
        .type,
    ).toBe("thread.created");
    const { tags: _tags, ...oldFacts } = detail.thread.work ?? {};
    expect(() => decode(WorkFacts)(oldFacts)).toThrow();
  });
  it("preserves absent fields, explicit nulls and empty tag sets on updates", () => {
    expect(decode(UpdateWork)({})).toEqual({});
    expect(decode(UpdateWork)({ status: null, ownerId: null, tags: [] })).toEqual({
      status: null,
      ownerId: null,
      tags: [],
    });
    expect(() => decode(UpdateWork)({ tags: null })).toThrow();
  });
  it("accepts optional briefs and strictly checks statuses and wire shapes", () => {
    const body = { name: "A post", status: "planned", ownerId: null, tags: ["api"], message: null };
    expect(decode(CreateBoardPost)(body)).toEqual(body);
    expect(() => decode(CreateBoardPost)({ ...body, status: "unknown" })).toThrow();
    expect(() => decode(BoardStatusFilter)("working")).toThrow();
    expect(() =>
      decode(BoardListing)({ ...boardListing(), ownerOptions: [{ userId: "7", agent: false }] }),
    ).toThrow();
  });
});
