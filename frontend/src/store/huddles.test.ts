import { describe, expect, it } from "vitest";
import { userFixture } from "../api/testing.ts";
import type { HuddlePresence } from "../gen/HuddlePresence.ts";
import type { StageState } from "../gen/StageState.ts";
import { loadHuddlePresence, loadStageDetail, setHuddlePresence, setStage } from "./huddles.ts";
import { initialState } from "./state.ts";

function presence(roomId: number, userIds: readonly number[], live = false): HuddlePresence {
  return {
    roomId,
    participants: userIds.map((userId) => ({
      userId,
      membershipId: roomId * 100 + userId,
      identities: [`identity-${userId}`],
      serverMuted: false,
    })),
    live,
  };
}

const stage: StageState = {
  roomId: 12,
  members: [
    { membershipId: 1, userId: 1, role: "host", handRaisedAt: null, serverMuted: false },
    { membershipId: 2, userId: 2, role: "listener", handRaisedAt: null, serverMuted: false },
  ],
  live: null,
};

describe("huddle presence", () => {
  it("keeps a call while someone is in it and drops it when it ends", () => {
    const joined = setHuddlePresence(initialState, presence(8, [1, 2]));

    expect(joined.huddles[8]?.participants).toHaveLength(2);

    const ended = setHuddlePresence(joined, presence(8, []));

    expect(ended.huddles[8]).toBeUndefined();
  });

  it("ignores an empty call it never held", () => {
    expect(setHuddlePresence(initialState, presence(8, []))).toBe(initialState);
  });

  it("keeps a live stage nobody is in", () => {
    expect(setHuddlePresence(initialState, presence(12, [], true)).huddles[12]?.live).toBe(true);
  });

  it("replaces everything with the poll, so quiet drop-outs disappear", () => {
    const before = setHuddlePresence(
      setHuddlePresence(initialState, presence(8, [1])),
      presence(9, [2]),
    );

    const after = loadHuddlePresence(before, {
      rooms: [presence(9, [2, 3]), presence(10, [])],
      users: [userFixture(3, "Cleo")],
    });

    expect(Object.keys(after.huddles)).toEqual(["9"]);
    expect(after.huddles[9]?.participants).toHaveLength(2);
    expect(after.users[3]?.name).toBe("Cleo");
  });
});

describe("stages", () => {
  it("stores a stage's roster by room", () => {
    const next = setStage(initialState, stage);

    expect(next.stages[12]?.members.map((member) => member.role)).toEqual(["host", "listener"]);
  });

  it("merges the detail's profiles", () => {
    const next = loadStageDetail(initialState, {
      stage,
      users: [userFixture(1, "Hana"), userFixture(2, "Ivo")],
    });

    expect(next.stages[12]).toBe(stage);
    expect(next.users[2]?.name).toBe("Ivo");
  });
});
