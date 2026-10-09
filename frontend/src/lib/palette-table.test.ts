// @vitest-environment node

import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { PALETTES, paletteTokens } from "./palette.ts";
import { inlinePaletteTable, PALETTE_TABLE_MARKER } from "./palette-table.ts";

describe("inlinePaletteTable", () => {
  it("puts every palette's own tokens where index.html's blocking script reads them", () => {
    const html = inlinePaletteTable(`<script>const presets = ${PALETTE_TABLE_MARKER};</script>`);
    const json = /const presets = (.*);<\/script>/.exec(html)?.[1] ?? "";
    const table = new Map<string, Record<string, string>>(Object.entries(JSON.parse(json)));

    expect([...table.keys()]).toEqual(
      PALETTES.filter((palette) => palette.seed !== null).map((palette) => palette.value),
    );

    for (const [name, tokens] of table) {
      const preset = PALETTES.find((palette) => palette.value === name)?.value ?? "smartfire";

      // The very values hydration sets, so the page doesn't shift when the SPA starts.
      expect(new Map(Object.entries(tokens))).toEqual(paletteTokens(preset));
    }
  });

  it("refuses a page without the marker rather than ship no palettes", () => {
    expect(() => inlinePaletteTable("<script></script>")).toThrow(/marker|palette/i);
  });

  it("finds the marker once in index.html", () => {
    const page = readFileSync(new URL("../../index.html", import.meta.url), "utf8");

    expect(page.split(PALETTE_TABLE_MARKER)).toHaveLength(2);
  });
});
