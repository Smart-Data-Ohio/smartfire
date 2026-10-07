import { describe, expect, it } from "vitest";
import { ago, closesLabel, compactCount, eventTile, eventWhen, hostOf, percent } from "./format.ts";

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

  it("shows an event in its own zone, with the year and the zone's name", () => {
    const zone = "America/New_York";

    expect(eventWhen("2026-10-08T15:00:00Z", "2026-10-08T15:45:00Z", zone)).toBe(
      "Thu, Oct 8, 2026 · 11:00 AM – 11:45 AM EDT",
    );
    expect(eventWhen("2027-01-04T15:00:00Z", null, zone)).toBe("Mon, Jan 4, 2027 · 10:00 AM EST");
    expect(eventWhen("2026-10-09T03:00:00Z", "2026-10-09T05:00:00Z", zone)).toBe(
      "Thu, Oct 8, 2026 · 11:00 PM – Fri, Oct 9, 2026 · 1:00 AM EDT",
    );
    // 02:30 UTC on the 9th is still the 8th in New York.
    expect(eventTile("2026-10-09T02:30:00Z", zone)).toEqual({ month: "Oct", day: "8" });
    expect(eventWhen("2026-10-08T15:00:00Z", null, "Asia/Tokyo")).toBe(
      "Fri, Oct 9, 2026 · 12:00 AM GMT+9",
    );
  });

  it("falls back to the viewer's zone when the event's isn't known here", () => {
    expect(eventWhen("2026-10-08T15:00:00Z", null, "Not/AZone")).toMatch(
      /^\w{3}, \w{3} \d+, 2026 · /,
    );
  });

  it("shortens big counts and shows a link's host", () => {
    expect(compactCount(412)).toBe("412");
    expect(compactCount(12_400)).toBe("12.4K");
    expect(hostOf("https://www.example.com/a/b")).toBe("example.com");
    expect(hostOf("not a url")).toBe("not a url");
  });
});
