import { type PointerEvent as ReactPointerEvent, useEffect, useRef } from "react";

/** How long a finger rests on a control before its long press fires. */
export const LONG_PRESS_MS = 500;

/** How far a resting finger may drift before it's a scroll, not a press. */
const LONG_PRESS_SLOP = 10;

/** How long after the finger lifts the click it ends in may still arrive (and be swallowed). */
const CLICK_WINDOW_MS = 350;

interface Press {
  readonly pointerId: number;
  readonly x: number;
  readonly y: number;
  readonly timer: number;
  readonly unlisten: () => void;
}

export interface LongPress {
  /** Spread onto the control. */
  readonly handlers: {
    readonly onPointerDown: (event: ReactPointerEvent<HTMLElement>) => void;
    readonly onContextMenu: (event: { preventDefault: () => void }) => void;
  };
  /** In the control's click handler: whether this click ends a long press, so it does nothing. */
  readonly endsLongPress: () => boolean;
}

/**
 * A long press with a finger. `onLongPress` runs only when that finger lifts after resting
 * `LONG_PRESS_MS` (a popover opened while it is still down would be light-dismissed by that very
 * release); drifting past the slop or a `pointercancel` (the browser took the touch for a scroll)
 * drops the press at any point before then, fired or not. The click the gesture ends in is the
 * caller's to swallow (`endsLongPress`). Mouse and pen presses, and presses while `enabled()` is
 * false, are ordinary clicks.
 */
export function useLongPress(onLongPress: () => void, enabled: () => boolean): LongPress {
  const press = useRef<Press | null>(null);
  const firedAt = useRef<number | null>(null);
  const endedAt = useRef(Number.NEGATIVE_INFINITY);
  const callback = useRef(onLongPress);
  const allowed = useRef(enabled);

  callback.current = onLongPress;
  allowed.current = enabled;

  /** Drops the press in flight; `open` lifts it into the long press, if that has fired. */
  const end = (open: boolean) => {
    const current = press.current;

    if (current === null) {
      return;
    }

    window.clearTimeout(current.timer);
    current.unlisten();
    press.current = null;

    if (firedAt.current === null) {
      return;
    }

    endedAt.current = performance.now();

    if (open) {
      // After the release's own events, so they can't light-dismiss what opens.
      window.setTimeout(() => {
        if (allowed.current()) {
          callback.current();
        }
      }, 0);
    }
  };

  const endRef = useRef(end);

  endRef.current = end;

  useEffect(() => () => endRef.current(false), []);

  const start = (event: ReactPointerEvent<HTMLElement>) => {
    const { pointerId, clientX: x, clientY: y } = event;

    const ours = (pointer: PointerEvent) => pointer.pointerId === pointerId;

    const onMove = (pointer: PointerEvent) => {
      if (ours(pointer) && Math.hypot(pointer.clientX - x, pointer.clientY - y) > LONG_PRESS_SLOP) {
        endRef.current(false);
      }
    };

    const onUp = (pointer: PointerEvent) => {
      if (ours(pointer)) {
        endRef.current(true);
      }
    };

    const onCancel = (pointer: PointerEvent) => {
      if (ours(pointer)) {
        endRef.current(false);
      }
    };

    window.addEventListener("pointermove", onMove, true);
    window.addEventListener("pointerup", onUp, true);
    window.addEventListener("pointercancel", onCancel, true);

    press.current = {
      pointerId,
      x,
      y,
      timer: window.setTimeout(() => {
        firedAt.current = performance.now();
      }, LONG_PRESS_MS),
      unlisten: () => {
        window.removeEventListener("pointermove", onMove, true);
        window.removeEventListener("pointerup", onUp, true);
        window.removeEventListener("pointercancel", onCancel, true);
      },
    };
  };

  return {
    handlers: {
      onPointerDown: (event) => {
        end(false);
        firedAt.current = null;

        if (event.pointerType === "touch" && enabled()) {
          start(event);
        }
      },
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

      // Still held, or lifted (or dropped) a moment ago.
      return (
        fired !== null &&
        (press.current !== null || performance.now() - endedAt.current < CLICK_WINDOW_MS)
      );
    },
  };
}
