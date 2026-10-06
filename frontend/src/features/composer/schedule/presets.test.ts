import { describe, expect, it } from "vitest";
import {
  customTimeProblem,
  fromLocalInput,
  schedulePresets,
  sendAtLabel,
  toLocalInput,
} from "./presets.ts";

/** Tuesday 6 October 2026, 14:37:42 local time. */
const TUESDAY = new Date(2026, 9, 6, 14, 37, 42);

function presetTimes(now: Date) {
  return schedulePresets(now, "en-US").map((preset) => [preset.id, toLocalInput(preset.at)]);
}

describe("schedulePresets", () => {
  it("offers an hour from now, tomorrow morning and next Monday morning", () => {
    expect(presetTimes(TUESDAY)).toEqual([
      ["hour", "2026-10-06T15:37"],
      ["tomorrow", "2026-10-07T09:00"],
      ["monday", "2026-10-12T09:00"],
    ]);
  });

  it("labels the day presets with the morning time", () => {
    expect(schedulePresets(TUESDAY, "en-US").map((preset) => preset.label)).toEqual([
      "In 1 hour",
      "Tomorrow at 9:00 AM",
      "Monday at 9:00 AM",
    ]);
  });

  it("picks next week's Monday on a Monday, and tomorrow's on a Sunday", () => {
    expect(presetTimes(new Date(2026, 9, 5, 8, 0))[2]).toEqual(["monday", "2026-10-12T09:00"]);
    expect(presetTimes(new Date(2026, 9, 11, 20, 0))[2]).toEqual(["monday", "2026-10-12T09:00"]);
  });

  it("rolls over months and years", () => {
    expect(presetTimes(new Date(2026, 11, 31, 23, 30))).toEqual([
      ["hour", "2027-01-01T00:30"],
      ["tomorrow", "2027-01-01T09:00"],
      ["monday", "2027-01-04T09:00"],
    ]);
  });
});

describe("sendAtLabel", () => {
  it("reads relative to today", () => {
    expect(sendAtLabel(new Date(2026, 9, 6, 16, 0), TUESDAY, "en-US")).toBe("Today at 4:00 PM");
    expect(sendAtLabel(new Date(2026, 9, 7, 9, 0), TUESDAY, "en-US")).toBe("Tomorrow at 9:00 AM");
    expect(sendAtLabel(new Date(2026, 9, 12, 9, 0), TUESDAY, "en-US")).toBe(
      "Mon, Oct 12 at 9:00 AM",
    );
    expect(sendAtLabel(new Date(2027, 0, 4, 9, 0), TUESDAY, "en-US")).toBe(
      "Mon, Jan 4, 2027 at 9:00 AM",
    );
  });
});

describe("local input values", () => {
  it("round-trips a datetime-local value", () => {
    const value = toLocalInput(TUESDAY);

    expect(value).toBe("2026-10-06T14:37");
    expect(fromLocalInput(value)?.getTime()).toBe(new Date(2026, 9, 6, 14, 37).getTime());
    expect(fromLocalInput("")).toBeNull();
    expect(fromLocalInput("tomorrow")).toBeNull();
  });

  it("wants a custom time at least a minute ahead", () => {
    expect(customTimeProblem(null, TUESDAY)).toBe("Pick a date and time.");
    expect(customTimeProblem(new Date(TUESDAY.getTime() + 30_000), TUESDAY)).toMatch(/minute/);
    expect(customTimeProblem(new Date(TUESDAY.getTime() + 120_000), TUESDAY)).toBeNull();
  });
});
