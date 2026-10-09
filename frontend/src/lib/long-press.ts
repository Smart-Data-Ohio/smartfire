import { type PointerEvent, useEffect, useLayoutEffect, useRef } from "react";

/** How long a finger rests on a row before its menu opens (touch has no right click). */
export const LONG_PRESS_MS = 500;

/** How far a resting finger may drift: further is a scroll or a swipe, not a long press. */
export const LONG_PRESS_SLOP = 8;

/** How long a press may wait for its release before the menu opens anyway. */
export const RELEASE_WAIT_MS = 1000;

/**
 * Runs `open` once the pressed button or finger comes up: a menu opened mid-press is in the top
 * layer when the release lands outside it, and the browser would light-dismiss it at once.
 */
export function afterRelease(pressed: boolean, open: () => void): void {
  if (!pressed) {
    open();

    return;
  }

  const done = () => {
    window.removeEventListener("pointerup", done, true);
    window.removeEventListener("pointercancel", done, true);
    window.clearTimeout(fallback);
    window.setTimeout(open, 0);
  };

  const fallback = window.setTimeout(done, RELEASE_WAIT_MS);

  window.addEventListener("pointerup", done, true);
  window.addEventListener("pointercancel", done, true);
}

interface Press {
  readonly x: number;
  readonly y: number;
  timer: number;
  /** The finger is still down and resting. */
  live: boolean;
  fired: boolean;
}

export interface LongPress {
  /** A pointer went down: a finger starts the timer; a mouse or pen forgets the last press. */
  readonly start: (event: PointerEvent<HTMLElement>) => void;
  /** A finger that drifts past the slop is scrolling: no long press. */
  readonly move: (event: PointerEvent<HTMLElement>) => void;
  readonly cancel: () => void;
  /** Whether the gesture that just ended opened the menu (its click shouldn't act). */
  readonly fired: () => boolean;
  /**
   * The browser's own long-press `contextmenu` (Android's) for a finger still resting: fires the
   * long press now, once, rather than opening a second menu. `false` when no finger is down, for
   * a right click to take its own path.
   */
  readonly claim: () => boolean;
}

/**
 * A long press on touch screens: a finger that rests `LONG_PRESS_MS` without drifting calls
 * `onLongPress` with where it rests, once it lifts (see `afterRelease`). Mouse and pen presses
 * are left to right click.
 */
export function useLongPress(onLongPress: (x: number, y: number) => void): LongPress {
  const press = useRef<Press | null>(null);
  const callback = useRef(onLongPress);

  useLayoutEffect(() => {
    callback.current = onLongPress;
  });

  useEffect(() => () => window.clearTimeout(press.current?.timer), []);

  const fire = (current: Press) => {
    window.clearTimeout(current.timer);
    current.fired = true;
    afterRelease(true, () => callback.current(current.x, current.y));
  };

  const cancel = () => {
    const current = press.current;

    if (current !== null) {
      window.clearTimeout(current.timer);
      current.live = false;
    }
  };

  return {
    start: (event) => {
      cancel();

      if (event.pointerType !== "touch") {
        press.current = null;

        return;
      }

      const current: Press = {
        x: event.clientX,
        y: event.clientY,
        timer: 0,
        live: true,
        fired: false,
      };

      current.timer = window.setTimeout(() => fire(current), LONG_PRESS_MS);
      press.current = current;
    },
    move: (event) => {
      const current = press.current;

      if (
        current !== null &&
        Math.hypot(event.clientX - current.x, event.clientY - current.y) > LONG_PRESS_SLOP
      ) {
        cancel();
      }
    },
    cancel,
    fired: () => press.current?.fired ?? false,
    claim: () => {
      const current = press.current;

      if (current === null || !current.live) {
        return false;
      }

      if (!current.fired) {
        fire(current);
      }

      return true;
    },
  };
}
