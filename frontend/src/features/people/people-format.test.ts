import { describe, expect, it } from "vitest";
import type { PersonProfile } from "../../gen/PersonProfile.ts";
import type { User } from "../../gen/User.ts";
import {
  botPage,
  directoryBadge,
  landBan,
  newerUser,
  presenceOf,
  selectionPlan,
  toggleSelection,
} from "./people-format.ts";

function user(id: number, role: User["role"], agent = false): User {
  return {
    id,
    name: `User ${id}`,
    role,
    status: "active",
    bio: null,
    avatarUrl: `/users/${id}/avatar`,
    hasAvatar: false,
    customStatus: null,
    avatarIcon: null,
    agent: agent ? { agentId: id, kind: "workspace", status: "idle", suspended: false } : null,
    createdAt: "2026-01-01T00:00:00.000Z",
    updatedAt: "2026-01-01T00:00:00.000000Z",
  };
}

const BOTS = new Set([90, 91]);

const isBot = (id: number) => BOTS.has(id);

describe("selectionPlan", () => {
  it("counts everyone for Message and only people for Start huddle", () => {
    const plan = selectionPlan([1, 2, 90], isBot);

    expect(plan.messageLabel).toBe("Message (3)");
    expect(plan.huddleLabel).toBe("Start huddle (2)");
    expect(plan.messageDisabled).toBe(false);
    expect(plan.huddleDisabled).toBe(false);
    expect(plan.note).toBe("1 agent stays in the DM but won't be rung.");
  });

  it("says agents can't join a huddle of only agents", () => {
    const plan = selectionPlan([90, 91], isBot);

    expect(plan.huddleDisabled).toBe(true);
    expect(plan.note).toBe("Agents can't join huddles.");
  });

  it("pluralises the agents left out", () => {
    expect(selectionPlan([1, 90, 91], isBot).note).toBe(
      "2 agents stay in the DM but won't be rung.",
    );
  });

  it("turns both buttons off past the group DM's size", () => {
    const plan = selectionPlan([1, 2, 3, 4, 5, 6, 7, 8, 9, 10], isBot);

    expect(plan.messageDisabled).toBe(true);
    expect(plan.huddleDisabled).toBe(true);
    expect(plan.note).toBe("Group DMs hold at most 10 people including you.");
  });

  it("allows nine others and has nothing to say", () => {
    const plan = selectionPlan([1, 2, 3, 4, 5, 6, 7, 8, 9], isBot);

    expect(plan.messageDisabled).toBe(false);
    expect(plan.note).toBe("");
  });
});

describe("toggleSelection", () => {
  const order = [1, 2, 3, 4, 5];

  it("adds and takes out one id", () => {
    expect(toggleSelection([], 3, order, null, false)).toEqual([3]);
    expect(toggleSelection([3, 4], 3, order, 3, false)).toEqual([4]);
  });

  it("selects the range from the anchor on Shift", () => {
    expect(toggleSelection([2], 4, order, 2, true)).toEqual([2, 3, 4]);
    expect(toggleSelection([4], 2, order, 4, true)).toEqual([4, 2, 3]);
  });

  it("clears the range when the clicked one was on", () => {
    expect(toggleSelection([1, 2, 3, 4], 4, order, 2, true)).toEqual([1]);
  });

  it("treats Shift without an anchor as a single toggle", () => {
    expect(toggleSelection([], 4, order, null, true)).toEqual([4]);
  });
});

describe("directoryBadge", () => {
  it("labels agents, other bots and nobody else", () => {
    expect(directoryBadge(user(90, "bot", true), true)).toBe("Agent");
    expect(directoryBadge(user(91, "bot"), false)).toBe("Bot");
    expect(directoryBadge(user(1, "member"), false)).toBeNull();
  });
});

describe("botPage", () => {
  it("opens an agent's profile when the SPA has one, else the classic page", () => {
    expect(botPage(user(90, "bot", true), true)).toBe("/app/agents/90");
    expect(botPage(user(90, "bot", true), false)).toBe("/users/90?classic=1");
    expect(botPage(user(91, "bot"), true)).toBe("/users/91?classic=1");
  });
});

describe("presenceOf", () => {
  it("prefers the live presence and falls back to the loaded one", () => {
    expect(presenceOf("away", "online")).toBe("idle");
    expect(presenceOf("dnd", "offline")).toBe("dnd");
    expect(presenceOf(undefined, "online")).toBe("online");
  });
});

describe("newerUser", () => {
  const at = (minute: number): User => ({
    ...user(5, "member"),
    updatedAt: `2026-10-07T10:0${minute}:00.000000Z`,
  });

  it("picks the later copy and keeps the first on a tie or when there's no other", () => {
    expect(newerUser(at(1), at(2)).updatedAt).toBe(at(2).updatedAt);
    expect(newerUser(at(2), at(1)).updatedAt).toBe(at(2).updatedAt);
    expect(newerUser(at(1), undefined).updatedAt).toBe(at(1).updatedAt);
  });
});

describe("landBan", () => {
  const page = (status: User["status"], minute: number): PersonProfile => ({
    user: { ...user(5, "member"), status, updatedAt: `2026-10-07T10:0${minute}:00.000000Z` },
    status: { presence: status === "active" ? "online" : "offline", statusText: null },
    dndAllowed: status === "active" ? false : null,
    emailAddress: "sam@example.com",
    transferUrl: status === "active" ? "https://chat.example/session/transfers/t" : null,
    transferQrSvg: status === "active" ? "<svg/>" : null,
    canBan: true,
  });

  it("takes a newer reply whole: the status and what follows from it", () => {
    const reply = page("active", 2);

    expect(landBan(page("banned", 1), reply)).toBe(reply);
  });

  it("changes nothing when the page already has a later copy of them", () => {
    const current = page("active", 3);

    expect(landBan(current, page("banned", 2))).toBe(current);
  });
});
