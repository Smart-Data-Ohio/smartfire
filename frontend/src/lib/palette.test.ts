// @vitest-environment node

import { describe, expect, it } from "vitest";
import { contrastRatio, inGamut, oklchToRgb, parseOklch } from "./color.ts";
import { clampLightness, PALETTE_TOKEN_NAMES, PALETTES, paletteTokens } from "./palette.ts";

/** The two halves of `light-dark(a, b)`. */
function halves(value: string): readonly [string, string] {
  const match = /^light-dark\((oklch\([^)]*\)), (oklch\([^)]*\))\)$/.exec(value);

  if (match === null) {
    throw new Error(`not a light-dark() oklch pair: ${value}`);
  }

  return [match[1] ?? "", match[2] ?? ""];
}

describe("palettes", () => {
  it("starts with Smartfire's own colours, which change nothing", () => {
    expect(PALETTES[0]?.value).toBe("smartfire");
    expect(paletteTokens("smartfire").size).toBe(0);
  });

  it("offers more than one hue, and none in the violet kept for agents", () => {
    const hues = PALETTES.flatMap((palette) => (palette.seed === null ? [] : [palette.seed.accent.hue]));

    expect(hues.length).toBeGreaterThanOrEqual(4);

    for (const hue of hues) {
      expect(hue < 280 || hue > 320, `accent hue ${hue}`).toBe(true);
    }
  });

  it.each(PALETTES.filter((palette) => palette.seed !== null))(
    "$label sets every palette token to an in-gamut light/dark pair",
    ({ value }) => {
      const tokens = paletteTokens(value);

      expect([...tokens.keys()].sort()).toEqual([...PALETTE_TOKEN_NAMES].sort());

      for (const [name, pair] of tokens) {
        for (const half of halves(pair)) {
          const color = parseOklch(half);

          expect(color, `${name}: ${half}`).not.toBeNull();
          expect(color !== null && inGamut(color), `${name}: ${half} is outside sRGB`).toBe(true);
        }
      }
    },
  );
});

describe("clampLightness", () => {
  const white = { l: 1, c: 0, h: 0, alpha: 1 };
  const black = { l: 0, c: 0, h: 0, alpha: 1 };

  it("darkens a pale text until it holds 4.5:1 on every background", () => {
    const pale = { l: 0.8, c: 0.1, h: 40, alpha: 1 };
    const clamped = clampLightness(pale, [white, { l: 0.95, c: 0.01, h: 40, alpha: 1 }], "darker");

    expect(clamped.l).toBeLessThan(pale.l);
    expect(contrastRatio(oklchToRgb(clamped), oklchToRgb(white))).toBeGreaterThanOrEqual(4.5);
  });

  it("lightens on dark backgrounds, and leaves a colour that already holds alone", () => {
    const dim = { l: 0.4, c: 0.1, h: 230, alpha: 1 };

    expect(clampLightness(dim, [black], "lighter").l).toBeGreaterThan(dim.l);

    const strong = { l: 0.2, c: 0, h: 0, alpha: 1 };

    expect(clampLightness(strong, [white], "darker")).toEqual(strong);
  });
});
