import { describe, expect, it } from "vitest";
import {
  emptyFreshness,
  finishRead,
  membership,
  observeMembership,
  placeId,
  reloading,
  replay,
  retireReads,
  startRead,
} from "./freshness.ts";

const order = (left: number, right: number) => right - left;

describe("list membership history", () => {
  it("replays additions and removals until their list's current read finishes", () => {
    const first = startRead(emptyFreshness, "approved");
    const added = membership(first.freshness, "approved", [2], [3, 2]);
    const removed = membership(added, "approved", [3, 2], [3]);

    expect(replay(removed, "approved", first.ticket, [2], order)).toEqual([3]);
    expect(finishRead(removed, first.ticket).freshness.deltas).toEqual([]);
  });

  it("retires superseded reloads while preserving another list's outstanding read", () => {
    const other = startRead(emptyFreshness, "other");
    const first = startRead(other.freshness, "approved");
    const added = membership(first.freshness, "approved", [], [3]);
    const second = startRead(added, "approved");
    const removed = membership(second.freshness, "approved", [3], []);
    const finished = finishRead(removed, first.ticket).freshness;

    expect(Object.keys(finished.reads)).toEqual([String(other.ticket), String(second.ticket)]);
    expect(finished.deltas).toHaveLength(1);
    expect(replay(finished, "approved", second.ticket, [3], order)).toEqual([]);
    expect(retireReads(finished, "approved").freshness).toMatchObject({
      reads: { [other.ticket]: { list: "other" } },
      deltas: [],
    });
  });

  it("lets a next page leave an outstanding reload in place", () => {
    const reload = startRead(emptyFreshness, "approved");
    const more = startRead(reload.freshness, "approved", false);

    expect(Object.keys(more.freshness.reads)).toEqual([String(reload.ticket), String(more.ticket)]);
    expect(reloading(more.freshness, "approved")).toBe(true);
    expect(reloading(finishRead(more.freshness, reload.ticket).freshness, "approved")).toBe(false);
    expect(Object.keys(startRead(more.freshness, "approved").freshness.reads)).toHaveLength(1);
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

  it("keeps only the newest outstanding read and observations for each list and id", () => {
    let state = emptyFreshness;

    for (let index = 0; index < 100; index += 1) {
      state = startRead(state, "approved").freshness;

      for (let echo = 0; echo < 100; echo += 1) {
        state = observeMembership(state, "approved", 3, echo % 2 === 0);
      }

      expect(Object.values(state.reads)).toHaveLength(1);
      expect(state.deltas).toHaveLength(1);
    }

    const ticket = Number(Object.keys(state.reads)[0]);

    expect(replay(state, "approved", ticket, [3], order)).toEqual([]);
    expect(finishRead(state, ticket).freshness.deltas).toEqual([]);
  });

  it("keeps an existing member's place", () => {
    const ids = [100, 52, 51];

    expect(placeId(ids, 51, true, order)).toBe(ids);
  });
});
