import { describe, expect, it } from "vitest";
import { BANNER_EXTRA, shouldFold } from "./use-banner-fold.ts";

const LONG = 2000;

const BOX = 600;

describe("shouldFold", () => {
  it("folds once a long list scrolls past a few pixels, and unfolds at the top", () => {
    expect(shouldFold(false, 4, LONG, BOX)).toBe(false);

    expect(shouldFold(false, 40, LONG, BOX)).toBe(true);

    expect(shouldFold(true, 4, LONG, BOX)).toBe(true);

    expect(shouldFold(true, 0, LONG, BOX)).toBe(false);
  });

  it("never folds a list too short to keep scrolling once the banner is gone", () => {
    expect(shouldFold(false, 40, BOX + BANNER_EXTRA, BOX)).toBe(false);
  });
});
