import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { SlackRun } from "../../gen/SlackRun.ts";
import { ActionError } from "../../sync/run.ts";
import { POLL_MS } from "./slack-format.ts";
import { newestOnly, usePoll } from "./slack-poll.ts";

const RUNNING: SlackRun = {
  id: 12,
  kind: "workspace",
  mode: "dry_run",
  status: "running",
  title: "Workspace dry run #12",
  startedBy: "Grace",
  createdAt: "2026-10-06T12:00:00.000Z",
  startedAt: "2026-10-06T12:00:05.000Z",
  finishedAt: null,
  phase: "conversations",
  current: "#general",
  queuedBehind: false,
  people: null,
  counts: null,
  apiCalls: 41,
  issuesCount: 0,
  error: null,
  active: true,
  cancellable: true,
  undoable: false,
  undoBlockedReason: null,
  planReady: false,
  catchUp: false,
  conversations: [],
};

const DONE: SlackRun = { ...RUNNING, status: "completed", active: false, cancellable: false };

/** Sets whether the tab is hidden and tells the page, as the browser does. */
function hide(hidden: boolean) {
  Object.defineProperty(document, "hidden", { configurable: true, get: () => hidden });
  document.dispatchEvent(new Event("visibilitychange"));
}

/** Lets the timers run `ms` on and the answers they started settle. */
async function advance(ms: number) {
  await act(async () => {
    await vi.advanceTimersByTimeAsync(ms);
  });
}

describe("a run page's polling", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    hide(false);
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("reads every few seconds while the run is active, then settles", async () => {
    const read = vi.fn().mockResolvedValueOnce(RUNNING).mockResolvedValueOnce(DONE);
    const settled = vi.fn();

    renderHook(() => usePoll(RUNNING, read, settled));

    await advance(POLL_MS - 1);
    expect(read).not.toHaveBeenCalled();

    await advance(1);
    expect(read).toHaveBeenCalledTimes(1);

    await advance(POLL_MS);
    expect(read).toHaveBeenCalledTimes(2);
    expect(settled).toHaveBeenCalledTimes(1);
  });

  it("pauses while the tab is hidden and reads at once when it's shown", async () => {
    const read = vi.fn().mockResolvedValue(RUNNING);

    renderHook(() => usePoll(RUNNING, read, () => undefined));

    act(() => hide(true));
    await advance(POLL_MS * 4);
    expect(read).not.toHaveBeenCalled();

    act(() => hide(false));
    await advance(0);
    expect(read).toHaveBeenCalledTimes(1);
  });

  it("waits longer after each failed read and says so", async () => {
    const read = vi.fn().mockRejectedValue(new ActionError("NetworkError", "offline"));
    const { result } = renderHook(() => usePoll(RUNNING, read, () => undefined));

    await advance(POLL_MS);
    expect(read).toHaveBeenCalledTimes(1);
    expect(result.current).toBe("retrying");

    await advance(POLL_MS);
    expect(read).toHaveBeenCalledTimes(1);

    await advance(POLL_MS);
    expect(read).toHaveBeenCalledTimes(2);
  });

  it("stops when the run is gone", async () => {
    const read = vi.fn().mockRejectedValue(new ActionError("NotFound", "Not found"));
    const { result } = renderHook(() => usePoll(RUNNING, read, () => undefined));

    await advance(POLL_MS);
    expect(result.current).toBe("stopped");

    await advance(POLL_MS * 20);
    expect(read).toHaveBeenCalledTimes(1);
  });

  it("keeps going when a newer read answered first", async () => {
    const read = vi.fn().mockResolvedValueOnce(null).mockResolvedValueOnce(DONE);
    const settled = vi.fn();

    renderHook(() => usePoll(RUNNING, read, settled));

    await advance(POLL_MS);
    await advance(POLL_MS);
    expect(read).toHaveBeenCalledTimes(2);
    expect(settled).toHaveBeenCalledTimes(1);
  });

  it("doesn't read a run that isn't active", async () => {
    const read = vi.fn();

    renderHook(() => usePoll(DONE, read, () => undefined));

    await advance(POLL_MS * 3);
    expect(read).not.toHaveBeenCalled();
  });
});

describe("tickets", () => {
  it("make a read that started before a write stale", () => {
    const tickets = newestOnly();
    const read = tickets.take();

    expect(read()).toBe(true);

    // An undo lands and replaces the run; the read's answer must not put the old status back.
    tickets.take();
    expect(read()).toBe(false);
  });
});
