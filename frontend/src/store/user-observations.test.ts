import { describe, expect, it } from "vitest";
import { userFixture } from "../api/testing.ts";
import { nextObservation, observationOf, observeObject } from "../lib/request-observation.ts";
import { mergeUserList } from "./ordering.ts";

const USER = 7;

describe("immutable User observations within each row revision", () => {
  it("lets a newer revision expire after a later request observed an older revision", () => {
    const beforeExpiry = nextObservation();
    const afterExpiry = nextObservation();
    const olderRowRead = nextObservation();
    const old = observeObject(userFixture(USER), olderRowRead);

    const current = {
      ...userFixture(USER),
      updatedAt: "2026-10-07T10:00:00.000000Z",
    };

    const active = observeObject(
      { ...current, customStatus: { emoji: "🌴", text: "Away", expiresAt: null } },
      beforeExpiry,
    );

    const expired = observeObject({ ...current, customStatus: null }, afterExpiry);

    const first = mergeUserList({}, [old]);
    const second = mergeUserList(first, [active]);
    const third = mergeUserList(second, [expired]);

    expect(second[USER]?.customStatus?.text).toBe("Away");
    expect(third[USER]?.customStatus).toBeNull();
    expect(third[USER]?.updatedAt).toBe(current.updatedAt);
  });

  it("advances identical observations without changing a retained earlier snapshot", () => {
    const earlier = nextObservation();
    const middle = nextObservation();
    const latest = nextObservation();
    const first = mergeUserList({}, [observeObject(userFixture(USER), earlier)]);
    const changed = observeObject({ ...userFixture(USER), hasAvatar: true }, middle);
    const refreshed = mergeUserList(first, [observeObject(userFixture(USER), latest)]);
    const earlierBranch = mergeUserList(first, [changed]);
    const latestBranch = mergeUserList(refreshed, [changed]);

    expect(first[USER]).not.toBe(refreshed[USER]);
    expect(observationOf(first[USER] ?? {})).toBe(earlier);
    expect(earlierBranch[USER]?.hasAvatar).toBe(true);
    expect(latestBranch[USER]?.hasAvatar).toBe(false);
  });
});
