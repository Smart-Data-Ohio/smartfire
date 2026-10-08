import { useEffect } from "react";
import { COARSE_QUERY } from "./breakpoints.ts";

/** Under this, a change in the viewport's height is browser chrome folding away, not a keyboard. */
const KEYBOARD_MIN_HEIGHT = 120;

/**
 * The on-screen keyboard, for the whole app; the shell runs it once. Where the keyboard overlays
 * the page instead of resizing it (iOS Safari), the strip of the layout viewport it covers becomes
 * `--keyboard-inset` on <html>, which the shell's height subtracts. While any keyboard is up,
 * overlaid or resizing (Android, under `interactive-widget=resizes-content`), <html> also carries
 * `data-keyboard="open"`. Pinch zoom shrinks the visual viewport too, so a zoomed page has none.
 */
export function useKeyboardInset(): void {
  useEffect(() => {
    const viewport = window.visualViewport;

    if (viewport === null || viewport === undefined) {
      return;
    }

    const root = document.documentElement;
    const coarse = window.matchMedia(COARSE_QUERY);
    // The tallest layout viewport at this width: a resizing keyboard shows as a drop from it.
    let width = window.innerWidth;
    let tallest = window.innerHeight;

    const update = () => {
      if (window.innerWidth !== width) {
        width = window.innerWidth;
        tallest = window.innerHeight;
      }

      tallest = Math.max(tallest, window.innerHeight);

      const zoomed = Math.abs(viewport.scale - 1) > 0.01;
      const covered = zoomed ? 0 : window.innerHeight - viewport.height;
      const inset = zoomed ? 0 : Math.max(0, covered - viewport.offsetTop);
      const resized = coarse.matches ? tallest - window.innerHeight : 0;

      root.style.setProperty("--keyboard-inset", `${Math.round(inset)}px`);

      if (Math.max(covered, resized) >= KEYBOARD_MIN_HEIGHT) {
        root.dataset.keyboard = "open";
      } else {
        delete root.dataset.keyboard;
      }
    };

    update();
    viewport.addEventListener("resize", update);
    viewport.addEventListener("scroll", update);
    window.addEventListener("resize", update);

    return () => {
      viewport.removeEventListener("resize", update);
      viewport.removeEventListener("scroll", update);
      window.removeEventListener("resize", update);
      root.style.removeProperty("--keyboard-inset");
      delete root.dataset.keyboard;
    };
  }, []);
}
