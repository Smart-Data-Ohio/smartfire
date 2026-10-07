import { describe, expect, it } from "vitest";
import type { Thread } from "../../store/model.ts";
import { lastReplyLabel, replyCountLabel, threadTitle, timeAgo } from "./thread-format.ts";

const NOW = Date.parse("2026-10-06T12:00:00.000Z");

function ago(millis: number): string {
  return new Date(NOW - millis).toISOString();
}

const MINUTE = 60_000;

const HOUR = 60 * MINUTE;

const DAY = 24 * HOUR;

describe("replyCountLabel", () => {
  it("says reply for one and replies otherwise", () => {
    expect(replyCountLabel(1)).toBe("1 reply");
    expect(replyCountLabel(0)).toBe("0 replies");
    expect(replyCountLabel(12)).toBe("12 replies");
  });
});

describe("timeAgo", () => {
  it("says just now under a minute, and for a clock that's a little ahead", () => {
    expect(timeAgo(ago(20_000), NOW)).toBe("just now");
    expect(timeAgo(ago(-5_000), NOW)).toBe("just now");
  });

  it("counts minutes, hours and days", () => {
    expect(timeAgo(ago(MINUTE), NOW)).toBe("1 minute ago");
    expect(timeAgo(ago(5 * MINUTE + 30_000), NOW)).toBe("5 minutes ago");
    expect(timeAgo(ago(3 * HOUR), NOW)).toBe("3 hours ago");
    expect(timeAgo(ago(DAY), NOW)).toBe("yesterday");
    expect(timeAgo(ago(4 * DAY), NOW)).toBe("4 days ago");
  });

  it("gives the date after a week, with the year only when it differs", () => {
    const sameYear = timeAgo(ago(10 * DAY), NOW);
    const lastYear = timeAgo("2025-03-02T12:00:00.000Z", NOW);

    expect(sameYear).not.toMatch(/2026/);
    expect(sameYear).toMatch(/26|Sep/);
    expect(lastYear).toMatch(/2025/);
  });
});

describe("lastReplyLabel", () => {
  it("prefixes the time", () => {
    expect(lastReplyLabel(ago(3 * HOUR), NOW)).toBe("Last reply 3 hours ago");
  });
});

describe("threadTitle", () => {
  const thread: Thread = {
    id: 1,
    roomId: 2,
    parentMessageId: 3,
    creatorId: 4,
    name: "Launch plan",
    status: "active",
    replyCount: 2,
    lastActivityAt: ago(HOUR),
    autoArchiveAfterMinutes: 1440,
    createdAt: ago(DAY),
    work: null,
  };

  it("uses the name, or Thread when it's blank or unknown", () => {
    expect(threadTitle(thread)).toBe("Launch plan");
    expect(threadTitle({ ...thread, name: "   " })).toBe("Thread");
    expect(threadTitle(undefined)).toBe("Thread");
  });
});
