import { PALETTES, paletteTokens } from "./palette.ts";

/** Where index.html's blocking script takes every palette's tokens (vite.config.ts fills it). */
export const PALETTE_TABLE_MARKER = "{/*palettes*/}";

/**
 * `html` with each palette's tokens (Smartfire's own palette has none) in place of the marker in
 * its blocking script: the very values src/lib/appearance.ts sets when the SPA starts, so the
 * first paint and the hydrated page match, and an upgrade's palettes apply from its first paint.
 */
export function inlinePaletteTable(html: string): string {
  if (!html.includes(PALETTE_TABLE_MARKER)) {
    throw new Error(`index.html has no ${PALETTE_TABLE_MARKER} marker for the palette table`);
  }

  const table = Object.fromEntries(
    PALETTES.filter((palette) => palette.seed !== null).map((palette) => [
      palette.value,
      Object.fromEntries(paletteTokens(palette.value)),
    ]),
  );

  // Inside a <script>: no "<", so no "</script>" or "<!--".
  const json = JSON.stringify(table).replaceAll("<", "\\u003c");

  return html.replace(PALETTE_TABLE_MARKER, () => json);
}
