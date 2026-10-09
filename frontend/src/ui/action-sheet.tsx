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

/** How long after its release a press's click may still come; a tap's can lag a frame or two. */
const CLICK_WAIT_MS = 1000;

/**
 * Swallows the rest of the press `pointerId` began, which has already done its work: its release,
 * a long press's context menu, and the click it ends in. Only that pointer's events are touched.
 * A cancelled press (a scroll, a lost capture) has no click to wait for, so it ends there; the
 * click ends it otherwise, or the next press, or a second after the release. A click from a key
 * or an assistive tool (detail 0, no pointer) is never swallowed.
 */
function swallowPressTail(pointerId: number): void {
  let released = false;
  let timer = 0;

  const fromPointer = (event: Event) =>
    !(event instanceof globalThis.PointerEvent) || event.pointerId === pointerId;

  const ours = (event: Event) =>
    event instanceof globalThis.PointerEvent && event.pointerId === pointerId;

  const swallow = (event: Event) => {
    event.preventDefault();
    event.stopPropagation();
  };

  const onUp = (event: Event) => {
    if (!released && ours(event)) {
      swallow(event);
      released = true;
      timer = window.setTimeout(stop, CLICK_WAIT_MS);
    }
  };

  // After the release, a touch pointer's implicit capture is let go too, and the click still comes.
  const onCancel = (event: Event) => {
    if (!released && ours(event)) {
      stop();
    }
  };

  const onContextMenu = (event: Event) => {
    if (!released && fromPointer(event)) {
      swallow(event);
    }
  };

  // Chromium sends a pointer's click as a PointerEvent with its pointerId; a key's has detail 0.
  const onClick = (event: MouseEvent) => {
    if (released && event.detail > 0 && fromPointer(event)) {
      swallow(event);
      stop();
    }
  };

  const listeners = [
    ["pointerup", onUp],
    ["pointercancel", onCancel],
    ["lostpointercapture", onCancel],
    ["contextmenu", onContextMenu],
    ["click", onClick],
  ] as const;

  function stop() {
    window.clearTimeout(timer);

    for (const [type, listener] of listeners) {
      window.removeEventListener(type, listener, true);
    }

    window.removeEventListener("pointerdown", stop, true);
  }

  for (const [type, listener] of listeners) {
    window.addEventListener(type, listener, true);
  }

  // Registered once this press's own pointerdown has passed, so only the next press stops it.
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
      swallowPressTail(event.pointerId);
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

        // jsdom (the unit tests) has no pointer capture.
        if ("setPointerCapture" in event.currentTarget) {
          event.currentTarget.setPointerCapture(event.pointerId);
        }

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
