import { describe, expect, it } from "vitest";
import { dayKey, formatDay } from "./time.ts";

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
