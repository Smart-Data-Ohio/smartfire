import { afterEach, describe, expect, it } from "vitest";
import { sidebarFixture, userFixture } from "../api/testing.ts";
import type { User } from "./model.ts";
import { mutations, sidebarRowClock, store } from "./store.ts";

const ADA = 7;

/** Ada as the server had her at `minute` past ten. */
function ada(minute: number, change: Partial<User> = {}): User {
  const at = `2026-10-07T10:${String(minute).padStart(2, "0")}:00.000000Z`;

  return { ...userFixture(ADA, "Ada Lovelace"), updatedAt: at, ...change };
}

const held = () => store.getState().users[ADA];

describe("a user lands only as its newest copy", () => {
  afterEach(() => mutations.reset());

  it("keeps the newer of two overlapping directory loads, whichever arrives first", () => {
    mutations.mergeUsers([ada(1)]);
    mutations.mergeUsers([ada(2, { name: "Ada King" })]);
    expect(held()?.name).toBe("Ada King");

    mutations.reset();
    mutations.mergeUsers([ada(2, { name: "Ada King" })]);
    mutations.mergeUsers([ada(1)]);
    expect(held()?.name).toBe("Ada King");
  });

  it("drops a held page load once a resync brought a later ban", () => {
    mutations.mergeUsers([ada(1)]);
    mutations.loadSidebar(
      { ...sidebarFixture([]), users: [ada(3, { status: "banned" })] },
      sidebarRowClock(),
    );

    // The page load started before the ban and answers her as active.
    mutations.mergeUsers([ada(2)]);

    expect(held()?.status).toBe("banned");
  });

  it("drops a held banned copy after an unban that left her as she was (ABA)", () => {
    mutations.mergeUsers([ada(1)]);

    // Another session bans her (the held read captures this) and unbans her; the resync's copy
    // looks just like the one held, but it's later.
    const captured = ada(2, { status: "banned" });

    mutations.loadSidebar({ ...sidebarFixture([]), users: [ada(3)] }, sidebarRowClock());
    mutations.mergeUsers([captured]);

    expect(held()?.status).toBe("active");
    expect(held()?.updatedAt).toBe(ada(3).updatedAt);
  });

  it("lands a tie, so a refresh of other tables' fields still shows", () => {
    mutations.mergeUsers([ada(1)]);
    mutations.mergeUsers([
      ada(1, { customStatus: { emoji: "🌴", text: "Away", expiresAt: null } }),
    ]);

    expect(held()?.customStatus?.text).toBe("Away");
  });

  it("leaves the table untouched when nothing is newer", () => {
    mutations.mergeUsers([ada(2)]);

    const before = store.getState().users;

    mutations.mergeUsers([ada(1)]);

    expect(store.getState().users).toBe(before);
  });
});
