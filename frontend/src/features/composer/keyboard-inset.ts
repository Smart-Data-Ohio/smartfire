import { type RefObject, useEffect } from "react";

/**
 * Keeps the composer above an on-screen keyboard that overlays the page instead of resizing it
 * (iOS Safari): the gap between the layout and visual viewports becomes `--keyboard-inset` on
 * the composer, which pads its bottom. Elsewhere the inset stays 0.
 */
export function useKeyboardInset(ref: RefObject<HTMLElement | null>): void {
  useEffect(() => {
    const viewport = window.visualViewport;
    const element = ref.current;

    if (viewport === null || viewport === undefined || element === null) {
      return;
    }

    const update = () => {
      const inset = Math.max(0, window.innerHeight - viewport.height - viewport.offsetTop);

      element.style.setProperty("--keyboard-inset", `${Math.round(inset)}px`);
    };

    update();
    viewport.addEventListener("resize", update);
    viewport.addEventListener("scroll", update);

    return () => {
      viewport.removeEventListener("resize", update);
      viewport.removeEventListener("scroll", update);
    };
  }, [ref]);
}
