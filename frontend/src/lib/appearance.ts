import { useSyncExternalStore } from "react";
import type { TextSize } from "../gen/TextSize.ts";
import { prefersReducedMotion } from "../motion/reduced-motion.ts";

/**
 * Appearance lives as attributes on <html> (`data-theme`, `data-text-size`, `data-density`,
 * `data-motion`), which is all the stylesheets read. "system" removes `data-theme` and the OS
 * setting decides.
 *
 * The theme on screen comes from, in order:
 * 1. a theme pinned on this device (Settings → Appearance → This device), when there is one;
 * 2. the account's theme, from the inline boot JSON before the first paint and from `/me` after;
 * 3. the account's theme as this device last saw it, until boot is read (the Vite page has none);
 * 4. the OS setting.
 * Text size is the account's alone. Density and motion are this device's alone. Everything is
 * remembered under one localStorage key; its `theme` is the theme on screen, which the classic
 * pages' script (crates/assets/auth/auth.js) applies too.
 */
export type ThemePreference = "system" | "light" | "dark";

export type DensityPreference = "comfortable" | "compact";

export type MotionPreference = "system" | "reduce" | "full";

export type ResolvedTheme = "light" | "dark";

export type { TextSize };

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
}

const STORAGE_KEY = "smartfire.appearance";

const THEMES: readonly ThemePreference[] = ["system", "light", "dark"];

const TEXT_SIZES: readonly TextSize[] = ["smaller", "small", "default", "large", "larger"];

const DENSITIES: readonly DensityPreference[] = ["comfortable", "compact"];

const MOTIONS: readonly MotionPreference[] = ["system", "reduce", "full"];

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
};

let current: Appearance = DEFAULTS;

const listeners = new Set<() => void>();

function writeAttributes(appearance: Appearance): void {
  const { dataset } = document.documentElement;

  if (appearance.theme === "system") {
    delete dataset.theme;
  } else {
    dataset.theme = appearance.theme;
  }

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
}

function store(appearance: Appearance): void {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(appearance));
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

function readStored(): Map<string, string> {
  try {
    const parsed: unknown = JSON.parse(localStorage.getItem(STORAGE_KEY) ?? "null");

    // Each field as text: a stored `null` reads "null", which no choice matches.
    return parsed instanceof Object
      ? new Map<string, string>(Object.entries(parsed).map(([key, field]) => [key, String(field)]))
      : new Map();
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
    const parsed: unknown = JSON.parse(text);

    if (!(parsed instanceof Object)) {
      return {};
    }

    const fields = new Map<string, string>(Object.entries(parsed).map(([key, field]) => [key, String(field)]));

    return {
      theme: pick(THEMES, fields.get("theme")) ?? "system",
      textSize: pick(TEXT_SIZES, fields.get("textSize")) ?? "default",
    };
  } catch {
    return {};
  }
}

/**
 * Applies the appearance before the first render, so nothing flashes: the account's choices from
 * the inline boot JSON (else as last remembered), under this device's pinned theme. A device that
 * remembered only a bare `theme` (before themes followed the account) takes it as the account's
 * last known theme, which boot then replaces.
 */
export function restoreAppearance(): void {
  const saved = readStored();
  const boot = readInlineBoot();
  // Every save since themes followed the account carries `accountTheme`.
  const legacy = !saved.has("accountTheme");
  const remembered = pick(THEMES, saved.get(legacy ? "theme" : "accountTheme"));

  commit({
    themeOverride: legacy ? null : pick(THEMES, saved.get("themeOverride")),
    accountTheme: boot.theme ?? remembered ?? "system",
    textSize: boot.textSize ?? pick(TEXT_SIZES, saved.get("textSize")) ?? "default",
    density: pick(DENSITIES, saved.get("density")) ?? "comfortable",
    motion: pick(MOTIONS, saved.get("motion")) ?? "system",
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
