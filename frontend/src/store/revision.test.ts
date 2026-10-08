import { describe, expect, it } from "vitest";
import { landsOver } from "./revision.ts";

describe("the server revision comparison", () => {
  const held = { updatedAt: "2026-10-07T10:15:00.123456Z" };

  it.each([
    ["2026-10-07T10:15:00.123455Z", false],
    ["2026-10-07T10:15:00.123456Z", true],
    ["2026-10-07T10:15:00.123457Z", true],
  ])("compares microseconds and lands ties: %s", (updatedAt, expected) => {
    expect(landsOver(held, { updatedAt })).toBe(expected);
  });

  it("lands a first copy", () => {
    expect(landsOver(undefined, held)).toBe(true);
    expect(landsOver(null, held)).toBe(true);
  });
});
