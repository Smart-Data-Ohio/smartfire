import { act, renderHook } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import type { SlackPlan } from "../../gen/SlackPlan.ts";
import { usePlanLoad } from "./slack-plan-section.tsx";

const plan = (runId: number): SlackPlan => ({
  runId,
  conversations: [],
  rooms: [],
  samples: [],
  defaultOldest: "2026-09-22",
  scannedFrom: null,
  scannedUntil: null,
  startedAt: null,
});

/** A plan reader whose answers the test settles by hand, per run. */
function heldReads() {
  const answers = new Map<number, (plan: SlackPlan) => void>();

  const read = (runId: number) =>
    new Promise<SlackPlan>((resolve) => {
      answers.set(runId, resolve);
    });

  const answer = async (runId: number) => {
    await act(async () => {
      answers.get(runId)?.(plan(runId));
    });
  };

  return { read, answer };
}

describe("an import plan's read", () => {
  it("keeps the newer plan when the earlier plan's answer lands last", async () => {
    const { read, answer } = heldReads();

    const { result, rerender } = renderHook(({ runId }) => usePlanLoad(runId, true, read), {
      initialProps: { runId: 7 },
    });

    rerender({ runId: 8 });
    await answer(8);
    await answer(7);

    expect(result.current.load).toEqual({ status: "ready", plan: plan(8) });
  });
});
