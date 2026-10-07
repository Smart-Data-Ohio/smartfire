import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { HuddleNotice } from "../../gen/HuddleNotice.ts";
import {
  bannerText,
  CallNotices,
  JOIN_BATCH_MS,
  JOIN_TOAST_MS,
  LEAVE_DELAY_MS,
  LEAVE_TOAST_MS,
  noticeStore,
  sentence,
} from "./notices.ts";

const ROOM = 8;

const OTHER = 9;

function joined(userId: number, name: string, rejoin = false, roomId = ROOM): HuddleNotice {
  return {
    kind: "joined",
    roomId,
    roomName: "Lounge",
    userId,
    userName: name,
    inCall: false,
    rejoin,
  };
}

function left(userId: number, name: string, roomId = ROOM): HuddleNotice {
  return { kind: "left", roomId, roomName: "Lounge", userId, userName: name };
}

function toasts(): string[] {
  return noticeStore.getState().toasts.map((toast) => toast.text);
}

let callRoom: number | null = ROOM;

let sounds = 0;

const notices = new CallNotices({
  inCall: (roomId) => roomId === callRoom,
  playJoinSound: () => {
    sounds += 1;
  },
});

beforeEach(() => {
  vi.useFakeTimers();
  notices.reset();
  callRoom = ROOM;
  sounds = 0;
});

afterEach(() => {
  notices.reset();
  vi.useRealTimers();
});

describe("wording", () => {
  it("lists names as a sentence", () => {
    expect(sentence(["Chris"])).toBe("Chris");
    expect(sentence(["Chris", "Dean"])).toBe("Chris and Dean");
    expect(sentence(["Chris", "Dean", "Erin"])).toBe("Chris, Dean, and Erin");
  });

  it("says who's in the huddle", () => {
    expect(bannerText([])).toBe("Someone is in your huddle");
    expect(bannerText([{ id: 1, name: "Maya" }])).toBe("Maya is in your huddle");
    expect(
      bannerText([
        { id: 1, name: "Maya" },
        { id: 2, name: "Jonah" },
      ]),
    ).toBe("Maya and Jonah are in your huddle");
  });
});

describe("in the call", () => {
  it("batches joins into one toast with one blip, then clears it", () => {
    notices.received(joined(1, "Chris"));
    vi.advanceTimersByTime(JOIN_BATCH_MS - 1000);
    notices.received(joined(2, "Dean"));

    expect(toasts()).toEqual(["Chris and Dean joined"]);
    expect(sounds).toBe(1);

    vi.advanceTimersByTime(JOIN_TOAST_MS);
    expect(toasts()).toEqual([]);
  });

  it("starts a fresh toast after the batch window", () => {
    notices.received(joined(1, "Chris"));
    vi.advanceTimersByTime(JOIN_BATCH_MS + 1);
    notices.received(joined(2, "Dean"));

    expect(toasts()).toEqual(["Chris joined", "Dean joined"]);
    expect(sounds).toBe(2);
  });

  it("waits out the leave delay before saying someone left", () => {
    notices.received(left(1, "Chris"));
    expect(toasts()).toEqual([]);

    vi.advanceTimersByTime(LEAVE_DELAY_MS);
    expect(toasts()).toEqual(["Chris left"]);

    vi.advanceTimersByTime(LEAVE_TOAST_MS);
    expect(toasts()).toEqual([]);
  });

  it("forgets a leave toast's dismissal when it goes early or on reset", () => {
    for (const [id, name] of [
      [1, "A"],
      [2, "B"],
      [3, "C"],
      [4, "D"],
    ] as const) {
      notices.received(left(id, name));
    }

    vi.advanceTimersByTime(LEAVE_DELAY_MS);

    // Three toasts dismiss themselves (A's went early) and four leaves remember they fired.
    expect(vi.getTimerCount()).toBe(7);

    notices.reset();

    expect(vi.getTimerCount()).toBe(0);
  });

  it("keeps at most three leave toasts", () => {
    for (const [id, name] of [
      [1, "A"],
      [2, "B"],
      [3, "C"],
      [4, "D"],
    ] as const) {
      notices.received(left(id, name));
    }

    vi.advanceTimersByTime(LEAVE_DELAY_MS);
    expect(toasts()).toEqual(["B left", "C left", "D left"]);
  });

  it("a mute cycle (leave, then join) says nothing", () => {
    notices.received(left(1, "Chris"));
    notices.received(joined(1, "Chris", true));
    vi.advanceTimersByTime(LEAVE_DELAY_MS * 2);

    expect(toasts()).toEqual([]);
    expect(sounds).toBe(0);
  });

  it("a mute cycle the other way round (marked join, then leave) says nothing", () => {
    notices.received(joined(1, "Chris", true));
    notices.received(left(1, "Chris"));
    vi.advanceTimersByTime(LEAVE_DELAY_MS * 2);

    expect(toasts()).toEqual([]);
  });

  it("a marked rejoin after a leave that already toasted is a real join", () => {
    notices.received(left(1, "Chris"));
    vi.advanceTimersByTime(LEAVE_DELAY_MS);
    notices.received(joined(1, "Chris", true));

    expect(toasts()).toEqual(["Chris left", "Chris joined"]);

    // Nothing was armed, so the next real leave still toasts.
    notices.received(left(1, "Chris"));
    vi.advanceTimersByTime(LEAVE_DELAY_MS);
    expect(toasts().filter((text) => text === "Chris left").length).toBeGreaterThan(0);
  });

  it("drops a pending leave toast if the viewer leaves first", () => {
    notices.received(left(1, "Chris"));
    callRoom = null;
    vi.advanceTimersByTime(LEAVE_DELAY_MS);

    expect(toasts()).toEqual([]);
  });
});

describe("a call elsewhere", () => {
  it("banners the joiners and drops each one who leaves", () => {
    callRoom = null;
    notices.received(joined(1, "Maya"));
    notices.received(joined(2, "Jonah"));
    notices.received(joined(2, "Jonah"));

    expect(noticeStore.getState().banners[ROOM]?.joiners.map((joiner) => joiner.name)).toEqual([
      "Maya",
      "Jonah",
    ]);
    expect(toasts()).toEqual([]);

    notices.received(left(1, "Maya"));
    expect(noticeStore.getState().banners[ROOM]?.joiners).toHaveLength(1);

    notices.received(left(2, "Jonah"));
    expect(noticeStore.getState().banners[ROOM]).toBeUndefined();
  });

  it("clears when the call ends, when the viewer joins, or when presence empties", () => {
    callRoom = null;
    notices.received(joined(1, "Maya"));
    notices.received({ kind: "ended", roomId: ROOM });
    expect(noticeStore.getState().banners[ROOM]).toBeUndefined();

    notices.received(joined(1, "Maya"));
    notices.callActive(ROOM);
    expect(noticeStore.getState().banners[ROOM]).toBeUndefined();

    notices.received(joined(1, "Maya"));
    notices.received(joined(2, "Jonah"));
    notices.presenceChanged(ROOM, [2]);
    expect(noticeStore.getState().banners[ROOM]?.joiners.map((joiner) => joiner.id)).toEqual([2]);

    notices.presenceChanged(ROOM, []);
    expect(noticeStore.getState().banners[ROOM]).toBeUndefined();
  });

  it("keeps rooms apart", () => {
    callRoom = null;
    notices.received(joined(1, "Maya", false, ROOM));
    notices.received(joined(2, "Jonah", false, OTHER));
    notices.dismiss(ROOM);

    expect(Object.keys(noticeStore.getState().banners)).toEqual([String(OTHER)]);
  });
});
