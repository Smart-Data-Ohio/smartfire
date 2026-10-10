// @vitest-environment node

import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import {
  CONTRAST_PAIRS,
  type ColourReader,
  type ContrastResult,
  type CustomTokens,
  contrastReport,
  formatRatio,
  normalizeHex,
  PALETTE_FIELDS,
  parseColour,
  presetColours,
  resetField,
  sameTokens,
  setFieldColour,
  themeColours,
  tokensPayload,
} from "./custom-palette.ts";
import { PALETTES, type PalettePreset } from "./palette.ts";

const field = (token: string) => {
  const found = PALETTE_FIELDS.find((candidate) => candidate.token === token);

  if (found === undefined) throw new Error(`no field ${token}`);

  return found;
};

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

/** The editor's colours in both themes: `tokens` over the preset (or what `read` finds). */
function both(tokens: CustomTokens, preset: PalettePreset = "smartfire", read?: ColourReader) {
  return themeColours(tokens, {
    light: presetColours(preset, "light", read),
    dark: presetColours(preset, "dark", read),
  });
}

const pairOf = (report: readonly ContrastResult[], foreground: string, background: string) =>
  report.find((result) => result.foreground === foreground && result.background === background);

describe("contrast", () => {
  it("measures WCAG 2 ratios and shows them rounded down", () => {
    const text = pairOf(
      contrastReport(both({ "--text": "#000000", "--bg-pane": "#ffffff" })),
      "--text",
      "--bg-pane",
    );

    expect(text?.ratio).toBeCloseTo(21, 5);
    expect(text?.label).toBe("Primary text on messages");
    expect(formatRatio(4.4999)).toBe("4.4:1");
    expect(formatRatio(4.5)).toBe("4.5:1");
  });

  it.each(PALETTES.map((palette) => ({ palette: palette.value })))(
    "raises no warning for the $palette preset in either theme",
    ({ palette }) => {
      const low = contrastReport(both({}, palette)).filter((result) => !result.passes);

      expect(low.map((result) => `${result.label} ${formatRatio(result.ratio)}`)).toEqual([]);
    },
  );

  it("warns below 4.5:1 for text and below 3:1 for the accent against the pane", () => {
    const report = contrastReport(both({ "--text-muted": "#999999", "--accent-solid": "#d0d0ff" }));

    const low = report.filter((result) => !result.passes);

    expect(
      low.map((result) => [result.foreground, result.background, result.minimum, result.failing]),
    ).toEqual([
      ["--text-muted", "--bg-pane", 4.5, ["light"]],
      ["--text-muted", "--bg-sidebar", 4.5, ["light"]],
      ["--on-accent", "--accent-solid", 4.5, ["light", "dark"]],
      ["--accent-solid", "--bg-pane", 3, ["light"]],
    ]);
    expect(CONTRAST_PAIRS.filter((pair) => pair.minimum === 3)).toHaveLength(1);
  });

  it("checks a custom colour against the other theme's background too, and names the theme", () => {
    const report = contrastReport(both({ "--text": "#111111" }));
    const pane = pairOf(report, "--text", "--bg-pane");

    expect(pane?.ratios.light).toBeGreaterThan(15);
    expect(pane?.ratios.dark).toBeLessThan(1.5);
    expect(pane?.failing).toEqual(["dark"]);
    expect(pane?.passes).toBe(false);
    expect(pane?.ratio).toBe(pane?.ratios.dark);
    expect(pairOf(report, "--on-accent", "--accent-solid")?.failing).toEqual([]);
  });

  it("measures against the colours the page computes, workspace CSS included", () => {
    // Workspace CSS `:root { --bg-pane: #000000; }`: the computed pane is black in both themes.
    const read: ColourReader = (token) => (token === "--bg-pane" ? "rgb(0, 0, 0)" : null);
    const report = contrastReport(both({ "--text": "#000000" }, "smartfire", read));

    expect(pairOf(report, "--text", "--bg-pane")?.ratio).toBeCloseTo(1, 5);
    expect(presetColours("smartfire", "light", read).get("--bg-pane")).toBe("#000000");
    // Unreadable answers fall back to the palette's own colour.
    expect(presetColours("smartfire", "light", read).get("--bg-app")).toBe(
      presetColours("smartfire", "light").get("--bg-app"),
    );
  });

  it("reads computed oklch and color(srgb) values, with their alpha", () => {
    const read: ColourReader = (token) =>
      ({
        "--bg-sidebar": "oklch(0 0 0)",
        "--bg-raised": "color(srgb 1 1 1)",
        "--mention-bg": "oklch(1 0 0 / 0.5)",
        "--bg-pane": "rgb(0, 0, 0)",
      })[token] ?? null;

    const light = presetColours("smartfire", "light", read);

    expect(light.get("--bg-sidebar")).toBe("#000000");
    expect(light.get("--bg-raised")).toBe("#ffffff");
    expect(light.get("--mention-bg")).toBe("#808080");
  });

  it("paints a translucent custom colour over the messages pane", () => {
    const colours = both({ "--bg-pane": "#000000", "--mention-bg": "#ffffff80" });

    expect(colours.light.get("--mention-bg")).toEqual({ r: 128, g: 128, b: 128 });
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
