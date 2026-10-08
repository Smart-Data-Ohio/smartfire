import { useEffect, useRef } from "react";

/** How long focus follows the source row after the dialog closes. */
export const SOURCE_ROW_WAIT_MS = 5000;

export interface SourceFollower {
  /** The row as it is now (`null` until it mounts). */
  readonly row: HTMLElement | null;
  /** Stops following: disconnects the observer and listeners, and cancels the timer and frame. */
  readonly stop: () => void;
}

/**
 * Focus after the dialog: the source row. Closing can load the conversation around it (a direct
 * entry's permalink, or an older thread reply), which mounts the row late or replaces it, so for a
 * moment focus follows the row: whenever the row is replaced or focus falls back to the page or to
 * a container, focus returns to it.
 *
 * It stops at the first sign the viewer has moved on: a key press or a pointer press anywhere
 * (rows and menu items are reachable by keyboard and pointer without being tabbable), or a
 * control the viewer can reach (tabIndex 0 or more) taking focus. It also stops after `waitMs`,
 * and when its owner calls `stop`.
 */
export function followSourceRow(
  find: () => HTMLElement | null,
  waitMs: number = SOURCE_ROW_WAIT_MS,
): SourceFollower {
  let stopped = false;
  let frame: number | null = null;

  const follow = () => {
    frame = null;

    if (stopped) {
      return;
    }

    const active = document.activeElement;

    if (active instanceof HTMLElement && active !== document.body && active.tabIndex >= 0) {
      stop();

      return;
    }

    const row = find();

    if (row !== null && active !== row) {
      row.focus({ preventScroll: true });
    }
  };

  // Focus moving lands its focusin before the move settles (a pane focusing itself after a
  // reload, say), so the follower looks on the next frame.
  const onFocusIn = () => {
    if (frame === null) {
      frame = requestAnimationFrame(follow);
    }
  };

  const observer = new MutationObserver(follow);
  const timer = window.setTimeout(() => stop(), waitMs);

  function stop() {
    if (stopped) {
      return;
    }

    stopped = true;
    observer.disconnect();
    document.removeEventListener("focusin", onFocusIn);
    document.removeEventListener("keydown", stop, true);
    document.removeEventListener("pointerdown", stop, true);
    window.clearTimeout(timer);

    if (frame !== null) {
      cancelAnimationFrame(frame);
      frame = null;
    }
  }

  observer.observe(document.body, { childList: true, subtree: true });
  document.addEventListener("focusin", onFocusIn);
  // Capturing, so a handler that stops propagation can't hide the viewer's intent.
  document.addEventListener("keydown", stop, true);
  document.addEventListener("pointerdown", stop, true);

  return { row: find(), stop };
}

/**
 * A component's source-row follower: `follow` starts one (ending the previous). A new `opening`
 * (any non-null value: the dialog showing again) ends it, and so does unmounting, so nothing
 * outlives the moment it was for.
 */
export function useSourceFollower(opening: number | null) {
  const current = useRef<SourceFollower | null>(null);

  const follow = (find: () => HTMLElement | null) => {
    current.current?.stop();
    current.current = followSourceRow(find);

    return current.current.row;
  };

  useEffect(() => {
    if (opening !== null) {
      current.current?.stop();
      current.current = null;
    }
  }, [opening]);

  useEffect(
    () => () => {
      current.current?.stop();
      current.current = null;
    },
    [],
  );

  return { follow };
}
