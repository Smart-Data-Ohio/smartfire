import { type RefObject, useLayoutEffect, useRef } from "react";
import { readDurationMs } from "../../motion/durations.ts";
import { prefersReducedMotion } from "../../motion/reduced-motion.ts";

interface Placed {
  readonly element: HTMLElement;
  readonly top: number;
  /** The `data-flip` key of the nearest flipped ancestor (a row's section). */
  readonly parent: string | null;
}

function measure(container: HTMLElement | null): ReadonlyMap<string, Placed> {
  const placed = new Map<string, Placed>();

  if (container === null) {
    return placed;
  }

  for (const element of container.querySelectorAll<HTMLElement>("[data-flip]")) {
    const key = element.dataset.flip;

    if (key !== undefined && !placed.has(key)) {
      placed.set(key, {
        element,
        top: element.getBoundingClientRect().top,
        parent: element.parentElement?.closest<HTMLElement>("[data-flip]")?.dataset.flip ?? null,
      });
    }
  }

  return placed;
}

function cssValue(token: string, fallback: string): string {
  return getComputedStyle(document.documentElement).getPropertyValue(token).trim() || fallback;
}

/**
 * FLIP for the sidebar's reorders: `capture()` notes where every `[data-flip]` element is, and
 * the next render that moves any of them plays each one from its old place to its new one (its
 * section's own move subtracted, since that already carries it). A row that changed sections is
 * a new element inside a clipped panel, so it settles in place instead of flying across. Nothing
 * plays under reduced motion.
 */
export function useFlip(containerRef: RefObject<HTMLElement | null>): () => void {
  const snapshot = useRef<ReadonlyMap<string, Placed> | null>(null);
  const expiry = useRef(0);

  useLayoutEffect(() => {
    const before = snapshot.current;

    if (before === null) {
      return;
    }

    const after = measure(containerRef.current);
    const duration = readDurationMs("--duration-medium");
    const easing = cssValue("--ease-move", "ease-out");
    let moved = false;

    for (const [key, now] of after) {
      const was = before.get(key);

      if (was === undefined || !("animate" in now.element)) {
        continue;
      }

      if (was.parent !== now.parent) {
        moved = true;
        now.element.animate(
          [
            { opacity: 0, scale: "0.96" },
            { opacity: 1, scale: "1" },
          ],
          {
            duration: readDurationMs("--duration-small"),
            easing: cssValue("--ease-enter", "ease-out"),
          },
        );

        continue;
      }

      const parentWas = now.parent === null ? undefined : before.get(now.parent);
      const parentNow = now.parent === null ? undefined : after.get(now.parent);

      const carried =
        parentWas === undefined || parentNow === undefined ? 0 : parentWas.top - parentNow.top;

      const delta = was.top - now.top - carried;

      if (Math.abs(delta) < 1) {
        continue;
      }

      moved = true;
      now.element.animate([{ translate: `0 ${delta}px` }, { translate: "0 0" }], {
        duration,
        easing,
      });
    }

    if (moved) {
      snapshot.current = null;
    }
  });

  useLayoutEffect(() => () => window.clearTimeout(expiry.current), []);

  return () => {
    if (prefersReducedMotion()) {
      return;
    }

    snapshot.current = measure(containerRef.current);
    window.clearTimeout(expiry.current);
    // A change that never moves anything (refused, or a no-op) shouldn't animate a later one.
    expiry.current = window.setTimeout(() => {
      snapshot.current = null;
    }, 1000);
  };
}
