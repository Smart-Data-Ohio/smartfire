import {
  type MouseEvent,
  type PointerEvent,
  type RefObject,
  useEffect,
  useLayoutEffect,
  useRef,
} from "react";
import { LONG_PRESS_MS, LONG_PRESS_SLOP, RELEASE_WAIT_MS } from "../../lib/long-press.ts";
import { readDurationMs } from "../../motion/durations.ts";
import type { IconName } from "../../ui/icons/icon.tsx";

/** A row's one swipe: drag it towards the start edge and let go past the line to do this. */
export interface RowSwipe {
  /** What shows under the row as it slides ("Done"); its menu and keys carry the same action. */
  readonly label: string;
  readonly icon: IconName;
  readonly tone: "success" | "danger" | "neutral";
  readonly onSwipe: () => void;
}

/** How far a finger travels sideways before the row follows it (past the long press's slop). */
const SWIPE_START = 10;

/** Past this share of the row's width (or `SWIPE_LINE_MAX` px) a release does the action. */
const SWIPE_LINE_SHARE = 0.35;

const SWIPE_LINE_MAX = 120;

/** `data-swipe` on the row: following a finger, past the line, done (sliding away), easing back. */
type SwipeState = "dragging" | "armed" | "done" | "settling";

/**
 * One finger's gesture on the row, from pointerdown to pointerup or cancel. Whichever of the long
 * press and the swipe claims the finger first owns it until it lifts: `pending` is the only state
 * either can start from, and leaving it clears the other's way in.
 * - `pending`: down, nothing claimed; `timer` is the long press's (0 once the finger drifted).
 * - `pressed`: the long press fired; the menu opens on release (`fallback` if none comes).
 * - `swiping`: the row follows the finger sideways.
 * - `released`: nothing more this press: a scroll took it, or (`menu`) the menu already opened.
 */
type Gesture =
  | {
      readonly kind: "pending";
      readonly pointerId: number;
      readonly x: number;
      readonly y: number;
      readonly timer: number;
    }
  | {
      readonly kind: "pressed";
      readonly pointerId: number;
      readonly x: number;
      readonly y: number;
      readonly fallback: number;
    }
  | { readonly kind: "swiping"; readonly pointerId: number; readonly x: number; armed: boolean }
  | { readonly kind: "released"; readonly pointerId: number; readonly menu: boolean };

interface RowGestureProps {
  readonly onPointerDown: (event: PointerEvent<HTMLElement>) => void;
  readonly onPointerMove: (event: PointerEvent<HTMLElement>) => void;
  readonly onPointerUp: (event: PointerEvent<HTMLElement>) => void;
  readonly onPointerCancel: (event: PointerEvent<HTMLElement>) => void;
  readonly onClickCapture: (event: MouseEvent<HTMLElement>) => void;
}

export interface RowGestures {
  readonly props: RowGestureProps;
  /**
   * The browser's own long-press menu (Android's `contextmenu`) during a finger's press: the long
   * press has it (firing now if it hadn't yet), so the row's right-click menu shouldn't open as
   * well. `false` with no finger down, for a right click or the menu key to take their own path.
   */
  readonly claimContextMenu: () => boolean;
}

/** Cancels whatever the gesture has pending: the long press's timer or the release fallback. */
function clearGestureTimers(current: Gesture | null) {
  if (current?.kind === "pending") {
    window.clearTimeout(current.timer);
  } else if (current?.kind === "pressed") {
    window.clearTimeout(current.fallback);
  }
}

/**
 * Touch gestures for a destination row, as Slack's lists have them: a long press opens the row's
 * menu (an action sheet on phones) and a sideways drag towards the start edge slides the row
 * over its one swipe action, done on release past the line. They exclude each other: a press
 * held past `LONG_PRESS_MS` no longer swipes, and a swipe under way no longer long-presses. A
 * vertical drag scrolls the list as usual (the row is `touch-action: pan-y`). The press's click
 * is swallowed after either, so the row doesn't open too. Mouse and pen are left alone: they
 * have hover actions and right click.
 */
export function useRowGestures(
  rowRef: RefObject<HTMLElement | null>,
  innerRef: RefObject<HTMLElement | null>,
  swipe: RowSwipe | undefined,
  onLongPress: ((x: number, y: number) => void) | undefined,
): RowGestures {
  const gesture = useRef<Gesture | null>(null);
  const swallowClick = useRef(false);
  const timers = useRef<number[]>([]);
  const swipeRef = useRef(swipe);
  const longPressRef = useRef(onLongPress);

  useLayoutEffect(() => {
    swipeRef.current = swipe;
    longPressRef.current = onLongPress;
  });

  useEffect(
    () => () => {
      clearGestureTimers(gesture.current);

      for (const timer of timers.current) {
        window.clearTimeout(timer);
      }
    },
    [],
  );

  const setState = (state: SwipeState | null) => {
    const row = rowRef.current;

    if (row === null) {
      return;
    }

    if (state === null) {
      delete row.dataset.swipe;
    } else {
      row.dataset.swipe = state;
    }
  };

  const later = (ms: number, run: () => void) => {
    timers.current.push(window.setTimeout(run, ms));
  };

  const clearTimers = () => {
    for (const timer of timers.current) {
      window.clearTimeout(timer);
    }

    timers.current = [];
  };

  /** Eases the row back into place. */
  const settleBack = () => {
    setState("settling");
    innerRef.current?.style.removeProperty("translate");
    later(readDurationMs("--duration-small"), () => setState(null));
  };

  /**
   * Slides the row away and does the action. A row the action takes out of the list folds away
   * from there; one that stays (under "All") slides back once the change shows.
   */
  const commit = (action: RowSwipe) => {
    setState("done");
    innerRef.current?.style.setProperty("translate", "-100% 0");
    action.onSwipe();
    later(readDurationMs("--duration-medium"), () => {
      if (rowRef.current?.dataset.motion !== "leave") {
        settleBack();
      }
    });
  };

  /**
   * Opens the menu where the finger rested, once. After the release's own events: a popover
   * opened mid-press would be light-dismissed by the release landing outside it.
   */
  const openMenu = (x: number, y: number) => {
    window.setTimeout(() => longPressRef.current?.(x, y), 0);
  };

  /** The long press claims the finger: from `pending` only, so never during a swipe. */
  const press = () => {
    const current = gesture.current;

    if (current?.kind !== "pending") {
      return;
    }

    window.clearTimeout(current.timer);
    swallowClick.current = true;

    const { pointerId, x, y } = current;

    // The release opens the menu; a release that never reaches the row (it was removed) doesn't
    // leave it waiting.
    const fallback = window.setTimeout(() => {
      if (gesture.current?.kind === "pressed" && gesture.current.pointerId === pointerId) {
        gesture.current = { kind: "released", pointerId, menu: true };
        openMenu(x, y);
      }
    }, RELEASE_WAIT_MS);

    gesture.current = { kind: "pressed", pointerId, x, y, fallback };
  };

  const end = (event: PointerEvent<HTMLElement>, released: boolean) => {
    const current = gesture.current;

    if (current === null || current.pointerId !== event.pointerId) {
      return;
    }

    clearGestureTimers(gesture.current);
    gesture.current = null;

    if (current.kind === "pressed") {
      openMenu(current.x, current.y);
    } else if (current.kind === "swiping") {
      const action = swipeRef.current;

      if (released && current.armed && action !== undefined) {
        commit(action);
      } else {
        settleBack();
      }
    }
  };

  return {
    props: {
      onPointerDown: (event) => {
        if (!event.isPrimary) {
          return;
        }

        clearGestureTimers(gesture.current);
        gesture.current = null;
        swallowClick.current = false;

        const canPress = longPressRef.current !== undefined;

        if (event.pointerType !== "touch" || (!canPress && swipeRef.current === undefined)) {
          return;
        }

        clearTimers();
        gesture.current = {
          kind: "pending",
          pointerId: event.pointerId,
          x: event.clientX,
          y: event.clientY,
          timer: canPress ? window.setTimeout(press, LONG_PRESS_MS) : 0,
        };
      },
      onPointerMove: (event) => {
        const current = gesture.current;

        if (current === null || current.pointerId !== event.pointerId) {
          return;
        }

        if (current.kind === "pending") {
          const dx = event.clientX - current.x;
          const dy = event.clientY - current.y;

          if (Math.abs(dy) > SWIPE_START && Math.abs(dy) > Math.abs(dx)) {
            // A scroll: the browser takes it from here.
            window.clearTimeout(current.timer);
            gesture.current = { kind: "released", pointerId: current.pointerId, menu: false };

            return;
          }

          if (swipeRef.current !== undefined && dx < -SWIPE_START && Math.abs(dx) > Math.abs(dy)) {
            window.clearTimeout(current.timer);
            gesture.current = {
              kind: "swiping",
              pointerId: current.pointerId,
              x: current.x,
              armed: false,
            };
            swallowClick.current = true;
            setState("dragging");
          } else {
            if (current.timer !== 0 && Math.hypot(dx, dy) > LONG_PRESS_SLOP) {
              // Drifting: no long press this time, though it may still turn into a swipe.
              window.clearTimeout(current.timer);
              gesture.current = { ...current, timer: 0 };
            }

            return;
          }
        }

        const swiping = gesture.current;
        const inner = innerRef.current;

        if (swiping?.kind !== "swiping" || inner === null) {
          return;
        }

        const width = inner.getBoundingClientRect().width;
        const offset = Math.min(0, Math.max(event.clientX - swiping.x, -width));
        const armed = -offset >= Math.min(SWIPE_LINE_MAX, width * SWIPE_LINE_SHARE);

        inner.style.setProperty("translate", `${offset}px 0`);

        if (armed !== swiping.armed) {
          swiping.armed = armed;
          setState(armed ? "armed" : "dragging");
        }
      },
      onPointerUp: (event) => end(event, true),
      onPointerCancel: (event) => end(event, false),
      // A key's click (detail 0) still opens the row.
      onClickCapture: (event) => {
        if (event.detail !== 0 && swallowClick.current) {
          swallowClick.current = false;
          event.preventDefault();
          event.stopPropagation();
        }
      },
    },
    claimContextMenu: () => {
      const current = gesture.current;

      if (longPressRef.current === undefined || current === null) {
        return false;
      }

      if (current.kind === "pending") {
        press();

        return true;
      }

      return current.kind !== "released" || current.menu;
    },
  };
}
