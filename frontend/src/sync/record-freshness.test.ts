import { afterEach, describe, expect, it } from "vitest";
import { sidebarFixture, userFixture } from "../api/testing.ts";
import { mutations, store } from "../store/store.ts";
import { recordFreshness, touchChanged } from "./record-freshness.ts";

const ADA = 7;

describe("record freshness", () => {
  afterEach(() => mutations.reset());

  it("drops a read that overlaps a ban, in either reply order, and frees the next one", async () => {
    const freshness = recordFreshness();
    const settle = freshness.startWrite(ADA);

    // Navigating back starts a read while the ban is in flight: it may see them still active.
    const overlapping = freshness.startRead();

    expect(freshness.fresh(overlapping, ADA)).toBe(false);
    settle();
    expect(freshness.fresh(overlapping, ADA)).toBe(false);

    await freshness.settled(ADA);

    const after = freshness.startRead();

    expect(freshness.fresh(after, ADA)).toBe(true);
  });

  it("drops a read that started before a write, even once the write settles", () => {
    const freshness = recordFreshness();
    const read = freshness.startRead();

    freshness.startWrite(ADA)();

    expect(freshness.fresh(read, ADA)).toBe(false);
    expect(freshness.fresh(read, ADA + 1)).toBe(true);
  });

  it("waits for every write on a record before it counts as settled", async () => {
    const freshness = recordFreshness();
    const first = freshness.startWrite(ADA);
    const second = freshness.startWrite(ADA);
    let settled = false;

    void freshness.settled(ADA).then(() => {
      settled = true;
    });
    first();
    first();
    await Promise.resolve();
    expect(settled).toBe(false);

    second();
    await Promise.resolve();
    expect(settled).toBe(true);
  });

  it("drops a held read once a sync resync changes the user in the store", () => {
    const freshness = recordFreshness();

    const unsubscribe = store.subscribe((state, previous) =>
      touchChanged(freshness, previous.users, state.users),
    );

    mutations.mergeUsers([userFixture(ADA, "Ada Lovelace")]);

    const held = freshness.startRead();

    // The same user again changes nothing, so the held read can still land.
    mutations.loadSidebar(sidebarFixture([]));
    expect(freshness.fresh(held, ADA)).toBe(true);

    mutations.loadSidebar({
      ...sidebarFixture([]),
      users: [{ ...userFixture(ADA, "Ada Lovelace"), status: "banned" }],
    });
    expect(freshness.fresh(held, ADA)).toBe(false);
    unsubscribe();
  });
});
