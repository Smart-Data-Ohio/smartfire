import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { HuddleRing } from "../../gen/HuddleRing.ts";
import { ENDED_TIMEOUT_MS, IncomingCalls, RING_TIMEOUT_MS, ringStore } from "./ring.ts";

function ring(change: Partial<HuddleRing> = {}): HuddleRing {
  return {
    activityItemId: 41,
    event: "started",
    state: "unread",
    roomId: 5,
    roomName: "Maya Okafor",
    callerName: "Maya Okafor",
    silent: false,
    ...change,
  };
}

let log: string[] = [];

let callRoom: number | null = null;

const calls = new IncomingCalls({
  inCall: (roomId) => roomId === callRoom,
  startRinging: () => log.push("ring"),
  stopRinging: () => log.push("stop"),
  notify: () => {
    log.push("notify");

    return () => log.push("close");
  },
  answer: (id, action) => log.push(`answer ${id} ${action}`),
  join: (roomId) => log.push(`join ${roomId}`),
});

beforeEach(() => {
  vi.useFakeTimers();
  calls.hide();
  log = [];
  callRoom = null;
});

afterEach(() => {
  vi.useRealTimers();
});

describe("an incoming call", () => {
  it("rings and notifies, and Join answers the item and joins", () => {
    calls.received(ring());

    expect(ringStore.getState().ring?.callerName).toBe("Maya Okafor");
    expect(log).toEqual(["ring", "notify"]);

    calls.join();
    expect(ringStore.getState().ring).toBeNull();
    expect(log).toContain("answer 41 handled");
    expect(log.at(-1)).toBe("join 5");
  });

  it("Dismiss marks the item read", () => {
    calls.received(ring());
    calls.dismiss();

    expect(log).toContain("answer 41 read");
    expect(log).not.toContain("join 5");
  });

  it("a ring without an item answers nothing", () => {
    calls.received(ring({ activityItemId: null }));
    calls.dismiss();

    expect(log.some((entry) => entry.startsWith("answer"))).toBe(false);
  });

  it("a silent ring shows without a sound", () => {
    calls.received(ring({ silent: true }));

    expect(ringStore.getState().ring).not.toBeNull();
    expect(log).toEqual(["stop"]);
  });

  it("never rings for the call the viewer is in", () => {
    callRoom = 5;
    calls.received(ring());

    expect(ringStore.getState().ring).toBeNull();
  });

  it("ignores read or handled news as a new ring", () => {
    calls.received(ring({ state: "read" }));

    expect(ringStore.getState().ring).toBeNull();
  });

  it("stops by itself after 45 seconds", () => {
    calls.received(ring());
    vi.advanceTimersByTime(RING_TIMEOUT_MS);

    expect(ringStore.getState().ring).toBeNull();
    expect(log).toContain("stop");
  });

  it("says the caller left, then goes", () => {
    calls.received(ring());
    calls.received(ring({ event: "ended", state: "read" }));

    expect(ringStore.getState().phase).toBe("ended");

    vi.advanceTimersByTime(ENDED_TIMEOUT_MS);
    expect(ringStore.getState().ring).toBeNull();
  });

  it("an ended event for another ring changes nothing", () => {
    calls.received(ring());
    calls.received(ring({ event: "ended", activityItemId: 99 }));

    expect(ringStore.getState().phase).toBe("ringing");
  });

  it("matches a ring without an item by room", () => {
    calls.received(ring({ activityItemId: null }));
    calls.received(ring({ event: "ended", activityItemId: null, roomId: 6 }));
    expect(ringStore.getState().phase).toBe("ringing");

    calls.received(ring({ event: "ended", activityItemId: null }));
    expect(ringStore.getState().phase).toBe("ended");
  });

  it("hides when the item is answered elsewhere, or the viewer joins the call", () => {
    calls.received(ring());
    calls.received(ring({ event: "missed", state: "handled" }));
    expect(ringStore.getState().ring).toBeNull();

    calls.received(ring());
    calls.callActive(5);
    expect(ringStore.getState().ring).toBeNull();
  });
});
