import { useSyncExternalStore } from "react";

const QUERY = "(prefers-reduced-motion: reduce)";

function systemPrefersReduced(): boolean {
  return "matchMedia" in window && window.matchMedia(QUERY).matches;
}

/**
 * Whether motion should be reduced right now: the user's Smartfire override
 * (`<html data-motion="reduce" | "full">`) wins, otherwise the OS setting decides.
 */
export function prefersReducedMotion(): boolean {
  const override = document.documentElement.dataset.motion;

  if (override === "reduce") {
    return true;
  }

  if (override === "full") {
    return false;
  }

  return systemPrefersReduced();
}

function subscribe(onChange: () => void): () => void {
  const media = "matchMedia" in window ? window.matchMedia(QUERY) : null;
  const observer = new MutationObserver(onChange);

  media?.addEventListener("change", onChange);
  observer.observe(document.documentElement, {
    attributes: true,
    attributeFilter: ["data-motion"],
  });

  return () => {
    media?.removeEventListener("change", onChange);
    observer.disconnect();
  };
}

/** Live reduced-motion state for components that animate from JavaScript, WebGL or canvas. */
export function useReducedMotion(): boolean {
  return useSyncExternalStore(subscribe, prefersReducedMotion, () => false);
}
