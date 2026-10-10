// @vitest-environment node

import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import {
  CONTRAST_PAIRS,
  contrastReport,
  effectiveColours,
  formatRatio,
  normalizeHex,
  PALETTE_FIELDS,
  parseColour,
  presetColours,
  resetField,
  sameTokens,
  setFieldColour,
  tokensPayload,
} from "./custom-palette.ts";
import { PALETTES } from "./palette.ts";

const field = (token: string) => {
  const found = PALETTE_FIELDS.find((candidate) => candidate.token === token);

  if (found === undefined) throw new Error(`no field ${token}`);

  return found;
};

const PRESET_CASES = PALETTES.flatMap((palette) =>
  (["light", "dark"] as const).map((theme) => ({ palette: palette.value, theme })),
);

describe("colour parsing", () => {
  it("reads the hex and rgb forms the server accepts", () => {
    expect(parseColour("#abc")).toEqual({ rgb: { r: 170, g: 187, b: 204 }, alpha: 1 });
    expect(parseColour("#4F46E5")).toEqual({ rgb: { r: 79, g: 70, b: 229 }, alpha: 1 });
    expect(parseColour("#00000080")?.alpha).toBeCloseTo(0.5, 2);
    expect(parseColour("rgb(0, 255, 10)")).toEqual({ rgb: { r: 0, g: 255, b: 10 }, alpha: 1 });
    expect(parseColour("rgb(10% 20% 30% / 50%)")).toEqual({
      rgb: { r: 26, g: 51, b: 77 },
      alpha: 0.5,
    });
    expect(parseColour("rgba(10, 20, 30, .5)")?.alpha).toBe(0.5);
  });

  it("refuses what the server refuses", () => {
    for (const value of ["red", "#12", "#12345", "rgb(256, 0, 0)", "rgb(1 / 2 3)", "rgb(1, 2)"]) {
      expect(parseColour(value), value).toBeNull();
    }
  });

  it("normalizes typed hex to #rrggbb", () => {
    expect(normalizeHex("ABC")).toBe("#aabbcc");
    expect(normalizeHex(" #4F46E5 ")).toBe("#4f46e5");
    expect(normalizeHex("#4f46e")).toBeNull();
    expect(normalizeHex("blue")).toBeNull();
  });
});

describe("contrast", () => {
  it("measures WCAG 2 ratios and shows them rounded down", () => {
    const colours = effectiveColours(
      { "--text": "#000000", "--bg-pane": "#ffffff" },
      presetColours("smartfire", "light"),
    );

    const text = contrastReport(colours).find(
      (result) => result.foreground === "--text" && result.background === "--bg-pane",
    );

    expect(text?.ratio).toBeCloseTo(21, 5);
    expect(text?.label).toBe("Primary text on messages");
    expect(formatRatio(4.4999)).toBe("4.4:1");
    expect(formatRatio(4.5)).toBe("4.5:1");
  });

  it.each(PRESET_CASES)(
    "raises no warning for the $palette preset in $theme",
    ({ palette, theme }) => {
      const low = contrastReport(effectiveColours({}, presetColours(palette, theme))).filter(
        (result) => !result.passes,
      );

      expect(low.map((result) => `${result.label} ${formatRatio(result.ratio)}`)).toEqual([]);
    },
  );

  it("warns below 4.5:1 for text and below 3:1 for the accent against the pane", () => {
    const report = contrastReport(
      effectiveColours(
        { "--text-muted": "#999999", "--accent-solid": "#d0d0ff" },
        presetColours("smartfire", "light"),
      ),
    );

    const low = report.filter((result) => !result.passes);

    expect(low.map((result) => [result.foreground, result.background, result.minimum])).toEqual([
      ["--text-muted", "--bg-pane", 4.5],
      ["--text-muted", "--bg-sidebar", 4.5],
      ["--on-accent", "--accent-solid", 4.5],
      ["--accent-solid", "--bg-pane", 3],
    ]);
    expect(CONTRAST_PAIRS.filter((pair) => pair.minimum === 3)).toHaveLength(1);
  });

  it("paints a translucent custom colour over the messages pane", () => {
    const colours = effectiveColours(
      { "--bg-pane": "#000000", "--mention-bg": "#ffffff80" },
      presetColours("smartfire", "light"),
    );

    expect(colours.get("--mention-bg")).toEqual({ r: 128, g: 128, b: 128 });
  });
});

describe("preset colours", () => {
  it("starts from the stylesheet's colours for Smartfire and the derived ones for a preset", () => {
    const light = presetColours("smartfire", "light");

    expect(light.get("--bg-pane")).toBe("#ffffff");
    expect(light.get("--on-accent")).toBe("#ffffff");
    expect(presetColours("smartfire", "dark").get("--bg-pane")).not.toBe("#ffffff");
    expect(presetColours("ocean", "light").get("--accent-solid")).not.toBe(
      light.get("--accent-solid"),
    );

    for (const value of light.values()) expect(value).toMatch(/^#[\da-f]{6}$/);
  });
});

describe("editing tokens", () => {
  it("sets and resets a colour with its companion tokens", () => {
    const danger = field("--danger-text");
    const edited = setFieldColour({ "--accent": "#123456" }, danger, "#ff0000");

    expect(edited).toEqual({
      "--accent": "#123456",
      "--danger-text": "#ff0000",
      "--danger": "#ff0000",
    });
    expect(resetField(edited, danger)).toEqual({ "--accent": "#123456" });
    expect(sameTokens(resetField(edited, danger), { "--accent": "#123456" })).toBe(true);
    expect(sameTokens(edited, { "--accent": "#123456" })).toBe(false);
  });

  it("saves the colours in key order, and no colours as a clear", () => {
    expect(tokensPayload({ "--text": "#111111", "--accent-solid": "#222222" })).toEqual({
      "--accent-solid": "#222222",
      "--text": "#111111",
    });
    expect(Object.keys(tokensPayload({ "--text": "#1", "--accent": "#2" }) ?? {})).toEqual([
      "--accent",
      "--text",
    ]);
    expect(tokensPayload({})).toBeNull();
  });

  it("offers only tokens the server allows", () => {
    const rust = readFileSync(
      new URL("../../../crates/db/src/models/user/profile_settings.rs", import.meta.url),
      "utf8",
    );

    const allowed = rust.split("pub const APPEARANCE_TOKENS")[1]?.split("];")[0] ?? "";

    for (const { token, also } of PALETTE_FIELDS) {
      for (const name of [token, ...(also ?? [])]) expect(allowed, name).toContain(`"${name}"`);
    }
  });
});
