import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ActionError } from "../../sync/run.ts";

import { huddleAvailability, PRESENCE_POLL_MS, startPresencePolling } from "./presence.ts";

const refreshPresence = vi.fn<() => Promise<void>>();

/** Lets pending promises run (only intervals are faked). */
function settle(): Promise<void> {
  return new Promise((resolve) => {
    setTimeout(resolve, 0);
  });
}

beforeEach(() => {
  vi.useFakeTimers({ toFake: ["setInterval", "clearInterval"] });
  refreshPresence.mockReset();
  refreshPresence.mockResolvedValue(undefined);
  huddleAvailability.setState({ available: true });
});

afterEach(() => {
  vi.useRealTimers();
});

describe("presence polling", () => {
  it("polls at once and then every 15 seconds until stopped", async () => {
    const stop = startPresencePolling(refreshPresence);

    expect(refreshPresence).toHaveBeenCalledTimes(1);
    await vi.advanceTimersByTimeAsync(PRESENCE_POLL_MS * 2);
    expect(refreshPresence).toHaveBeenCalledTimes(3);

    stop();
    await vi.advanceTimersByTimeAsync(PRESENCE_POLL_MS * 2);
    expect(refreshPresence).toHaveBeenCalledTimes(3);
  });

  it("hides huddles when the server has none, and shows them again when it does", async () => {
    refreshPresence.mockRejectedValueOnce(new ActionError("Unavailable", "No LiveKit"));

    const stop = startPresencePolling(refreshPresence);

    await settle();
    expect(huddleAvailability.getState().available).toBe(false);

    await vi.advanceTimersByTimeAsync(PRESENCE_POLL_MS);
    await settle();
    expect(huddleAvailability.getState().available).toBe(true);
    stop();
  });

  it("keeps huddles shown through other failures", async () => {
    refreshPresence.mockRejectedValue(new ActionError("NetworkError", "offline"));

    const stop = startPresencePolling(refreshPresence);

    await settle();
    expect(huddleAvailability.getState().available).toBe(true);
    stop();
  });
});
