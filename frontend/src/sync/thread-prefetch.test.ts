import { describe, expect, it } from "vitest";
import { membershipFixture, threadFixture } from "../features/threads/test-fixtures.ts";
import type { ThreadList } from "../gen/ThreadList.ts";
import { initialState, type State } from "../store/state.ts";
import { mergeThreadSummaries } from "./thread-prefetch.ts";

describe("mergeThreadSummaries", () => {
  const list: ThreadList = {
    threads: [
      {
        thread: threadFixture(1),
        membership: membershipFixture(1, { unreadAt: "2026-10-06T09:00:00.000Z" }),
      },
      { thread: threadFixture(2), membership: null },
    ],
    users: [],
  };

  it("adds unknown threads and memberships, null meaning not a member", () => {
    const next = mergeThreadSummaries(initialState, list);

    expect(Object.keys(next.threads)).toEqual(["1", "2"]);
    expect(next.threadMemberships[1]?.unreadAt).toBe("2026-10-06T09:00:00.000Z");
    expect(next.threadMemberships[2]).toBeNull();
    expect(next.roomThreads).toBe(initialState.roomThreads);
  });

  it("keeps what the store already holds, which is fresher", () => {
    const held: State = {
      ...initialState,
      threads: { 1: threadFixture(1, { name: "Renamed" }) },
      threadMemberships: { 1: membershipFixture(1, { involvement: "everything" }) },
    };

    const next = mergeThreadSummaries(held, list);

    expect(next.threads[1]?.name).toBe("Renamed");
    expect(next.threadMemberships[1]?.involvement).toBe("everything");
    expect(next.threadMemberships[2]).toBeNull();
  });

  it("returns the same state when there's nothing new", () => {
    const once = mergeThreadSummaries(initialState, list);

    expect(mergeThreadSummaries(once, list)).toBe(once);
  });
});
