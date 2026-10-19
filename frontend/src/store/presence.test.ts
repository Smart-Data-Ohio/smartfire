import { describe, expect, it } from "vitest";
import type { UserPresence } from "./model.ts";
import { applyEvents, setPresence } from "./reducers.ts";
import { initialState } from "./state.ts";

const online: UserPresence = { userId: 7, presence: "online", statusText: null };

const other: UserPresence = { userId: 8, presence: "idle", statusText: "Reading" };

describe("repeated presence heartbeats", () => {
  it("keeps the store and presence entries unchanged for an identical sync event", () => {
    const state = setPresence(initialState, [online, other]);
    const next = applyEvents(
      state,
      [{ seq: 2, topic: "user", type: "presence", data: { ...online } }],
      0,
    );

    expect(next).toBe(state);
    expect(next.presence[7]).toBe(state.presence[7]);
    expect(next.presence[8]).toBe(state.presence[8]);
  });

  it("applies a changed presence while preserving the other person's entry", () => {
    const state = setPresence(initialState, [online, other]);
    const next = setPresence(state, [{ ...online, presence: "idle" }, { ...other }]);

    expect(next.presence[7]?.presence).toBe("idle");
    expect(next.presence[8]).toBe(state.presence[8]);
  });

  it("applies status text changes even when the presence stays online", () => {
    const state = setPresence(initialState, [online]);
    const changed = { ...online, statusText: "Back at my desk" };

    expect(setPresence(state, [changed]).presence[7]).toEqual(changed);
  });
});
