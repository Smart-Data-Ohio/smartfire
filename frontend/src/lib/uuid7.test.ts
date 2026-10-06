import { describe, expect, it } from "vitest";
import { uuid7 } from "./uuid7.ts";

describe("uuid7", () => {
  it("is a version 7, RFC 9562 variant UUID carrying the time", () => {
    const id = uuid7(Date.UTC(2026, 9, 6, 12, 0, 0));

    expect(id).toMatch(/^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/);
    expect(Number.parseInt(id.replace("-", "").slice(0, 12), 16)).toBe(Date.UTC(2026, 9, 6, 12));
  });

  it("sorts by time", () => {
    const ids = [3000, 1000, 2000].map((at) => uuid7(at));

    expect(ids.toSorted()).toEqual([ids[1], ids[2], ids[0]]);
  });
});
