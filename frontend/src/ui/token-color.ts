import { useMemo } from "react";
import { useResolvedTheme } from "../lib/appearance.ts";
import { tokenToRgbString } from "../lib/color.ts";

/**
 * A colour token as `rgb()`, for canvas and WebGL libraries that can't read CSS variables. A probe
 * element resolves `light-dark()` and `var()` chains for the current theme.
 */
export function readTokenRgb(token: string, fallback: string): string {
  const probe = document.createElement("span");

  probe.style.color = `var(${token})`;
  probe.style.display = "none";
  document.body.append(probe);

  const value = getComputedStyle(probe).color;

  probe.remove();

  return tokenToRgbString(value, fallback);
}

/** readTokenRgb, re-read when the theme flips. */
export function useTokenRgb(token: string, fallback: string): string {
  const theme = useResolvedTheme();

  // biome-ignore lint/correctness/useExhaustiveDependencies: the token resolves differently per theme
  return useMemo(() => readTokenRgb(token, fallback), [token, fallback, theme]);
}
