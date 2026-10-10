import { useSyncExternalStore } from "react";
import type { TextSize } from "../gen/TextSize.ts";
import { prefersReducedMotion } from "../motion/reduced-motion.ts";
import { PALETTE_TOKEN_NAMES, PALETTES, type PalettePreset, paletteTokens } from "./palette.ts";

/**
 * Appearance lives as attributes on <html> (`data-theme`, `data-text-size`, `data-density`,
 * `data-motion`), which is all the stylesheets read. "system" removes `data-theme` and the OS
 * setting decides.
 *
 * The theme on screen comes from, in order:
 * 1. a theme pinned on this device (Settings → Appearance → This device), when there is one;
 * 2. the account's theme, from the inline boot JSON and from `/me` after;
 * 3. the OS setting.
 * Text size is the account's alone. Density, motion, the colour palette (`data-palette`, its
 * tokens set on <html>) and the font (`data-font`) are this device's alone.
 *
 * Only this device's own choices (the pin, density, motion, palette, font) are stored, under one
 * localStorage key: never the account's, so the next person to sign in on this browser doesn't
 * inherit them. index.html's blocking script applies the stored choices and the inline boot's
 * before the stylesheet paints; the classic pages' script (crates/assets/auth/auth.js) applies
 * the pin.
 */
export type ThemePreference = "system" | "light" | "dark";

export type DensityPreference = "comfortable" | "compact";

export type MotionPreference = "system" | "reduce" | "full";

export type ResolvedTheme = "light" | "dark";

/** Inter is Smartfire's own; the rest are self-hosted presets (typography.css). */
export type FontPreset = "inter" | "system" | "atkinson" | "serif" | "mono";

export type { PalettePreset, TextSize };

/** The appearance an account carries (boot, `/me`, the settings page). */
export interface AccountAppearance {
  readonly theme: ThemePreference;
  readonly textSize: TextSize;
}

export interface Appearance {
  /** The theme on screen: this device's pin, else the account's. */
  readonly theme: ThemePreference;
  /** A theme pinned on this device, over the account's; `null` follows the account. */
  readonly themeOverride: ThemePreference | null;
  readonly accountTheme: ThemePreference;
  readonly textSize: TextSize;
  readonly density: DensityPreference;
  readonly motion: MotionPreference;
  readonly palette: PalettePreset;
  readonly font: FontPreset;
}

const STORAGE_KEY = "smartfire.appearance";

const THEMES: readonly ThemePreference[] = ["system", "light", "dark"];

const TEXT_SIZES: readonly TextSize[] = ["smaller", "small", "default", "large", "larger"];

const DENSITIES: readonly DensityPreference[] = ["comfortable", "compact"];

const MOTIONS: readonly MotionPreference[] = ["system", "reduce", "full"];

const PALETTE_VALUES: readonly PalettePreset[] = PALETTES.map((palette) => palette.value);

const FONTS: readonly FontPreset[] = ["inter", "system", "atkinson", "serif", "mono"];

function pick<T extends string>(options: readonly T[], value: string | undefined): T | null {
  return options.find((option) => option === value) ?? null;
}

const DEFAULTS: Appearance = {
  theme: "system",
  themeOverride: null,
  accountTheme: "system",
  textSize: "default",
  density: "comfortable",
  motion: "system",
  palette: "smartfire",
  font: "inter",
};

let current: Appearance = DEFAULTS;

const listeners = new Set<() => void>();

function writeAttributes(appearance: Appearance): void {
  const root = document.documentElement;
  const { dataset } = root;
  root.style.setProperty(
    "color-scheme",
    appearance.theme === "system" ? "light dark" : appearance.theme,
  );

  if (appearance.theme === "system") {
    delete dataset.theme;
  } else {
    dataset.theme = appearance.theme;
  }

  writeThemeColor(appearance.theme);

  dataset.textSize = appearance.textSize;

  if (appearance.density === "comfortable") {
    delete dataset.density;
  } else {
    dataset.density = appearance.density;
  }

  if (appearance.motion === "system") {
    delete dataset.motion;
  } else {
    dataset.motion = appearance.motion;
  }

  if (appearance.font === "inter") {
    delete dataset.font;
    root.style.removeProperty("--font-sans");
    root.style.removeProperty("--font-features");
  } else {
    dataset.font = appearance.font;
    root.style.setProperty("--font-sans", `var(--font-preset-${appearance.font})`);
    root.style.setProperty("--font-features", "normal");
  }

  writePalette(appearance.palette);
}

/**
 * Points index.html's two `theme-color` metas at the theme on screen: each follows the OS under
 * "system", and a pinned theme switches its own on and the other off.
 */
function writeThemeColor(theme: ThemePreference): void {
  for (const meta of document.querySelectorAll<HTMLMetaElement>(
    'meta[name="theme-color"][data-scheme]',
  )) {
    const scheme = meta.dataset.scheme;

    meta.media =
      theme === "system"
        ? `(prefers-color-scheme: ${scheme})`
        : theme === scheme
          ? "all"
          : "not all";
  }
}

/**
 * Sets the palette's tokens on <html> (through the CSSOM, which the shell's CSP allows), over the
 * stylesheet's and the workspace's custom CSS; Smartfire's own palette removes them.
 */
function writePalette(palette: PalettePreset): void {
  const root = document.documentElement;
  const tokens = paletteTokens(palette);

  if (palette === "smartfire") {
    delete root.dataset.palette;
  } else {
    root.dataset.palette = palette;
  }

  for (const name of PALETTE_TOKEN_NAMES) {
    const value = tokens.get(name);

    if (value === undefined) {
      root.style.removeProperty(name);
    } else {
      root.style.setProperty(name, value);
    }
  }
}

function store(appearance: Appearance): void {
  const { themeOverride, density, motion, palette, font } = appearance;

  try {
    // The palette by name only: index.html carries every palette's tokens. Whatever else an
    // earlier version stored (a palette's tokens, an account's theme) goes with this write.
    localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify({ themeOverride, density, motion, palette, font }),
    );
  } catch {
    // Storage can be unavailable (private windows, quota); the choice still applies to this tab.
  }
}

/** Shows `next` (the theme on screen recomputed), remembers it, and tells the hooks. */
function commit(next: Omit<Appearance, "theme">): void {
  current = { ...next, theme: next.themeOverride ?? next.accountTheme };
  writeAttributes(current);
  store(current);

  for (const listener of listeners) {
    listener();
  }
}

/** Runs `change`, cross-fading the page where View Transitions exist and motion is allowed. */
function transition(change: () => void): void {
  if ("startViewTransition" in document && !prefersReducedMotion()) {
    document.startViewTransition(change);

    return;
  }

  change();
}

/**
 * The text fields of the JSON object in `json`, as index.html's blocking script reads them: a
 * corrupted `["dark"]` (which `String` would turn into "dark") is no choice in either place, and
 * JSON that isn't an object has none. Throws on JSON that doesn't parse.
 */
function textFields(json: string): Map<string, string> {
  const parsed: unknown = JSON.parse(json);

  return parsed instanceof Object
    ? new Map(
        Object.entries(parsed).flatMap(([key, field]): [string, string][] =>
          String(field) === field ? [[key, field]] : [],
        ),
      )
    : new Map();
}

function readStored(): Map<string, string> {
  try {
    return textFields(localStorage.getItem(STORAGE_KEY) ?? "null");
  } catch {
    return new Map();
  }
}

/** The account's theme and size in the shell's inline boot JSON, when it has one. */
function readInlineBoot(): Partial<AccountAppearance> {
  const text = document.getElementById("boot")?.textContent;

  if (text === undefined || text === null || text === "") {
    return {};
  }

  try {
    const fields = textFields(text);

    return {
      theme: pick(THEMES, fields.get("theme")) ?? "system",
      textSize: pick(TEXT_SIZES, fields.get("textSize")) ?? "default",
    };
  } catch {
    return {};
  }
}

/**
 * Takes over what index.html's blocking script painted, before the first render: the account's
 * choices from the inline boot JSON (the Vite page without one follows the OS until boot loads),
 * under this device's pinned theme. Anything else an earlier version stored (a bare `theme`, an
 * `accountTheme`) was some account's, so it is dropped rather than shown to whoever is here now.
 */
export function restoreAppearance(): void {
  const saved = readStored();
  const boot = readInlineBoot();

  commit({
    themeOverride: pick(THEMES, saved.get("themeOverride")),
    accountTheme: boot.theme ?? "system",
    textSize: boot.textSize ?? "default",
    density: pick(DENSITIES, saved.get("density")) ?? "comfortable",
    motion: pick(MOTIONS, saved.get("motion")) ?? "system",
    palette: pick(PALETTE_VALUES, saved.get("palette")) ?? "smartfire",
    font: pick(FONTS, saved.get("font")) ?? "inter",
  });
}

/**
 * The account's appearance, as boot, `/me` or a settings save reports it. A theme pinned on this
 * device stays on screen.
 */
export function applyAccountAppearance(account: AccountAppearance): void {
  if (current.accountTheme === account.theme && current.textSize === account.textSize) {
    return;
  }

  commit({ ...current, accountTheme: account.theme, textSize: account.textSize });
}

/** A theme the person just chose for their account: shown at once, cross-fading. */
export function showAccountTheme(theme: ThemePreference): void {
  transition(() => commit({ ...current, accountTheme: theme }));
}

/** A text size the person just chose for their account: shown at once. */
export function showTextSize(textSize: TextSize): void {
  commit({ ...current, textSize });
}

/** Pins `theme` on this device over the account's, or (`null`) follows the account again. */
export function setThemeOverride(theme: ThemePreference | null): void {
  transition(() => commit({ ...current, themeOverride: theme }));
}

export function setDensity(density: DensityPreference): void {
  commit({ ...current, density });
}

export function setMotion(motion: MotionPreference): void {
  commit({ ...current, motion });
}

/** Paints this device in `palette`, cross-fading like a theme change. */
export function setPalette(palette: PalettePreset): void {
  transition(() => commit({ ...current, palette }));
}

export function setFont(font: FontPreset): void {
  commit({ ...current, font });
}

const DARK_QUERY = "(prefers-color-scheme: dark)";

function subscribe(onChange: () => void): () => void {
  const media = "matchMedia" in window ? window.matchMedia(DARK_QUERY) : null;

  listeners.add(onChange);
  media?.addEventListener("change", onChange);

  return () => {
    listeners.delete(onChange);
    media?.removeEventListener("change", onChange);
  };
}

/** The appearance as last applied. */
export function appearanceSnapshot(): Appearance {
  return current;
}

export function useAppearance(): Appearance {
  return useSyncExternalStore(subscribe, appearanceSnapshot, () => DEFAULTS);
}

/** The theme actually on screen: the pinned one, or the OS preference under "system". */
export function resolvedTheme(): ResolvedTheme {
  const pinned = document.documentElement.dataset.theme;

  if (pinned === "light" || pinned === "dark") {
    return pinned;
  }

  return "matchMedia" in window && window.matchMedia(DARK_QUERY).matches ? "dark" : "light";
}

export function useResolvedTheme(): ResolvedTheme {
  return useSyncExternalStore(subscribe, resolvedTheme, () => "light");
}
