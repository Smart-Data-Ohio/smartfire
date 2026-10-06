import { useSyncExternalStore } from "react";

/** How often relative times ("3 minutes ago") refresh. */
const TICK_MS = 30_000;

let now = Date.now();

const listeners = new Set<() => void>();

let timer: number | undefined;

function subscribe(listener: () => void): () => void {
  listeners.add(listener);

  if (timer === undefined) {
    now = Date.now();
    timer = window.setInterval(() => {
      now = Date.now();

      for (const notify of listeners) {
        notify();
      }
    }, TICK_MS);
  }

  return () => {
    listeners.delete(listener);

    if (listeners.size === 0) {
      window.clearInterval(timer);
      timer = undefined;
    }
  };
}

const snapshot = () => now;

/**
 * The current time, refreshed every 30 s by one shared timer, so every "Last reply 3 minutes
 * ago" on screen ticks over together without a timer each.
 */
export function useNow(): number {
  return useSyncExternalStore(subscribe, snapshot, snapshot);
}
