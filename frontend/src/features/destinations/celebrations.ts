import { useEffect, useRef, useState } from "react";
import { readDurationMs } from "../../motion/durations.ts";

interface Celebrations {
  /** Whether `id`'s row draws its success check in. */
  readonly has: (id: number) => boolean;
  /** The viewer just finished `id` here: its check draws in, and the flag clears once it has. */
  readonly start: (id: number) => void;
  /** The action failed (or the row moved on): no check. */
  readonly stop: (id: number) => void;
}

/** How long the success check's draw-in takes (motion/recipes/success-check.css), plus a margin. */
function holdMs(): number {
  return readDurationMs("--duration-intent") + readDurationMs("--duration-spring") + 100;
}

/**
 * The rows the viewer just marked handled or done, so their check draws in (motion/success-check)
 * rather than appearing. Each flag clears after the animation, or at once when the action fails.
 */
export function useCelebrations(): Celebrations {
  const [ids, setIds] = useState<ReadonlySet<number>>(() => new Set());
  const timers = useRef(new Map<number, number>());

  useEffect(() => {
    const pending = timers.current;

    return () => {
      for (const timer of pending.values()) {
        window.clearTimeout(timer);
      }

      pending.clear();
    };
  }, []);

  const stop = (id: number) => {
    window.clearTimeout(timers.current.get(id));
    timers.current.delete(id);
    setIds((current) => {
      if (!current.has(id)) {
        return current;
      }

      const next = new Set(current);

      next.delete(id);

      return next;
    });
  };

  const start = (id: number) => {
    window.clearTimeout(timers.current.get(id));
    timers.current.set(
      id,
      window.setTimeout(() => stop(id), holdMs()),
    );
    setIds((current) => (current.has(id) ? current : new Set(current).add(id)));
  };

  return { has: (id) => ids.has(id), start, stop };
}
