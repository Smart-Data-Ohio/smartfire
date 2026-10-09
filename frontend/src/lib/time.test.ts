import { describe, expect, it } from "vitest";
import { dayKey, formatDay, formatListTime } from "./time.ts";

describe("formatDay", () => {
  const now = new Date(2026, 9, 6, 15, 30).getTime();

  it("names today and yesterday", () => {
    expect(formatDay(new Date(2026, 9, 6, 0, 5).getTime(), now)).toBe("Today");
    expect(formatDay(new Date(2026, 9, 5, 23, 59).getTime(), now)).toBe("Yesterday");
  });

  it("spells out older days, with the year only when it differs", () => {
    expect(formatDay(new Date(2026, 9, 1, 12).getTime(), now)).toContain("October");
    expect(formatDay(new Date(2025, 11, 31, 12).getTime(), now)).toContain("2025");
  });
});

describe("dayKey", () => {
  it("keys by local calendar day", () => {
    expect(dayKey(new Date(2026, 0, 2, 23, 59).getTime())).toBe("2026-01-02");
  });
});

describe("formatListTime", () => {
  const now = new Date(2026, 9, 6, 15, 30).getTime();
  const at = (...parts: [number, number, number, number]) => new Date(...parts).toISOString();

  it("shows the time today, then yesterday and the weekday", () => {
    expect(formatListTime(at(2026, 9, 6, 9), now)).toMatch(/9/);
    expect(formatListTime(at(2026, 9, 5, 23), now)).toBe("Yesterday");
    expect(formatListTime(at(2026, 9, 2, 12), now)).not.toContain("October");
  });

  it("falls back to a short date, with the year only when it differs", () => {
    expect(formatListTime(at(2026, 8, 20, 12), now)).not.toContain("2026");
    expect(formatListTime(at(2025, 11, 31, 12), now)).toContain("2025");
  });
});
