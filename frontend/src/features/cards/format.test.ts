import { describe, expect, it } from "vitest";
import { ago, closesLabel, compactCount, hostOf, percent } from "./format.ts";

const NOW = Date.parse("2026-10-06T12:00:00.000Z");

const later = (ms: number) => new Date(NOW + ms).toISOString();

describe("card formatting", () => {
  it("counts down to a poll closing, and says Closed once past", () => {
    expect(closesLabel(later(2 * 3_600_000), NOW)).toBe("Closes in 2 hours");
    expect(closesLabel(later(10_000), NOW)).toBe("Closes in 1 minute");
    expect(closesLabel(later(24 * 3_600_000), NOW)).toBe("Closes tomorrow");
    expect(closesLabel(later(-1), NOW)).toBe("Closed");
  });

  it("says how long ago, never in the future", () => {
    expect(ago(later(-3 * 60_000), NOW)).toBe("3 minutes ago");
    expect(ago(later(60_000), NOW)).toBe("now");
  });

  it("rounds shares and keeps an empty poll at zero", () => {
    expect(percent(1, 3)).toBe(33);
    expect(percent(2, 3)).toBe(67);
    expect(percent(0, 0)).toBe(0);
  });

  it("shortens big counts and shows a link's host", () => {
    expect(compactCount(412)).toBe("412");
    expect(compactCount(12_400)).toBe("12.4K");
    expect(hostOf("https://www.example.com/a/b")).toBe("example.com");
    expect(hostOf("not a url")).toBe("not a url");
  });
});
