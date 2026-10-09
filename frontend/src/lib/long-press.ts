import { type PointerEvent as ReactPointerEvent, useEffect, useRef } from "react";

/** How long a finger rests on a control before its long press fires. */
export const LONG_PRESS_MS = 500;

/** How far a resting finger may drift before it's a scroll, not a press. */
const LONG_PRESS_SLOP = 10;

/** How long a fired press waits for its finger to lift before it opens anyway. */
const RELEASE_WAIT_MS = 1000;

/** How long after the finger lifts the click it ends in may still arrive (and be swallowed). */
const CLICK_WINDOW_MS = 350;

interface Press {
  readonly x: number;
  readonly y: number;
  readonly timer: number;
}

export interface LongPress {
  /** Spread onto the control. */
  readonly handlers: {
    readonly onPointerDown: (event: ReactPointerEvent<HTMLElement>) => void;
    readonly onPointerMove: (event: ReactPointerEvent<HTMLElement>) => void;
    readonly onPointerUp: () => void;
    readonly onPointerCancel: () => void;
    readonly onContextMenu: (event: { preventDefault: () => void }) => void;
  };
  /** In the control's click handler: whether this click ends a long press, so it does nothing. */
  readonly endsLongPress: () => boolean;
}

/**
 * A long press with a finger: `onLongPress` runs once the finger lifts (a popover opened while it
 * is still down would be light-dismissed by that very release), and the click the gesture ends
 * in is the caller's to swallow (`endsLongPress`). Mouse and pen presses, and presses while
 * `enabled()` is false, are ordinary clicks.
 */
export function useLongPress(onLongPress: () => void, enabled: () => boolean): LongPress {
  const press = useRef<Press | null>(null);
  const firedAt = useRef<number | null>(null);
  const releasedAt = useRef(Number.NEGATIVE_INFINITY);
  const stopWaiting = useRef<(() => void) | null>(null);
  const callback = useRef(onLongPress);

  callback.current = onLongPress;

  useEffect(
    () => () => {
      window.clearTimeout(press.current?.timer);
      stopWaiting.current?.();
    },
    [],
  );

  const cancel = () => {
    window.clearTimeout(press.current?.timer);
    press.current = null;
  };

  const fire = () => {
    press.current = null;

    const fired = performance.now();
    let opened = false;

    firedAt.current = fired;

    const open = () => {
      if (!opened) {
        opened = true;
        callback.current();
      }
    };

    const fallback = window.setTimeout(open, RELEASE_WAIT_MS);

    const release = () => {
      stopWaiting.current?.();
      releasedAt.current = performance.now();
      window.setTimeout(open, 0);
    };

    window.addEventListener("pointerup", release, true);
    window.addEventListener("pointercancel", release, true);
    stopWaiting.current = () => {
      window.removeEventListener("pointerup", release, true);
      window.removeEventListener("pointercancel", release, true);
      window.clearTimeout(fallback);
      stopWaiting.current = null;
    };
  };

  return {
    handlers: {
      onPointerDown: (event) => {
        cancel();
        firedAt.current = null;

        if (event.pointerType !== "touch" || !enabled()) {
          return;
        }

        press.current = {
          x: event.clientX,
          y: event.clientY,
          timer: window.setTimeout(fire, LONG_PRESS_MS),
        };
      },
      onPointerMove: (event) => {
        const current = press.current;

        if (
          current !== null &&
          Math.hypot(event.clientX - current.x, event.clientY - current.y) > LONG_PRESS_SLOP
        ) {
          cancel();
        }
      },
      onPointerUp: cancel,
      onPointerCancel: cancel,
      // Android raises the context menu for a long press: the press is ours.
      onContextMenu: (event) => {
        if (press.current !== null || firedAt.current !== null) {
          event.preventDefault();
        }
      },
    },
    endsLongPress: () => {
      const fired = firedAt.current;

      firedAt.current = null;

      // Still held (the menu opened on the fallback), or lifted a moment ago.
      return (
        fired !== null &&
        (releasedAt.current < fired || performance.now() - releasedAt.current < CLICK_WINDOW_MS)
      );
    },
  };
}
