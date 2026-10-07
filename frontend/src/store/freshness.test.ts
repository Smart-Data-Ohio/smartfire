import { describe, expect, it } from "vitest";
import { emptyFreshness, finishRead, membership, placeId, replay, startRead } from "./freshness.ts";

const order = (left: number, right: number) => right - left;

describe("list membership history", () => {
  it("replays additions and removals and prunes only deltas no outstanding read needs", () => {
    const first = startRead(emptyFreshness, "approved");
    const added = membership(first.freshness, "approved", [2], [3, 2]);
    const second = startRead(added, "approved");
    const removed = membership(second.freshness, "approved", [3, 2], [3]);
    const page = replay(removed, "approved", first.ticket, [2], order);
    const finished = finishRead(removed, first.ticket);

    expect(page).toEqual([3]);
    expect(finished.freshness.deltas).toHaveLength(1);
    expect(replay(finished.freshness, "approved", second.ticket, [3, 2], order)).toEqual([3]);
    expect(finishRead(finished.freshness, second.ticket).freshness.deltas).toEqual([]);
  });

  it("records membership changes independently of equal record echoes", () => {
    const read = startRead(emptyFreshness, "approved");
    const changed = membership(read.freshness, "approved", [], [3]);

    expect(replay(changed, "approved", read.ticket, [], order)).toEqual([3]);
  });

  it("prunes a list's deltas while an unrelated list read is still outstanding", () => {
    const unrelated = startRead(emptyFreshness, "other");
    const read = startRead(unrelated.freshness, "approved");
    const changed = membership(read.freshness, "approved", [], [3]);

    expect(finishRead(changed, read.ticket).freshness.deltas).toEqual([]);
  });

  it("keeps an existing member's place", () => {
    const ids = [100, 52, 51];

    expect(placeId(ids, 51, true, order)).toBe(ids);
  });
});
