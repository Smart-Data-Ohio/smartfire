import { composite, contrastRatio, type Oklch, oklchToRgb, parseOklch, type Rgb } from "./color.ts";
import { type PalettePreset, paletteTokens } from "./palette.ts";

/**
 * A personal palette (Settings → Appearance → Custom palette): the chosen preset with some of its
 * semantic tokens replaced by colours of the person's own, saved to the account as
 * `appearancePreferences.tokens` (`{"--accent": "#4f46e5"}`) and set on <html> after the preset's.
 * A custom colour is one colour, as the server stores it, so it holds in the light and the dark
 * theme alike; the editor starts each token from the preset's colour in the theme on screen.
 */
export type PaletteTheme = "light" | "dark";

/** One colour the editor offers; `also` are tokens written with the same colour. */
export interface PaletteField {
  readonly token: string;
  readonly label: string;
  readonly hint: string;
  readonly also?: readonly string[];
}

export interface PaletteFieldGroup {
  readonly title: string;
  readonly fields: readonly PaletteField[];
}

/** The tokens that carry the UI, from the server's allow-list (profile_settings.rs). */
export const PALETTE_FIELD_GROUPS: readonly PaletteFieldGroup[] = [
  {
    title: "Surfaces",
    fields: [
      { token: "--bg-app", label: "App frame", hint: "Behind the rail and the panes" },
      { token: "--bg-sidebar", label: "Sidebar", hint: "The conversation list" },
      { token: "--bg-pane", label: "Messages", hint: "The conversation and settings panes" },
      { token: "--bg-raised", label: "Menus and cards", hint: "Menus, dialogs and cards" },
      { token: "--bg-selected", label: "Selected row", hint: "The open conversation" },
    ],
  },
  {
    title: "Text",
    fields: [
      { token: "--text", label: "Primary text", hint: "Messages, names and headings" },
      { token: "--text-muted", label: "Secondary text", hint: "Sidebar names, times and hints" },
    ],
  },
  {
    title: "Accent",
    fields: [
      { token: "--accent-solid", label: "Accent", hint: "Buttons, toggles and badges" },
      { token: "--on-accent", label: "Text on accent", hint: "Labels on accent buttons" },
      { token: "--accent", label: "Links and focus", hint: "Links, focus rings, active icons" },
    ],
  },
  {
    title: "Lines and states",
    fields: [
      { token: "--border", label: "Borders", hint: "Dividers and control outlines" },
      { token: "--mention-bg", label: "Mention highlight", hint: "Messages that mention you" },
      { token: "--mention", label: "Mention marker", hint: "The bar beside a mention" },
      {
        token: "--danger-text",
        label: "Danger",
        hint: "Errors and destructive actions",
        also: ["--danger"],
      },
      {
        token: "--success-text",
        label: "Success",
        hint: "Confirmations and done states",
        also: ["--success"],
      },
    ],
  },
];

export const PALETTE_FIELDS: readonly PaletteField[] = PALETTE_FIELD_GROUPS.flatMap(
  (group) => group.fields,
);

/**
 * tokens.css's colour for each token the editor shows (light, dark), resolved to its primitive;
 * src/styles/tokens.test.ts keeps them equal to the stylesheet.
 */
export const STYLESHEET_COLOURS: ReadonlyMap<string, readonly [string, string]> = new Map([
  ["--bg-app", ["oklch(98.5% 0.002 250)", "oklch(17% 0.006 260)"]],
  ["--bg-sidebar", ["oklch(96.5% 0.004 250)", "oklch(20% 0.007 260)"]],
  ["--bg-pane", ["oklch(100% 0 0)", "oklch(23% 0.007 260)"]],
  ["--bg-raised", ["oklch(100% 0 0)", "oklch(27% 0.008 260)"]],
  ["--bg-selected", ["oklch(92% 0.02 265)", "oklch(32% 0.03 265)"]],
  ["--text", ["oklch(22% 0.01 260)", "oklch(94% 0.005 260)"]],
  ["--text-muted", ["oklch(46% 0.01 260)", "oklch(74% 0.01 260)"]],
  ["--accent-solid", ["oklch(52% 0.19 268)", "oklch(56% 0.19 268)"]],
  ["--on-accent", ["oklch(100% 0 0)", "oklch(100% 0 0)"]],
  ["--accent", ["oklch(52% 0.19 268)", "oklch(72% 0.14 268)"]],
  ["--border", ["oklch(91% 0.004 250)", "oklch(30% 0.008 260)"]],
  ["--mention-bg", ["oklch(97% 0.04 90)", "oklch(82% 0.12 85 / 8%)"]],
  ["--mention", ["oklch(80% 0.13 85)", "oklch(82% 0.12 85)"]],
  ["--danger-text", ["oklch(53% 0.2 27)", "oklch(72% 0.16 25)"]],
  ["--success-text", ["oklch(50% 0.13 150)", "oklch(76% 0.15 150)"]],
]);

const LIGHT_DARK = /^light-dark\((oklch\([^)]*\)), (oklch\([^)]*\))\)$/;

const BLACK: Oklch = { l: 0, c: 0, h: 0, alpha: 1 };

/** The preset's own colour for `token` in `theme`, before any custom colour. */
function presetOklch(token: string, preset: PalettePreset, theme: PaletteTheme): Oklch {
  const pair = paletteTokens(preset).get(token);
  const match = pair === undefined ? null : LIGHT_DARK.exec(pair);
  const [light, dark] = match === null ? (STYLESHEET_COLOURS.get(token) ?? []) : match.slice(1);

  return parseOklch((theme === "light" ? light : dark) ?? "") ?? BLACK;
}

/** `#rrggbb` for an sRGB colour. */
export function toHex({ r, g, b }: Rgb): string {
  return `#${[r, g, b].map((channel) => channel.toString(16).padStart(2, "0")).join("")}`;
}

/**
 * The colour the page computes for `token` in `theme` without any custom colour (the palette over
 * the stylesheet, under workspace CSS), as `getComputedStyle` gives it; null when it can't tell.
 */
export type ColourReader = (token: string, theme: PaletteTheme) => string | null;

/**
 * The colour every token the editor shows starts from, as `#rrggbb` in `theme`: what `read`
 * finds on the page, else the preset's own. A translucent one (the dark theme's mention wash) is
 * shown as painted over the messages pane.
 */
export function presetColours(
  preset: PalettePreset,
  theme: PaletteTheme,
  read?: ColourReader,
): ReadonlyMap<string, string> {
  const paint = (token: string): Paint => {
    const computed = read?.(token, theme) ?? null;
    const found = computed === null ? null : parseComputed(computed);

    if (found !== null) return found;

    const color = presetOklch(token, preset, theme);

    return { rgb: oklchToRgb({ ...color, alpha: 1 }), alpha: color.alpha };
  };

  const pane = paint("--bg-pane").rgb;

  return new Map(
    PALETTE_FIELDS.map((field) => {
      const { rgb, alpha } = paint(field.token);

      return [field.token, toHex(alpha < 1 ? composite(rgb, alpha, pane) : rgb)];
    }),
  );
}

/** A colour the server accepts, with its alpha (1 when opaque). */
interface Paint {
  readonly rgb: Rgb;
  readonly alpha: number;
}

const CHANNEL = /^(\d+(?:\.\d+)?|\.\d+)(%?)$/;

function channel(value: string, max: number): number | null {
  const match = CHANNEL.exec(value.trim());

  if (match === null) {
    return null;
  }

  const number = Number(match[1]);
  const scaled = match[2] === "%" ? (number / 100) * max : number;

  return scaled <= max ? scaled : null;
}

/** Parses the hex and `rgb()` forms the server accepts (profile_settings.rs `valid_colour`). */
export function parseColour(value: string): Paint | null {
  const trimmed = value.trim().toLowerCase();
  const hex = /^#([\da-f]{3,4}|[\da-f]{6}|[\da-f]{8})$/.exec(trimmed)?.[1];

  if (hex !== undefined) {
    const long = hex.length <= 4 ? [...hex].map((digit) => digit + digit).join("") : hex;
    const byte = (index: number) => Number.parseInt(long.slice(index * 2, index * 2 + 2), 16);

    return {
      rgb: { r: byte(0), g: byte(1), b: byte(2) },
      alpha: long.length === 8 ? byte(3) / 255 : 1,
    };
  }

  const body = /^rgba?\((.*)\)$/.exec(trimmed)?.[1];

  if (body === undefined) {
    return null;
  }

  const comma = body.includes(",");
  const [rgb = "", alphaPart] = comma ? [body] : body.split("/");
  const alpha = alphaPart === undefined ? [] : [alphaPart];
  const parts = comma ? rgb.split(",") : [...rgb.trim().split(/\s+/), ...alpha];
  const [r, g, b, a] = parts.map((part, index) => channel(part, index < 3 ? 255 : 1));
  const counted = parts.length >= 3 && parts.length <= 4;

  if (!counted || [r, g, b, a].slice(0, parts.length).includes(null)) {
    return null;
  }

  return {
    rgb: { r: Math.round(r ?? 0), g: Math.round(g ?? 0), b: Math.round(b ?? 0) },
    alpha: a ?? 1,
  };
}

/** `#rgb`, `rrggbb` and the like, as typed, to `#rrggbb`; null for anything else. */
export function normalizeHex(input: string): string | null {
  const digits = input.trim().replace(/^#/, "").toLowerCase();

  if (!/^([\da-f]{3}|[\da-f]{6})$/.test(digits)) {
    return null;
  }

  return `#${digits.length === 3 ? [...digits].map((digit) => digit + digit).join("") : digits}`;
}

const SRGB = /^color\(srgb ([\d.]+) ([\d.]+) ([\d.]+)(?: \/ ([\d.]+))?\)$/;

/** A computed colour: the hex and `rgb()` forms, `oklch()` (Chrome keeps it) or `color(srgb)`. */
function parseComputed(value: string): Paint | null {
  const trimmed = value.trim().replaceAll("none", "0");
  const oklch = parseOklch(trimmed);

  if (oklch !== null) return { rgb: oklchToRgb({ ...oklch, alpha: 1 }), alpha: oklch.alpha };

  const srgb = SRGB.exec(trimmed);

  if (srgb === null) return parseColour(trimmed);

  const [, r = "0", g = "0", b = "0", alpha = "1"] = srgb;
  const byte = (channel: string) => Math.round(Math.min(1, Math.max(0, Number(channel))) * 255);

  return { rgb: { r: byte(r), g: byte(g), b: byte(b) }, alpha: Number(alpha) };
}

/** Custom colours by token name, as `appearancePreferences.tokens` holds them. */
export interface CustomTokens {
  readonly [token: string]: string;
}

/** The colour each editor token paints with: the custom one if it parses, else the preset's. */
export function effectiveColours(
  tokens: CustomTokens,
  presets: ReadonlyMap<string, string>,
): ReadonlyMap<string, Rgb> {
  const fallback = parseColour(presets.get("--bg-pane") ?? "#ffffff")?.rgb ?? {
    r: 255,
    g: 255,
    b: 255,
  };

  const paint = (token: string): Paint | null => {
    const custom = tokens[token];

    return (
      (custom === undefined ? null : parseColour(custom)) ?? parseColour(presets.get(token) ?? "")
    );
  };

  const pane = paint("--bg-pane")?.rgb ?? fallback;

  return new Map(
    PALETTE_FIELDS.map((field) => {
      const color = paint(field.token) ?? { rgb: fallback, alpha: 1 };

      return [field.token, color.alpha < 1 ? composite(color.rgb, color.alpha, pane) : color.rgb];
    }),
  );
}

/** The editor's colours in each theme. */
export type ThemeColours = Readonly<Record<PaletteTheme, ReadonlyMap<string, Rgb>>>;

/**
 * `tokens` over each theme's starting colours: a custom colour is one colour in both themes, so
 * it meets the other theme's background wherever that isn't customised too.
 */
export function themeColours(
  tokens: CustomTokens,
  presets: Readonly<Record<PaletteTheme, ReadonlyMap<string, string>>>,
): ThemeColours {
  return {
    light: effectiveColours(tokens, presets.light),
    dark: effectiveColours(tokens, presets.dark),
  };
}

/** A foreground the UI puts on a background, and the WCAG 2 AA ratio it should hold. */
export interface ContrastPair {
  readonly foreground: string;
  readonly background: string;
  /** 4.5:1 for text; 3:1 for large text and UI shapes such as a button against the pane. */
  readonly minimum: 4.5 | 3;
  /** Where "<foreground> on <background>" reads awkwardly. */
  readonly label?: string;
}

export const CONTRAST_PAIRS: readonly ContrastPair[] = [
  { foreground: "--text", background: "--bg-pane", minimum: 4.5 },
  { foreground: "--text", background: "--bg-sidebar", minimum: 4.5 },
  { foreground: "--text", background: "--bg-app", minimum: 4.5 },
  { foreground: "--text", background: "--bg-raised", minimum: 4.5 },
  { foreground: "--text", background: "--bg-selected", minimum: 4.5 },
  { foreground: "--text", background: "--mention-bg", minimum: 4.5 },
  { foreground: "--text-muted", background: "--bg-pane", minimum: 4.5 },
  { foreground: "--text-muted", background: "--bg-sidebar", minimum: 4.5 },
  { foreground: "--accent", background: "--bg-pane", minimum: 4.5 },
  {
    foreground: "--on-accent",
    background: "--accent-solid",
    minimum: 4.5,
    label: "Text on accent buttons",
  },
  { foreground: "--danger-text", background: "--bg-pane", minimum: 4.5 },
  { foreground: "--success-text", background: "--bg-pane", minimum: 4.5 },
  {
    foreground: "--accent-solid",
    background: "--bg-pane",
    minimum: 3,
    label: "Accent buttons against messages",
  },
];

export interface ContrastResult extends ContrastPair {
  readonly label: string;
  readonly ratios: Readonly<Record<PaletteTheme, number>>;
  /** The lower of the two. */
  readonly ratio: number;
  /** The themes the pair falls short in. */
  readonly failing: readonly PaletteTheme[];
  readonly passes: boolean;
}

const LABELS: ReadonlyMap<string, string> = new Map(
  PALETTE_FIELDS.map((field) => [field.token, field.label]),
);

const THEMES: readonly PaletteTheme[] = ["light", "dark"];

/** Every pair's ratio in both themes under `colours` (from `themeColours`). */
export function contrastReport(colours: ThemeColours): readonly ContrastResult[] {
  const white = { r: 255, g: 255, b: 255 };

  return CONTRAST_PAIRS.map((pair) => {
    const measure = (theme: PaletteTheme) =>
      contrastRatio(
        colours[theme].get(pair.foreground) ?? white,
        colours[theme].get(pair.background) ?? white,
      );

    const ratios = { light: measure("light"), dark: measure("dark") };
    const failing = THEMES.filter((theme) => ratios[theme] < pair.minimum);
    const foreground = LABELS.get(pair.foreground) ?? pair.foreground;
    const background = (LABELS.get(pair.background) ?? pair.background).toLowerCase();

    return {
      ...pair,
      label: pair.label ?? `${foreground} on ${background}`,
      ratios,
      ratio: Math.min(ratios.light, ratios.dark),
      failing,
      passes: failing.length === 0,
    };
  });
}

/** A ratio as shown: rounded down, so 4.47:1 never reads as 4.5:1. */
export function formatRatio(ratio: number): string {
  return `${(Math.floor(ratio * 10) / 10).toFixed(1)}:1`;
}

/** `tokens` with `field` (and its companions) set to `colour`. */
export function setFieldColour(
  tokens: CustomTokens,
  field: PaletteField,
  colour: string,
): CustomTokens {
  const written = [field.token, ...(field.also ?? [])].map((token) => [token, colour] as const);

  return { ...tokens, ...Object.fromEntries(written) };
}

/** `tokens` without `field` (and its companions): the preset's colour again. */
export function resetField(tokens: CustomTokens, field: PaletteField): CustomTokens {
  const drop = new Set([field.token, ...(field.also ?? [])]);

  return Object.fromEntries(Object.entries(tokens).filter(([token]) => !drop.has(token)));
}

export function sameTokens(first: CustomTokens, second: CustomTokens): boolean {
  const keys = Object.keys(first);

  return (
    keys.length === Object.keys(second).length && keys.every((key) => first[key] === second[key])
  );
}

/** The `tokens` field a save sends: the colours in key order, or `null` to clear them all. */
export function tokensPayload(tokens: CustomTokens): CustomTokens | null {
  const entries = Object.entries(tokens).sort(([first], [second]) => first.localeCompare(second));

  return entries.length === 0 ? null : Object.fromEntries(entries);
}
