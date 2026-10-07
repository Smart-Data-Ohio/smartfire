import { type RefObject, useLayoutEffect, useRef } from "react";
import { readDurationMs } from "../../motion/durations.ts";
import { prefersReducedMotion } from "../../motion/reduced-motion.ts";

interface Placed {
  readonly left: number;
  readonly top: number;
  /** The `data-flip-group` of the list it sits in (a column), or "" outside one. */
  readonly group: string;
}

function cssValue(token: string, fallback: string): string {
  return getComputedStyle(document.documentElement).getPropertyValue(token).trim() || fallback;
}

/**
 * Where every `[data-flip]` row sits, in the container's scrolled coordinates (each enclosing
 * `[data-flip-scroll]` list's own scroll added back), so scrolling between two renders doesn't
 * read as a move.
 */
function measure(container: HTMLElement): Map<string, Placed> {
  const placed = new Map<string, Placed>();
  const origin = container.getBoundingClientRect();

  for (const element of container.querySelectorAll<HTMLElement>("[data-flip]")) {
    const key = element.dataset.flip;

    if (key === undefined || placed.has(key)) {
      continue;
    }

    const rect = element.getBoundingClientRect();
    const list = element.closest<HTMLElement>("[data-flip-scroll]");
    const scrollTop = (list ?? container).scrollTop;
    const scrollLeft = container.scrollLeft;

    placed.set(key, {
      left: rect.left - origin.left + scrollLeft,
      top: rect.top - origin.top + scrollTop,
      group: element.closest<HTMLElement>("[data-flip-group]")?.dataset.flipGroup ?? "",
    });
  }

  return placed;
}

/**
 * Live row moves for a list that reorders on its own (sync events, not the viewer's hand): after
 * every render whose `signature` changed, each `[data-flip]` row glides from where it was to
 * where it is now. A row that moved to another `[data-flip-group]` (a post changing column) is a
 * new element there, so it settles in with a short fade and scale instead of flying across, and
 * a row that just appeared fades in. Nothing plays on the first layout, under reduced motion, or
 * while the list is replaced wholesale (`reset` changes: a new filter).
 */
export function useLiveFlip(
  containerRef: RefObject<HTMLElement | null>,
  signature: string,
  reset: string,
): void {
  const placed = useRef<Map<string, Placed> | null>(null);
  const lastSignature = useRef(signature);
  const lastReset = useRef(reset);

  useLayoutEffect(() => {
    const container = containerRef.current;

    if (container === null) {
      return;
    }

    const before = placed.current;
    const after = measure(container);
    const changed = signature !== lastSignature.current;
    const replaced = reset !== lastReset.current;

    placed.current = after;
    lastSignature.current = signature;
    lastReset.current = reset;

    if (before === null || !changed || replaced || prefersReducedMotion()) {
      return;
    }

    const duration = readDurationMs("--duration-medium");
    const small = readDurationMs("--duration-small");
    const move = cssValue("--ease-move", "ease-out");
    const enter = cssValue("--ease-enter", "ease-out");

    for (const element of container.querySelectorAll<HTMLElement>("[data-flip]")) {
      const key = element.dataset.flip;
      const now = key === undefined ? undefined : after.get(key);
      const was = key === undefined ? undefined : before.get(key);

      if (now === undefined || !("animate" in element)) {
        continue;
      }

      if (was === undefined || was.group !== now.group) {
        element.animate(
          [
            { opacity: 0, scale: "0.97" },
            { opacity: 1, scale: "1" },
          ],
          { duration: small, easing: enter },
        );
        continue;
      }

      const dx = was.left - now.left;
      const dy = was.top - now.top;

      if (Math.abs(dx) < 1 && Math.abs(dy) < 1) {
        continue;
      }

      element.animate([{ translate: `${dx}px ${dy}px` }, { translate: "0 0" }], {
        duration,
        easing: move,
      });
    }
  });
}
