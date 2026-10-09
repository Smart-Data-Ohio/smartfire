import {
  type MouseEvent,
  type PointerEvent,
  type RefObject,
  useEffect,
  useLayoutEffect,
  useRef,
} from "react";
import { useLongPress } from "../../lib/long-press.ts";
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

interface Drag {
  readonly pointerId: number;
  readonly x: number;
  readonly y: number;
  swiping: boolean;
  armed: boolean;
}

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
   * press has it, so the row's right-click menu shouldn't open as well.
   */
  readonly claimContextMenu: () => boolean;
}

/**
 * Touch gestures for a destination row, as Slack's lists have them: a long press opens the row's
 * menu (an action sheet on phones) and a sideways drag towards the start edge slides the row
 * over its one swipe action, done on release past the line. A vertical drag scrolls the list as
 * usual (the row is `touch-action: pan-y`). The press's click is swallowed after either, so the
 * row doesn't open too. Mouse and pen are left alone: they have hover actions and right click.
 */
export function useRowGestures(
  rowRef: RefObject<HTMLElement | null>,
  innerRef: RefObject<HTMLElement | null>,
  swipe: RowSwipe | undefined,
  onLongPress: ((x: number, y: number) => void) | undefined,
): RowGestures {
  const longPress = useLongPress((x, y) => onLongPress?.(x, y));
  const drag = useRef<Drag | null>(null);
  const swallowClick = useRef(false);
  const timers = useRef<number[]>([]);
  const swipeRef = useRef(swipe);

  useLayoutEffect(() => {
    swipeRef.current = swipe;
  });

  useEffect(
    () => () => {
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

  const end = (event: PointerEvent<HTMLElement>, released: boolean) => {
    longPress.cancel();

    const current = drag.current;

    if (current === null || current.pointerId !== event.pointerId) {
      return;
    }

    drag.current = null;

    if (!current.swiping) {
      return;
    }

    const action = swipeRef.current;

    if (released && current.armed && action !== undefined) {
      commit(action);
    } else {
      settleBack();
    }
  };

  return {
    props: {
      onPointerDown: (event) => {
        swallowClick.current = false;

        if (onLongPress !== undefined) {
          longPress.start(event);
        }

        if (event.pointerType !== "touch" || !event.isPrimary || swipeRef.current === undefined) {
          drag.current = null;

          return;
        }

        clearTimers();
        drag.current = {
          pointerId: event.pointerId,
          x: event.clientX,
          y: event.clientY,
          swiping: false,
          armed: false,
        };
      },
      onPointerMove: (event) => {
        longPress.move(event);

        const current = drag.current;
        const inner = innerRef.current;

        if (current === null || current.pointerId !== event.pointerId || inner === null) {
          return;
        }

        const dx = event.clientX - current.x;
        const dy = event.clientY - current.y;

        if (!current.swiping) {
          if (Math.abs(dy) > SWIPE_START && Math.abs(dy) > Math.abs(dx)) {
            // A scroll: the browser takes it from here.
            drag.current = null;

            return;
          }

          if (dx > -SWIPE_START || Math.abs(dx) <= Math.abs(dy)) {
            return;
          }

          current.swiping = true;
          swallowClick.current = true;
          longPress.cancel();
          setState("dragging");
        }

        const width = inner.getBoundingClientRect().width;
        const offset = Math.min(0, Math.max(dx, -width));
        const armed = -offset >= Math.min(SWIPE_LINE_MAX, width * SWIPE_LINE_SHARE);

        inner.style.setProperty("translate", `${offset}px 0`);

        if (armed !== current.armed) {
          current.armed = armed;
          setState(armed ? "armed" : "dragging");
        }
      },
      onPointerUp: (event) => {
        if (longPress.fired()) {
          swallowClick.current = true;
        }

        end(event, true);
      },
      onPointerCancel: (event) => end(event, false),
      // A key's click (detail 0) still opens the row.
      onClickCapture: (event) => {
        if (event.detail !== 0 && (swallowClick.current || longPress.fired())) {
          swallowClick.current = false;
          event.preventDefault();
          event.stopPropagation();
        }
      },
    },
    claimContextMenu: () => onLongPress !== undefined && longPress.claim(),
  };
}
