import { contrastRatio, fitGamut, type Oklch, oklchToRgb } from "./color.ts";

/**
 * Colour palettes a person can pick for this device (Settings → Appearance). A palette is a seed,
 * a tint for the surfaces and a hue for the accent, from which the semantic colour tokens are
 * derived in OKLCH: the surfaces keep tokens.css's lightness steps, so depth reads the same in
 * every palette, and each text token is then moved darker (light theme) or lighter (dark theme)
 * until it holds 4.5:1 on every surface it sits on. The tokens are set on <html> as `light-dark()`
 * pairs, so the theme still flips with one property. src/styles/tokens.test.ts holds every palette
 * to the same contrast pairs as the stylesheet.
 */
export type PalettePreset = "smartfire" | "graphite" | "ocean" | "forest" | "ember" | "rose";

interface Tint {
  readonly hue: number;
  readonly chroma: number;
}

export interface PaletteSeed {
  readonly surface: Tint;
  readonly accent: Tint;
}

export interface Palette {
  readonly value: PalettePreset;
  readonly label: string;
  /** `null`: the stylesheet's own colours. */
  readonly seed: PaletteSeed | null;
}

/** Violet (hues 280-320) is kept for agents, so no accent sits there. */
export const PALETTES: readonly Palette[] = [
  { value: "smartfire", label: "Smartfire", seed: null },
  {
    value: "graphite",
    label: "Graphite",
    seed: { surface: { hue: 260, chroma: 0 }, accent: { hue: 250, chroma: 0.06 } },
  },
  {
    value: "ocean",
    label: "Ocean",
    seed: { surface: { hue: 230, chroma: 0.014 }, accent: { hue: 225, chroma: 0.14 } },
  },
  {
    value: "forest",
    label: "Forest",
    seed: { surface: { hue: 160, chroma: 0.012 }, accent: { hue: 155, chroma: 0.13 } },
  },
  {
    value: "ember",
    label: "Ember",
    seed: { surface: { hue: 60, chroma: 0.012 }, accent: { hue: 50, chroma: 0.16 } },
  },
  {
    value: "rose",
    label: "Rose",
    seed: { surface: { hue: 10, chroma: 0.012 }, accent: { hue: 0, chroma: 0.16 } },
  },
];

/** tokens.css's own value for every token a palette replaces (the test keeps them equal). */
export const DEFAULT_PALETTE_TOKENS: ReadonlyMap<string, string> = new Map([
  ["--bg-app", "light-dark(var(--neutral-50), var(--charcoal-950))"],
  ["--bg-sidebar", "light-dark(var(--neutral-100), var(--charcoal-900))"],
  ["--bg-pane", "light-dark(var(--neutral-0), var(--charcoal-850))"],
  ["--bg-raised", "light-dark(var(--neutral-0), var(--charcoal-800))"],
  ["--bg-hover", "light-dark(var(--neutral-150), var(--charcoal-800))"],
  ["--bg-active", "light-dark(var(--neutral-200), var(--charcoal-750))"],
  ["--bg-selected", "light-dark(var(--indigo-selected-light), var(--indigo-selected-dark))"],
  ["--bg-sunken", "light-dark(var(--neutral-150), var(--charcoal-950))"],
  ["--bg-skeleton", "light-dark(var(--neutral-200), var(--charcoal-750))"],
  ["--text", "light-dark(var(--neutral-900), var(--charcoal-100))"],
  ["--text-muted", "light-dark(var(--neutral-600), var(--charcoal-300))"],
  ["--text-faint", "light-dark(var(--neutral-500), var(--charcoal-400))"],
  ["--border", "light-dark(var(--neutral-250), var(--charcoal-750))"],
  ["--border-strong", "light-dark(var(--neutral-300), var(--charcoal-700))"],
  ["--accent", "light-dark(var(--indigo-500), var(--indigo-300))"],
  ["--accent-solid", "light-dark(var(--indigo-500), var(--indigo-450))"],
  ["--accent-soft", "light-dark(var(--indigo-soft-light), var(--indigo-soft-dark))"],
]);

export const PALETTE_TOKEN_NAMES: readonly string[] = [...DEFAULT_PALETTE_TOKENS.keys()];

type Theme = "light" | "dark";

/** Surface lightness steps, as tokens.css has them (light, dark). */
const SURFACE_STEPS: ReadonlyMap<string, readonly [number, number]> = new Map([
  ["--bg-app", [0.985, 0.17]],
  ["--bg-sidebar", [0.965, 0.2]],
  ["--bg-pane", [0.995, 0.23]],
  ["--bg-raised", [1, 0.27]],
  ["--bg-hover", [0.95, 0.27]],
  ["--bg-active", [0.925, 0.3]],
  ["--bg-sunken", [0.95, 0.17]],
  ["--bg-skeleton", [0.925, 0.3]],
  ["--border", [0.91, 0.3]],
  ["--border-strong", [0.86, 0.34]],
]);

/** The surfaces text sits on (tokens.test.ts's SURFACES). */
const TEXT_SURFACES = [
  "--bg-app",
  "--bg-sidebar",
  "--bg-pane",
  "--bg-raised",
  "--bg-hover",
  "--bg-active",
  "--bg-selected",
  "--bg-sunken",
];

/** Above 4.5:1 by a hair, so a browser's rounding never lands a token just under it. */
const TARGET_CONTRAST = 4.55;

const WHITE: Oklch = { l: 1, c: 0, h: 0, alpha: 1 };

/** The colour as it will be written: rounded to the CSS precision and inside sRGB. */
function settle(color: Oklch): Oklch {
  const l = Math.min(1, Math.max(0, Math.round(color.l * 1000) / 1000));
  const h = Math.round(color.h);
  const fitted = fitGamut({ l, c: color.c, h, alpha: 1 });

  return { l, c: Math.floor(fitted.c * 1000) / 1000, h, alpha: 1 };
}

function format(color: Oklch): string {
  return `oklch(${(color.l * 100).toFixed(1)}% ${color.c.toFixed(3)} ${color.h})`;
}

function weakest(color: Oklch, backgrounds: readonly Oklch[]): number {
  const ink = oklchToRgb(color);

  return Math.min(...backgrounds.map((background) => contrastRatio(ink, oklchToRgb(background))));
}

/**
 * Moves `color` darker or lighter, in small lightness steps, until it holds 4.5:1 on every one of
 * `backgrounds`; a colour that already holds comes back as it was.
 */
export function clampLightness(
  color: Oklch,
  backgrounds: readonly Oklch[],
  direction: "darker" | "lighter",
): Oklch {
  const step = direction === "darker" ? -0.005 : 0.005;
  let candidate = settle(color);

  while (weakest(candidate, backgrounds) < TARGET_CONTRAST) {
    const l = candidate.l + step;

    if (l < 0 || l > 1) {
      break;
    }

    candidate = settle({ ...candidate, l });
  }

  return candidate;
}

function lightOrDark(theme: Theme, light: number, dark: number): number {
  return theme === "light" ? light : dark;
}

function derive(seed: PaletteSeed, theme: Theme): ReadonlyMap<string, Oklch> {
  const tokens = new Map<string, Oklch>();
  // Dark surfaces carry a touch more tint, which reads as the same mood at low lightness.
  const surfaceChroma = seed.surface.chroma * lightOrDark(theme, 1, 1.25);
  const accent = seed.accent;

  for (const [name, [light, dark]] of SURFACE_STEPS) {
    // An untinted light pane stays pure white, as Smartfire's own; a tinted one carries its tint.
    const white = theme === "light" && name === "--bg-pane" && surfaceChroma === 0;
    const l = white ? 1 : lightOrDark(theme, light, dark);

    tokens.set(name, settle({ l, c: surfaceChroma, h: seed.surface.hue, alpha: 1 }));
  }

  const soft = settle({
    l: lightOrDark(theme, 0.95, 0.3),
    c: lightOrDark(theme, 0.03, 0.06),
    h: accent.hue,
    alpha: 1,
  });

  tokens.set("--accent-soft", soft);
  tokens.set(
    "--bg-selected",
    settle({
      l: lightOrDark(theme, 0.92, 0.32),
      c: lightOrDark(theme, 0.02, 0.03),
      h: accent.hue,
      alpha: 1,
    }),
  );

  const surfaces = TEXT_SURFACES.map((name) => tokens.get(name) ?? WHITE);
  const direction = theme === "light" ? "darker" : "lighter";
  const ink = (l: number) => ({
    l,
    c: Math.min(surfaceChroma * 2, 0.02),
    h: seed.surface.hue,
    alpha: 1,
  });

  tokens.set("--text", clampLightness(ink(lightOrDark(theme, 0.22, 0.94)), surfaces, direction));
  tokens.set("--text-muted", clampLightness(ink(lightOrDark(theme, 0.46, 0.74)), surfaces, direction));
  tokens.set("--text-faint", clampLightness(ink(lightOrDark(theme, 0.51, 0.69)), surfaces, direction));
  tokens.set(
    "--accent",
    clampLightness(
      { l: lightOrDark(theme, 0.52, 0.72), c: accent.chroma, h: accent.hue, alpha: 1 },
      [...surfaces, soft],
      direction,
    ),
  );
  // White text sits on the solid accent (buttons) in both themes.
  tokens.set(
    "--accent-solid",
    clampLightness(
      { l: lightOrDark(theme, 0.52, 0.56), c: accent.chroma, h: accent.hue, alpha: 1 },
      [WHITE],
      "darker",
    ),
  );

  return tokens;
}

const derived = new Map<PalettePreset, ReadonlyMap<string, string>>();

/**
 * The tokens `preset` sets on <html>, as `light-dark(light, dark)` pairs; none for Smartfire's
 * own palette.
 */
export function paletteTokens(preset: PalettePreset): ReadonlyMap<string, string> {
  const known = derived.get(preset);

  if (known !== undefined) {
    return known;
  }

  const seed = PALETTES.find((palette) => palette.value === preset)?.seed ?? null;
  const tokens = new Map<string, string>();

  if (seed !== null) {
    const light = derive(seed, "light");
    const dark = derive(seed, "dark");

    for (const name of PALETTE_TOKEN_NAMES) {
      tokens.set(name, `light-dark(${format(light.get(name) ?? WHITE)}, ${format(dark.get(name) ?? WHITE)})`);
    }
  }

  derived.set(preset, tokens);

  return tokens;
}
