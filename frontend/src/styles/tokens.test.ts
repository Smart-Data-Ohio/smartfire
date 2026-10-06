// @vitest-environment node

import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { contrastRatio, oklchToRgb, parseOklch, type Rgb } from "../lib/color.ts";

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

/** Resolves var() and light-dark() chains down to one oklch() colour for the theme. */
function resolve(value: string, theme: Theme): string {
  const variable = /^var\((--[\w-]+)\)$/.exec(value);

  if (variable !== null) {
    const name = variable[1] ?? "";
    const next = TOKENS.get(name);

    if (next === undefined) {
      throw new Error(`unknown token ${name}`);
    }

    return resolve(next, theme);
  }

  if (value.startsWith("light-dark(") && value.endsWith(")")) {
    const [light, dark] = splitPair(value.slice("light-dark(".length, -1));

    return resolve(theme === "light" ? light : dark, theme);
  }

  return value;
}

function color(token: string, theme: Theme): Rgb {
  const resolved = resolve(`var(${token})`, theme);
  const parsed = parseOklch(resolved);

  if (parsed === null || parsed.alpha !== 1) {
    throw new Error(`${token} is not an opaque oklch() colour in ${theme}: ${resolved}`);
  }

  return oklchToRgb(parsed);
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
  ["--mention-text", ["--mention-bg", "--bg-pane", "--bg-raised", "--bg-sidebar"]],
  ["--danger-text", ["--bg-pane", "--bg-raised", "--bg-hover", "--danger-soft"]],
  ["--success-text", ["--bg-pane", "--bg-raised", "--bg-sidebar"]],
  ["--warning-text", ["--bg-pane", "--bg-raised", "--bg-sidebar"]],
  ["--agent-text", ["--bg-pane", "--bg-raised", "--bg-hover", "--agent-soft"]],
  ["--tooltip-text", ["--tooltip-bg"]],
  ["--on-accent", ["--accent-solid"]],
  ["--on-danger", ["--danger-solid"]],
]);

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
    const ratio = contrastRatio(color(text, theme), color(background, theme));

    expect(
      ratio,
      `${text} on ${background} (${theme}) is ${ratio.toFixed(2)}:1`,
    ).toBeGreaterThanOrEqual(4.5);
  });
});
