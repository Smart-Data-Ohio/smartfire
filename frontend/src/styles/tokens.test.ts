// @vitest-environment node

import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import {
  composite,
  contrastRatio,
  type Oklch,
  oklchToRgb,
  parseOklch,
  type Rgb,
} from "../lib/color.ts";
import { DEFAULT_PALETTE_TOKENS, PALETTES, paletteTokens } from "../lib/palette.ts";
import { AVATAR_HUES } from "../ui/avatar-palette.ts";

type Theme = "light" | "dark";

/** Every `--name: value;` in the first `:root` block (the primitives and the semantic tokens). */
function readTokens(css: string): Map<string, string> {
  const start = css.indexOf(":root {");
  const end = css.indexOf("\n  }", start);
  const block = css.slice(start, end).replaceAll(/\/\*[\s\S]*?\*\//g, "");
  const tokens = new Map<string, string>();

  for (const match of block.matchAll(/(--[\w-]+)\s*:\s*([^;]+);/g)) {
    const [, name = "", value = ""] = match;

    tokens.set(name, value.trim().replaceAll(/\s+/g, " "));
  }

  return tokens;
}

/** Splits `a, b` at its top-level comma (inside `light-dark(…)`). */
function splitPair(value: string): readonly [string, string] {
  let depth = 0;

  for (const [index, character] of [...value].entries()) {
    if (character === "(") {
      depth += 1;
    } else if (character === ")") {
      depth -= 1;
    } else if (character === "," && depth === 0) {
      return [value.slice(0, index).trim(), value.slice(index + 1).trim()];
    }
  }

  throw new Error(`expected a pair in ${value}`);
}

// Read from disk: Vitest stubs CSS imports, `?raw` included, unless CSS processing is on.
const TOKENS = readTokens(readFileSync(new URL("./tokens.css", import.meta.url), "utf8"));

/**
 * Resolves var() and light-dark() chains down to one oklch() colour for the theme, reading
 * `tokens` (tokens.css, or tokens.css under a palette's overrides).
 */
function resolve(value: string, theme: Theme, tokens: ReadonlyMap<string, string> = TOKENS): string {
  const variable = /^var\((--[\w-]+)\)$/.exec(value);

  if (variable !== null) {
    const name = variable[1] ?? "";
    const next = tokens.get(name);

    if (next === undefined) {
      throw new Error(`unknown token ${name}`);
    }

    return resolve(next, theme, tokens);
  }

  if (value.startsWith("light-dark(") && value.endsWith(")")) {
    const [light, dark] = splitPair(value.slice("light-dark(".length, -1));

    return resolve(theme === "light" ? light : dark, theme, tokens);
  }

  return value;
}

function parsed(value: string, label: string): Oklch {
  const color = parseOklch(value);

  if (color === null) {
    throw new Error(`${label} is not an oklch() colour: ${value}`);
  }

  return color;
}

function color(token: string, theme: Theme, tokens: ReadonlyMap<string, string> = TOKENS): Rgb {
  const value = parsed(resolve(`var(${token})`, theme, tokens), `${token} (${theme})`);

  if (value.alpha !== 1) {
    throw new Error(`${token} is translucent in ${theme}; test it as "${token} over <surface>"`);
  }

  return oklchToRgb(value);
}

/**
 * A background as painted: one opaque token, or translucent layers over one, written top first
 * ("--mention-chip-bg over --mention-bg over --bg-pane").
 */
function surface(spec: string, theme: Theme, tokens: ReadonlyMap<string, string> = TOKENS): Rgb {
  const layers = spec.split(" over ");
  const base = layers.pop() ?? spec;

  return layers.reduceRight(
    (below, token) => {
      const layer = parsed(resolve(`var(${token})`, theme, tokens), `${token} (${theme})`);

      return composite(oklchToRgb({ ...layer, alpha: 1 }), layer.alpha, below);
    },
    color(base, theme, tokens),
  );
}

/** The surfaces text sits on across the app shell. */
const SURFACES = [
  "--bg-app",
  "--bg-sidebar",
  "--bg-pane",
  "--bg-raised",
  "--bg-hover",
  "--bg-active",
  "--bg-selected",
  "--bg-sunken",
];

/** Text token → the backgrounds it is used on. */
const PAIRS = new Map<string, readonly string[]>([
  ["--text", SURFACES],
  ["--text-muted", SURFACES],
  ["--text-faint", SURFACES],
  ["--accent", [...SURFACES, "--accent-soft"]],
  [
    "--mention-text",
    [
      "--mention-bg over --bg-pane",
      "--mention-chip-bg over --mention-bg over --bg-pane",
      "--bg-pane",
      "--bg-raised",
      "--bg-sidebar",
    ],
  ],
  ["--text", ["--mention-bg over --bg-pane"]],
  ["--danger-text", ["--bg-pane", "--bg-raised", "--bg-hover", "--danger-soft"]],
  ["--success-text", ["--bg-pane", "--bg-raised", "--bg-sidebar"]],
  ["--warning-text", ["--bg-pane", "--bg-raised", "--bg-sidebar"]],
  ["--agent-text", ["--bg-pane", "--bg-raised", "--bg-hover", "--agent-soft"]],
  ["--tooltip-text", ["--tooltip-bg"]],
  ["--on-accent", ["--accent-solid"]],
  ["--on-danger", ["--danger-solid"]],
]);

/** A component's `--name: value;` declaration, read from its stylesheet in src/ui. */
function declaration(file: string, name: string): string {
  const css = readFileSync(new URL(`../ui/${file}`, import.meta.url), "utf8");
  const match = new RegExp(`${name}\\s*:\\s*([^;]+);`).exec(css);

  if (match === null) {
    throw new Error(`${file} has no ${name}`);
  }

  return (match[1] ?? "").trim().replaceAll(/\s+/g, " ");
}

/** The static metal button's ink against every stop of its brushed gradient. */
const METAL_STOPS = ["top", "upper", "mid", "crease", "lower", "bottom"];

const METAL_CASES = (["light", "dark"] as const).flatMap((theme) =>
  METAL_STOPS.map((stop) => ({ theme, stop })),
);

const AVATAR_CASES = (["light", "dark"] as const).flatMap((theme) =>
  AVATAR_HUES.map((hue) => ({ theme, hue })),
);

/** tokens.css as a palette leaves it: its tokens over the stylesheet's. */
const PALETTE_CASES = PALETTES.filter((palette) => palette.seed !== null).flatMap((palette) => {
  const tokens = new Map([...TOKENS, ...paletteTokens(palette.value)]);

  return (["light", "dark"] as const).flatMap((theme) =>
    [...PAIRS].flatMap(([text, backgrounds]) =>
      backgrounds.map((background) => ({ palette: palette.label, tokens, theme, text, background })),
    ),
  );
});

const CASES = (["light", "dark"] as const).flatMap((theme) =>
  [...PAIRS].flatMap(([text, backgrounds]) =>
    backgrounds.map((background) => ({ theme, text, background })),
  ),
);

describe("design tokens", () => {
  it("parses the semantic tokens", () => {
    expect(TOKENS.get("--text")).toBe("light-dark(var(--neutral-900), var(--charcoal-100))");
  });

  it.each(CASES)("$text on $background holds 4.5:1 in $theme", ({ theme, text, background }) => {
    const ratio = contrastRatio(color(text, theme), surface(background, theme));

    expect(
      ratio,
      `${text} on ${background} (${theme}) is ${ratio.toFixed(2)}:1`,
    ).toBeGreaterThanOrEqual(4.5);
  });

  it.each(PALETTE_CASES)(
    "$text on $background holds 4.5:1 in $theme with the $palette palette",
    ({ tokens, theme, text, background }) => {
      const ratio = contrastRatio(color(text, theme, tokens), surface(background, theme, tokens));

      expect(
        ratio,
        `${text} on ${background} (${theme}) is ${ratio.toFixed(2)}:1`,
      ).toBeGreaterThanOrEqual(4.5);
    },
  );

  it("knows the stylesheet's own value for every token a palette replaces", () => {
    // The palette picker paints Smartfire's swatch with these, whatever palette is on the page.
    for (const [name, value] of DEFAULT_PALETTE_TOKENS) {
      expect(value, name).toBe(TOKENS.get(name));
    }
  });

  it.each(AVATAR_CASES)("avatar initials hold 4.5:1 on hue $hue in $theme", ({ theme, hue }) => {
    const tile = (name: string) => {
      const value = resolve(
        declaration("avatar.css", name).replaceAll("var(--avatar-hue)", String(hue)),
        theme,
      );

      return oklchToRgb(parsed(value, `${name} at hue ${hue} (${theme})`));
    };

    const ratio = contrastRatio(tile("--avatar-ink"), tile("--avatar-fill"));

    expect(ratio, `hue ${hue} (${theme}) is ${ratio.toFixed(2)}:1`).toBeGreaterThanOrEqual(4.5);
  });

  it.each(METAL_CASES)(
    "static metal ink holds 4.5:1 on its $stop stop in $theme",
    ({ theme, stop }) => {
      const paint = (name: string) =>
        oklchToRgb(parsed(resolve(declaration("button.css", name), theme), `${name} (${theme})`));

      const ratio = contrastRatio(paint("--metal-ink"), paint(`--metal-${stop}`));

      expect(ratio, `${stop} (${theme}) is ${ratio.toFixed(2)}:1`).toBeGreaterThanOrEqual(4.5);
    },
  );
});
