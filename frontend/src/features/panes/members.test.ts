import { describe, expect, it } from "vitest";
import type { Member } from "../../gen/Member.ts";
import type { User } from "../../store/model.ts";
import { canStar, foldName, groupMembers, type MemberGroupingInput } from "./members.ts";

function user(id: number, name: string, role: User["role"] = "member"): User {
  return {
    id,
    name,
    role,
    status: "active",
    bio: null,
    avatarUrl: `/avatars/${id}`,
    hasAvatar: false,
    customStatus: null,
    createdAt: "2026-01-01T00:00:00.000Z",
  };
}

function member(userId: number, presence: Member["presence"], starred = false): Member {
  return { userId, presence, statusText: null, starred };
}

const users = {
  1: user(1, "Ada"),
  2: user(2, "Zoë"),
  3: user(3, "Grace"),
  4: user(4, "Helper", "bot"),
};

function input(extra: Partial<MemberGroupingInput> = {}): MemberGroupingInput {
  return {
    members: [
      member(1, "online"),
      member(2, "offline", true),
      member(3, "idle"),
      member(4, "offline"),
    ],
    users,
    presence: {},
    stars: {},
    query: "",
    viewerId: 1,
    ...extra,
  };
}

function names(sections: ReturnType<typeof groupMembers>): Record<string, string[]> {
  return Object.fromEntries(
    sections.map((section) => [section.key, section.members.map((entry) => entry.name)]),
  );
}

describe("groupMembers", () => {
  it("puts starred people first, then online (idle counts), then offline", () => {
    expect(names(groupMembers(input()))).toEqual({
      starred: ["Zoë"],
      online: ["Ada", "Grace"],
      offline: ["Helper"],
    });
  });

  it("lets live presence and local stars win over the list", () => {
    const sections = groupMembers(
      input({
        presence: { 3: { userId: 3, presence: "offline", statusText: "Out" } },
        stars: { 2: false },
      }),
    );

    expect(names(sections)).toEqual({ online: ["Ada"], offline: ["Zoë", "Grace", "Helper"] });
    expect(sections[1]?.members[1]?.statusText).toBe("Out");
  });

  it("searches names without case or accents and drops empty sections", () => {
    expect(names(groupMembers(input({ query: "ZOE" })))).toEqual({ starred: ["Zoë"] });
    expect(groupMembers(input({ query: "nobody" }))).toEqual([]);
  });

  it("flags bots and the viewer, who can't be starred", () => {
    const entries = groupMembers(input()).flatMap((section) => section.members);
    const byName = new Map(entries.map((entry) => [entry.name, entry]));

    expect(byName.get("Ada")?.viewer).toBe(true);
    expect([...byName.values()].flatMap((entry) => (canStar(entry) ? [entry.name] : []))).toEqual([
      "Zoë",
      "Grace",
    ]);
  });
});

describe("foldName", () => {
  it("folds case, accents and outer space", () => {
    expect(foldName("  Zoë Ångström ")).toBe("zoe angstrom");
  });
});
