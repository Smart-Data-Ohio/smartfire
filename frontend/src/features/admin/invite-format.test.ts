import { describe, expect, it } from "vitest";
import type { WorkspaceInvite } from "../../gen/WorkspaceInvite.ts";
import {
  emptyLine,
  expiresLabel,
  filterInvites,
  inviteState,
  linkTerms,
  maxUsesOf,
  revokeConfirmation,
  shortTime,
  timeLeft,
  usesLabel,
  withMade,
} from "./invite-format.ts";

const NOW = Date.parse("2026-10-11T12:00:00.000Z");

const at = (offsetMs: number) => new Date(NOW + offsetMs).toISOString();

function invite(change: Partial<WorkspaceInvite> = {}): WorkspaceInvite {
  return {
    id: 1,
    creator: { id: 1, name: "Riel" },
    expiresAt: null,
    maxUses: null,
    uses: 0,
    revokedAt: null,
    createdAt: at(-60_000),
    updatedAt: at(-60_000),
    state: "active",
    ...change,
  };
}

describe("invite format", () => {
  it("reads the use limit from the select", () => {
    expect(maxUsesOf("")).toBeNull();
    expect(maxUsesOf("25")).toBe(25);
  });

  it("says an active invite whose time ran out on screen is expired", () => {
    expect(inviteState(invite({ expiresAt: at(1000) }), NOW)).toBe("active");
    expect(inviteState(invite({ expiresAt: at(-1000) }), NOW)).toBe("expired");
    expect(inviteState(invite({ expiresAt: at(-1000), state: "revoked" }), NOW)).toBe("revoked");
  });

  it("keeps invites made on the page that the list answered without", () => {
    const listed = [invite({ id: 2 }), invite({ id: 1 })];
    const revoked = invite({ id: 3, state: "revoked" });

    expect(withMade([invite({ id: 4 }), invite({ id: 3 })], listed).map(({ id }) => id)).toEqual([
      4, 3, 2, 1,
    ]);
    // The list's own answer wins for one it already has.
    expect(withMade([invite({ id: 3 })], [revoked, ...listed])).toEqual([revoked, ...listed]);
  });

  it("filters by the state at the moment", () => {
    const list = [
      invite({ id: 1 }),
      invite({ id: 2, expiresAt: at(-1) }),
      invite({ id: 3, state: "exhausted" }),
    ];

    expect(filterInvites(list, "all", NOW).map((each) => each.id)).toEqual([1, 2, 3]);
    expect(filterInvites(list, "expired", NOW).map((each) => each.id)).toEqual([2]);
    expect(filterInvites(list, "exhausted", NOW).map((each) => each.id)).toEqual([3]);
  });

  it("says uses against the limit", () => {
    expect(usesLabel(invite({ uses: 3, maxUses: 10 }))).toBe("3 of 10");
    expect(usesLabel(invite({ uses: 3 }))).toBe("3, no limit");
  });

  it("says how long is left, or when it ran out", () => {
    expect(timeLeft(at(29 * 60_000 + 1), NOW)).toBe("in 30 minutes");
    expect(timeLeft(at(10_000), NOW)).toBe("in 1 minute");
    expect(timeLeft(at(24 * 3_600_000 + 30_000), NOW)).toBe("in 24 hours");
    expect(timeLeft(at(7 * 86_400_000 - 2 * 3_600_000), NOW)).toBe("in 7 days");

    expect(expiresLabel(invite(), NOW, () => "then")).toBe("Never");
    expect(expiresLabel(invite({ expiresAt: at(6 * 3_600_000) }), NOW, () => "then")).toBe(
      "in 6 hours",
    );
    expect(expiresLabel(invite({ expiresAt: at(-1) }), NOW, () => "Oct 11, 7:59 AM")).toBe(
      "Oct 11, 7:59 AM",
    );
  });

  it("says a new link's terms", () => {
    expect(linkTerms("7d", null)).toBe(
      "Expires in 7 days. No limit on how many people can join with it.",
    );
    expect(linkTerms("never", 1)).toBe("Never expires. Works for 1 person.");
    expect(linkTerms("30m", 25)).toBe("Expires in 30 minutes. Works for 25 people.");
  });

  it("says who stays when an invite is revoked", () => {
    expect(revokeConfirmation(invite())).toContain("No one has joined with it yet.");
    expect(revokeConfirmation(invite({ uses: 1 }))).toContain(
      "The person who joined with it stays.",
    );
    expect(revokeConfirmation(invite({ uses: 4 }))).toContain(
      "The 4 people who joined with it stay.",
    );
  });

  it("words the empty list for each filter", () => {
    expect(emptyLine("all")).toBe("No invites yet. Create one above to share a private link.");
    expect(emptyLine("exhausted")).toBe("No used up invites.");
  });
});

describe("short times", () => {
  it("leave out this year, and keep another", () => {
    expect(shortTime("2026-10-09T15:00:00.000Z", NOW, "en-US")).not.toContain("2026");
    expect(shortTime("2025-12-30T15:00:00.000Z", NOW, "en-US")).toContain("2025");
  });
});
