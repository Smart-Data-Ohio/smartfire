import {
  type PointerEvent,
  type RefObject,
  useEffect,
  useLayoutEffect,
  useRef,
  useSyncExternalStore,
} from "react";
import { COARSE_QUERY, PHONE_QUERY } from "../lib/breakpoints.ts";
import "./action-sheet.css";

/**
 * Where menus and popovers open as bottom action sheets instead of beside their trigger: a touch
 * screen at phone width, as in Slack and Discord. A phone-sized window with a mouse keeps the
 * anchored surfaces, shortcut hints and all.
 */
export const SHEET_QUERY = `${PHONE_QUERY} and ${COARSE_QUERY}`;

/** jsdom (the unit tests) has a `matchMedia` property but no function behind it. */
function canMatchMedia(): boolean {
  return "matchMedia" in window && window.matchMedia instanceof Function;
}

function subscribe(onChange: () => void): () => void {
  if (!canMatchMedia()) {
    return () => {};
  }

  const media = window.matchMedia(SHEET_QUERY);

  media.addEventListener("change", onChange);

  return () => media.removeEventListener("change", onChange);
}

function matches(): boolean {
  return canMatchMedia() && window.matchMedia(SHEET_QUERY).matches;
}

/** Whether a Menu or Popover opening now should be an action sheet; follows rotation and resizes. */
export function useActionSheet(): boolean {
  return useSyncExternalStore(subscribe, matches, () => false);
}

/** The rest of a press already handled: its release, and the click or menu it ends in. */
const PRESS_TAIL = ["pointerup", "click", "contextmenu"] as const;

/** Swallows the tail of the current press, however late it comes; the next press stops it. */
function swallowPressTail(): void {
  const swallow = (event: Event) => {
    event.preventDefault();
    event.stopPropagation();
  };

  const stop = () => {
    for (const type of PRESS_TAIL) {
      window.removeEventListener(type, swallow, true);
    }

    window.removeEventListener("pointerdown", stop, true);
  };

  for (const type of PRESS_TAIL) {
    window.addEventListener(type, swallow, true);
  }

  window.addEventListener("click", stop, { capture: true, once: true });
  // Registered after this press's own pointerdown has passed, so only the next one stops it.
  window.setTimeout(() => window.addEventListener("pointerdown", stop, true), 0);
}

/**
 * The scrim behind an open sheet. A popover's ::backdrop lets taps through (the UA stylesheet
 * gives it `pointer-events: none`), so a tap on it would also press whatever lies under it: a
 * header button, a message's long press. While `active`, a press outside the sheet only dismisses
 * it: the press, its compatibility mouse events (so focus stays put) and its click are swallowed.
 */
export function useSheetScrim(
  active: boolean,
  sheetRef: RefObject<HTMLElement | null>,
  onDismiss: () => void,
): void {
  const onDismissRef = useRef(onDismiss);

  useLayoutEffect(() => {
    onDismissRef.current = onDismiss;
  });

  useEffect(() => {
    if (!active) {
      return;
    }

    const onPointerDown = (event: globalThis.PointerEvent) => {
      const sheet = sheetRef.current;
      const target = event.target;

      if (sheet === null || !(target instanceof Node) || sheet.contains(target)) {
        return;
      }

      event.preventDefault();
      event.stopPropagation();
      swallowPressTail();
      onDismissRef.current();
    };

    window.addEventListener("pointerdown", onPointerDown, true);

    return () => window.removeEventListener("pointerdown", onPointerDown, true);
  }, [active, sheetRef]);
}

/** How far the handle must be pulled down to dismiss its sheet. */
const DISMISS_DISTANCE = 64;

interface Drag {
  readonly pointerId: number;
  readonly startY: number;
  readonly sheet: HTMLElement;
  distance: number;
}

/**
 * The grab handle across a sheet's top edge. Pulling it down drags the sheet along, and letting
 * go far enough dismisses it; a shorter pull springs back. The sheet's body scrolls as usual.
 */
export function SheetHandle({ onDismiss }: { readonly onDismiss: () => void }) {
  const drag = useRef<Drag | null>(null);

  const end = (event: PointerEvent<HTMLElement>, released: boolean) => {
    const current = drag.current;

    if (current === null || current.pointerId !== event.pointerId) {
      return;
    }

    drag.current = null;
    current.sheet.style.removeProperty("translate");
    delete current.sheet.dataset.dragging;

    if (released && current.distance >= DISMISS_DISTANCE) {
      onDismiss();
    }
  };

  return (
    <div
      className="action-sheet-handle"
      aria-hidden="true"
      onPointerDown={(event) => {
        const sheet = event.currentTarget.closest<HTMLElement>(".action-sheet");

        if (sheet === null || event.button !== 0) {
          return;
        }

        event.currentTarget.setPointerCapture(event.pointerId);
        sheet.dataset.dragging = "";
        drag.current = { pointerId: event.pointerId, startY: event.clientY, sheet, distance: 0 };
      }}
      onPointerMove={(event) => {
        const current = drag.current;

        if (current === null || current.pointerId !== event.pointerId) {
          return;
        }

        current.distance = Math.max(0, event.clientY - current.startY);
        current.sheet.style.setProperty("translate", `0 ${current.distance}px`);
      }}
      onPointerUp={(event) => end(event, true)}
      onPointerCancel={(event) => end(event, false)}
    />
  );
}
