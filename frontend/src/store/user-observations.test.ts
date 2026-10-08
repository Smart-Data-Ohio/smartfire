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

  it("keeps an unban over a ban read earlier at the same revision (a repeated server clock)", () => {
    const at = "2026-10-07T10:00:00.000000Z";
    const banned = { ...userFixture(USER), status: "banned" as const, updatedAt: at };
    const banStarted = nextObservation();
    const readStarted = nextObservation();
    const unbanStarted = nextObservation();
    const heldBanned = mergeUserList({}, [observeObject({ ...banned }, banStarted)]);
    const staleRead = observeObject({ ...banned }, readStarted);
    const unbanned = observeObject({ ...banned, status: "active" as const }, unbanStarted);
    const afterUnban = mergeUserList(heldBanned, [unbanned]);
    const afterRead = mergeUserList(afterUnban, [staleRead]);

    expect(afterUnban[USER]?.status).toBe("active");
    expect(afterRead[USER]?.status).toBe("active");
    expect(afterRead[USER]?.updatedAt).toBe(at);

    // The other way round: a ban answered after an earlier read of the active row still holds.
    const rebanStarted = nextObservation();
    const banReply = observeObject({ ...banned }, rebanStarted);
    const lateActive = observeObject({ ...banned, status: "active" as const }, unbanStarted);

    expect(mergeUserList(mergeUserList(afterRead, [banReply]), [lateActive])[USER]?.status).toBe(
      "banned",
    );
  });

  it("lets a later event land a different row at the same revision", () => {
    const at = "2026-10-07T10:00:00.000000Z";
    const read = observeObject({ ...userFixture(USER), updatedAt: at }, nextObservation());
    const held = mergeUserList({}, [read]);
    const event = { ...userFixture(USER), status: "banned" as const, updatedAt: at };

    expect(mergeUserList(held, [event])[USER]?.status).toBe("banned");
  });
});
