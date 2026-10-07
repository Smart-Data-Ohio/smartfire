import { describe, expect, it } from "vitest";
import { arrivals, departures, withLeaving } from "./list-motion.ts";

describe("arrivals", () => {
  it("animates only new rows above the first known one", () => {
    expect(arrivals([3, 2, 1], [5, 4, 3, 2, 1])).toEqual([5, 4]);
    expect(arrivals([3, 2, 1], [3, 2, 1, 0])).toEqual([]);
  });

  it("animates nothing for a big batch", () => {
    const fresh = Array.from({ length: 30 }, (_, index) => 100 + index);

    expect(arrivals([1], [...fresh, 1])).toEqual([]);
  });
});

describe("departures and withLeaving", () => {
  it("keeps a leaving row in its place while it exits", () => {
    const keyOf = (value: number) => value;
    const gone = departures([3, 2, 1], [3, 1], keyOf);

    expect(gone).toEqual([{ key: 2, value: 2, after: 3 }]);

    const rows = [3, 1].map((value) => ({ key: value, value, motion: undefined }));

    expect(withLeaving(rows, gone).map((row) => [row.key, row.motion])).toEqual([
      [3, undefined],
      [2, "leave"],
      [1, undefined],
    ]);
  });

  it("puts a leaving first row back at the top", () => {
    const gone = departures([3, 2], [2], (value: number) => value);
    const rows = [{ key: 2, value: 2, motion: undefined }];

    expect(withLeaving(rows, gone)[0]).toEqual({ key: 3, value: 3, motion: "leave" });
  });
});
