import { act, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { useCelebrations } from "./celebrations.ts";

let celebrations: ReturnType<typeof useCelebrations> | null = null;

function Probe() {
  const current = useCelebrations();

  celebrations = current;

  return <p>{current.has(7) ? "celebrating" : "quiet"}</p>;
}

function control(): ReturnType<typeof useCelebrations> {
  if (celebrations === null) {
    throw new Error("the probe hasn't rendered");
  }

  return celebrations;
}

afterEach(() => {
  vi.useRealTimers();
});

describe("celebrated rows", () => {
  it("clear once the check has drawn in", () => {
    vi.useFakeTimers();
    render(<Probe />);

    act(() => control().start(7));

    expect(screen.getByText("celebrating")).toBeTruthy();

    act(() => vi.advanceTimersByTime(5000));

    expect(screen.getByText("quiet")).toBeTruthy();
  });

  it("clear at once when the action fails", () => {
    render(<Probe />);

    act(() => control().start(7));
    act(() => control().stop(7));

    expect(screen.getByText("quiet")).toBeTruthy();
  });
});
