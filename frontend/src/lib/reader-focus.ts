/**
 * Whether a focus move inside the timeline belongs to the reader.
 *
 * Tab changes focus after keydown, so the viewport anchor cannot see it on the key itself.
 * A focusin counts only when the last keydown in this window was Tab (with or without Shift)
 * and no other key followed it, or when the last pointerdown landed on the newly focused
 * element or an ancestor of it. Escape, Enter, Space and every other key are the app moving
 * focus. `duringAppFocus` still covers a `focus()` the app already knows is its own.
 */

/** How recently a Tab or a click still explains the focus that follows it. */
const READER_FOCUS_WINDOW_MS = 100;

let lastKey: { readonly key: string; readonly at: number } | null = null;

let lastPointer: { readonly target: EventTarget | null; readonly at: number } | null = null;

let appFocus = 0;

let listeners = 0;

function onKeyDown(event: Event): void {
  if (!(event instanceof KeyboardEvent)) {
    return;
  }

  lastKey = { key: event.key, at: performance.now() };
}

function onPointerDown(event: Event): void {
  lastPointer = { target: event.target, at: performance.now() };
}

function forgetInput(): void {
  lastKey = null;
  lastPointer = null;
}

/**
 * Notices key and pointer presses on `doc` until the returned function runs. Several timelines
 * share one pair of listeners. The last release forgets the press, so a later test or screen
 * cannot inherit it.
 */
export function bindReaderInput(doc: Document): () => void {
  if (listeners === 0) {
    doc.addEventListener("keydown", onKeyDown, true);
    doc.addEventListener("pointerdown", onPointerDown, true);
  }

  listeners += 1;

  let released = false;

  return () => {
    if (released) {
      return;
    }

    released = true;
    listeners -= 1;

    if (listeners === 0) {
      doc.removeEventListener("keydown", onKeyDown, true);
      doc.removeEventListener("pointerdown", onPointerDown, true);
      forgetInput();
    }
  };
}

/** Marks `focus()` the app is about to call, covering the focusin that call fires. */
export function duringAppFocus<T>(run: () => T): T {
  appFocus += 1;

  try {
    return run();
  } finally {
    appFocus -= 1;
  }
}

/**
 * A focusin that followed the person's Tab, or a click on `focused` (or an ancestor of it).
 * Any other key, and a click that landed somewhere else, is the app.
 */
export function readerMovedFocus(
  focused: EventTarget | null = null,
  at = performance.now(),
): boolean {
  if (appFocus !== 0) {
    return false;
  }

  if (lastKey?.key === "Tab" && at - lastKey.at <= READER_FOCUS_WINDOW_MS) {
    return true;
  }

  return (
    focused instanceof Node &&
    lastPointer !== null &&
    lastPointer.target instanceof Node &&
    at - lastPointer.at <= READER_FOCUS_WINDOW_MS &&
    lastPointer.target.contains(focused)
  );
}
