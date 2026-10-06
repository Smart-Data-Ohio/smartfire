import { useSyncExternalStore } from "react";
import { prefersReducedMotion } from "../motion/reduced-motion.ts";

/**
 * Per-device appearance: theme, density and motion live as attributes on <html>
 * (`data-theme`, `data-density`, `data-motion`), which is all the stylesheets read, and are
 * remembered in localStorage. "system" removes the attribute and the OS setting decides.
 */
export type ThemePreference = "system" | "light" | "dark";

export type DensityPreference = "comfortable" | "compact";

export type MotionPreference = "system" | "reduce" | "full";

export type ResolvedTheme = "light" | "dark";

export interface Appearance {
  readonly theme: ThemePreference;
  readonly density: DensityPreference;
  readonly motion: MotionPreference;
}

const STORAGE_KEY = "smartfire.appearance";

const THEMES: readonly ThemePreference[] = ["system", "light", "dark"];

const DENSITIES: readonly DensityPreference[] = ["comfortable", "compact"];

const MOTIONS: readonly MotionPreference[] = ["system", "reduce", "full"];

function pick<T extends string>(options: readonly T[], value: string | undefined, fallback: T): T {
  return options.find((option) => option === value) ?? fallback;
}

function readAttributes(): Appearance {
  const { dataset } = document.documentElement;

  return {
    theme: pick(THEMES, dataset.theme, "system"),
    density: pick(DENSITIES, dataset.density, "comfortable"),
    motion: pick(MOTIONS, dataset.motion, "system"),
  };
}

function writeAttributes(appearance: Appearance): void {
  const { dataset } = document.documentElement;

  if (appearance.theme === "system") {
    delete dataset.theme;
  } else {
    dataset.theme = appearance.theme;
  }

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

/** Applies the remembered appearance. Call once before the first render to avoid a flash. */
export function restoreAppearance(): void {
  let saved: string | null = null;

  try {
    saved = localStorage.getItem(STORAGE_KEY);
  } catch {
    return;
  }

  if (saved === null) {
    return;
  }

  let fields: Map<string, string>;

  try {
    const parsed: unknown = JSON.parse(saved);

    if (!(parsed instanceof Object)) {
      return;
    }

    fields = new Map(Object.entries(parsed).map(([key, value]) => [key, String(value)]));
  } catch {
    return;
  }

  writeAttributes({
    theme: pick(THEMES, fields.get("theme"), "system"),
    density: pick(DENSITIES, fields.get("density"), "comfortable"),
    motion: pick(MOTIONS, fields.get("motion"), "system"),
  });
}

function update(change: Partial<Appearance>): void {
  const next = { ...readAttributes(), ...change };

  writeAttributes(next);
  store(next);
}

/**
 * Switches the theme. Where View Transitions exist (and motion is allowed) the whole page
 * cross-fades on the small step instead of every surface snapping at once.
 */
export function setTheme(theme: ThemePreference): void {
  if ("startViewTransition" in document && !prefersReducedMotion()) {
    document.startViewTransition(() => update({ theme }));

    return;
  }

  update({ theme });
}

export function setDensity(density: DensityPreference): void {
  update({ density });
}

export function setMotion(motion: MotionPreference): void {
  update({ motion });
}

const DARK_QUERY = "(prefers-color-scheme: dark)";

function subscribe(onChange: () => void): () => void {
  const media = "matchMedia" in window ? window.matchMedia(DARK_QUERY) : null;
  const observer = new MutationObserver(onChange);

  media?.addEventListener("change", onChange);
  observer.observe(document.documentElement, {
    attributes: true,
    attributeFilter: ["data-theme", "data-density", "data-motion"],
  });

  return () => {
    media?.removeEventListener("change", onChange);
    observer.disconnect();
  };
}

let cachedKey = "";

let cached: Appearance = { theme: "system", density: "comfortable", motion: "system" };

function appearanceSnapshot(): Appearance {
  const current = readAttributes();
  const key = `${current.theme}|${current.density}|${current.motion}`;

  if (key !== cachedKey) {
    cachedKey = key;
    cached = current;
  }

  return cached;
}

const SERVER_APPEARANCE: Appearance = { theme: "system", density: "comfortable", motion: "system" };

export function useAppearance(): Appearance {
  return useSyncExternalStore(subscribe, appearanceSnapshot, () => SERVER_APPEARANCE);
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
