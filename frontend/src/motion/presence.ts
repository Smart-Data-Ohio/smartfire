import { type RefObject, useEffect, useRef, useState } from "react";
import { longestMotionMs } from "./durations.ts";

export type PresenceState = "open" | "closing";

/**
 * Calls `done` once the element's exit motion has played: on its own transitionend or
 * animationend, or when its longest computed transition or animation should have finished
 * (whichever comes first). An element with no motion finishes on the next frame. Returns a
 * cancel function.
 */
export function afterExit(element: HTMLElement, done: () => void): () => void {
  let finished = false;

  const finish = () => {
    if (finished) {
      return;
    }

    finished = true;
    cleanup();
    done();
  };

  const onEnd = (event: Event) => {
    if (event.target === element) {
      finish();
    }
  };

  const timer = window.setTimeout(finish, longestMotionMs(element) + 34);

  const cleanup = () => {
    window.clearTimeout(timer);
    element.removeEventListener("transitionend", onEnd);
    element.removeEventListener("animationend", onEnd);
  };

  element.addEventListener("transitionend", onEnd);
  element.addEventListener("animationend", onEnd);

  return () => {
    finished = true;
    cleanup();
  };
}

interface Presence<T extends HTMLElement> {
  readonly ref: RefObject<T | null>;
  /** Render the element while this is true. */
  readonly mounted: boolean;
  /** Put this on the element as `data-state`; CSS plays the enter on "open" and the exit on "closing". */
  readonly state: PresenceState;
}

/**
 * Keeps an overlay mounted through its exit motion: it mounts as soon as `open` turns true and
 * unmounts once the "closing" state's transition has finished. This is the recipes' "swap
 * .is-open for .is-closing, clean up after the close duration" orchestration, timed from the
 * element's computed style instead of a duplicated constant.
 */
export function usePresence<T extends HTMLElement>(open: boolean): Presence<T> {
  const ref = useRef<T | null>(null);
  const [mounted, setMounted] = useState(open);

  if (open && !mounted) {
    setMounted(true);
  }

  useEffect(() => {
    if (open || !mounted) {
      return;
    }

    const element = ref.current;

    if (element === null) {
      setMounted(false);

      return;
    }

    return afterExit(element, () => setMounted(false));
  }, [open, mounted]);

  return { ref, mounted, state: open ? "open" : "closing" };
}
