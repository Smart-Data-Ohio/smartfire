import { describe, expect, it } from "vitest";
import { userFixture } from "../../api/testing.ts";
import type { User } from "../../store/model.ts";
import {
  addPlan,
  directIntent,
  MAX_OTHERS,
  pickerOptions,
  removeLast,
  toggleSelected,
} from "./picker.ts";

const USERS = {
  2: userFixture(2, "Maya Chen"),
  3: userFixture(3, "Jonah Park"),
  4: userFixture(4, "Priya Shah"),
  5: { ...userFixture(5, "Dana Gone"), status: "deactivated" },
  9: { ...userFixture(9, "Ember"), role: "bot" },
} satisfies Readonly<Record<number, User>>;

const CANDIDATES = [
  { userId: 4, agent: false, starred: true },
  { userId: 9, agent: true, starred: false },
  { userId: 3, agent: false, starred: false },
  { userId: 2, agent: false, starred: false },
  { userId: 5, agent: false, starred: false },
  { userId: 77, agent: false, starred: false },
];

const ids = (options: readonly { readonly userId: number }[]) =>
  options.map((option) => option.userId);

describe("pickerOptions", () => {
  it("keeps the server's order with no query, skipping inactive and unknown people", () => {
    expect(ids(pickerOptions(CANDIDATES, USERS, "", new Set()))).toEqual([4, 9, 3, 2]);
  });

  it("hides people already chosen or already in the conversation", () => {
    expect(ids(pickerOptions(CANDIDATES, USERS, "", new Set([4, 2])))).toEqual([9, 3]);
  });

  it("ranks by match when there is a query", () => {
    expect(ids(pickerOptions(CANDIDATES, USERS, "park", new Set()))).toEqual([3]);
    expect(ids(pickerOptions(CANDIDATES, USERS, "sha", new Set()))).toEqual([4]);
    expect(ids(pickerOptions(CANDIDATES, USERS, "@em", new Set()))).toEqual([9]);
    expect(pickerOptions(CANDIDATES, USERS, "ember", new Set())[0]).toMatchObject({
      name: "Ember",
      agent: true,
    });
  });
});

describe("chip selection", () => {
  it("toggles people in and out, in the order they were picked", () => {
    const picked = toggleSelected(toggleSelected([], 3), 2);

    expect(picked).toEqual([3, 2]);
    expect(toggleSelected(picked, 3)).toEqual([2]);
  });

  it("stops at the conversation's limit", () => {
    const full = Array.from({ length: MAX_OTHERS }, (_, index) => index + 100);

    expect(toggleSelected(full, 1)).toBe(full);
    expect(toggleSelected([1, 2], 3, 2)).toEqual([1, 2]);
  });

  it("takes the last chip off on Backspace", () => {
    expect(removeLast([3, 2])).toEqual([3]);
    expect(removeLast([])).toEqual([]);
  });

  it("says whether Enter makes a DM or a group DM", () => {
    expect(directIntent([])).toBe("none");
    expect(directIntent([3])).toBe("direct");
    expect(directIntent([3, 2])).toBe("group");
  });
});

describe("addPlan", () => {
  it("grows a group DM in place", () => {
    expect(addPlan([3, 4], false, [2])).toEqual({ kind: "add", userIds: [2] });
    expect(addPlan([3], true, [2])).toEqual({ kind: "add", userIds: [2] });
  });

  it("starts a new group from a one-to-one, with its member", () => {
    expect(addPlan([3], false, [2, 4])).toEqual({ kind: "new-group", userIds: [3, 2, 4] });
  });
});
