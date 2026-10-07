import { describe, expect, it } from "vitest";
import type { IntegrationsHealth } from "../../gen/IntegrationsHealth.ts";
import type { Person } from "../../gen/Person.ts";
import {
  filtering,
  filterValue,
  groupPeople,
  healthFacts,
  NO_FILTERS,
  visibleSections,
} from "./admin-format.ts";

function person(id: number, role: Person["role"]): Person {
  return {
    id,
    name: `Person ${id}`,
    avatarUrl: `/users/${id}/avatar`,
    role,
    banned: false,
    you: false,
    twoFactorEnabled: false,
    emailAddress: null,
    googleIdentityEmail: null,
    offerGoogleEmailLink: false,
  };
}

const HEALTH: IntegrationsHealth = {
  github: {
    workspaceToken: false,
    appConfigured: true,
    webhookSecret: true,
    connected: 3,
    appTokens: 1,
    deliveries24h: 12,
    disconnected: [],
    lastErrors: [],
    fetchErrors: [],
  },
  google: {
    configured: true,
    connected: 2,
    pushEnabled: true,
    pushChannels: 4,
    disconnected: [],
    entryErrors: [],
    expiring: [{ userId: 7, expiresAt: null, error: "gone" }],
  },
  fizzy: { configured: false, note: "No Fizzy integration is configured in this workspace." },
  agentDelivery: { pending: 0, failed24h: 1, recentErrors: [] },
  email: { enabled: true, roomsWithAddresses: 1 },
};

describe("the admin sections", () => {
  it("show members only the workspace and its people", () => {
    expect(visibleSections(false).map((section) => section.key)).toEqual(["workspace", "people"]);
    expect(visibleSections(true)).toHaveLength(6);
  });
});

describe("the people list", () => {
  it("puts administrators first, keeping each group's order", () => {
    const { administrators, members } = groupPeople([
      person(1, "member"),
      person(2, "administrator"),
      person(3, "member"),
    ]);

    expect(administrators.map((each) => each.id)).toEqual([2]);
    expect(members.map((each) => each.id)).toEqual([1, 3]);
  });
});

describe("the audit log filters", () => {
  it("treat blanks as unset", () => {
    expect(filterValue("  ")).toBeNull();
    expect(filterValue(" ada ")).toBe("ada");
    expect(filtering(NO_FILTERS)).toBe(false);
    expect(filtering({ ...NO_FILTERS, actor: "ada" })).toBe(true);
  });
});

describe("integration health", () => {
  it("reads as the classic page does", () => {
    const facts = healthFacts(HEALTH);

    expect(facts.github).toContainEqual([
      "Workspace token",
      "Not set — private cards need per-user accounts",
    ]);
    expect(facts.github).toContainEqual(["Connected accounts", "3 (1 App, 2 PAT)"]);
    expect(facts.google).toContainEqual(["Push channels", "4 watching"]);
    expect(facts.expiring).toEqual([{ subject: "user 7", detail: "expires unknown — gone" }]);
    expect(facts.fizzy).toBe("No Fizzy integration is configured in this workspace.");
    expect(facts.email).toBe("1 room with a forward-to address.");
  });
});
