/**
 * Whether a focus move inside the timeline belongs to the reader.
 *
 * Tab changes focus after keydown, so the viewport anchor cannot see it on the key itself.
 * One slot remembers the latest keydown or pointerdown. A focusin counts only when that
 * latest input is a Tab (with or without Shift) inside the window, or a pointerdown inside
 * the window on the newly focused element or an ancestor of it. A click after Tab replaces
 * the Tab. Escape, Enter, Space and every other key are the app moving focus.
 * `duringAppFocus` still covers a `focus()` the app already knows is its own.
 */

/** How recently a Tab or a click still explains the focus that follows it. */
const READER_FOCUS_WINDOW_MS = 100;

type LastInput =
  | { readonly kind: "key"; readonly key: string; readonly at: number }
  | { readonly kind: "pointer"; readonly target: EventTarget | null; readonly at: number };

let lastInput: LastInput | null = null;

let appFocus = 0;

let listeners = 0;

/** Wall time, unless a test has installed its own clock. */
let readNow = (): number => performance.now();

function onKeyDown(event: Event): void {
  if (!(event instanceof KeyboardEvent)) {
    return;
  }

  lastInput = { kind: "key", key: event.key, at: readNow() };
}

function onPointerDown(event: Event): void {
  lastInput = { kind: "pointer", target: event.target, at: readNow() };
}

function forgetInput(): void {
  lastInput = null;
}

/**
 * Drops the remembered press and the app-focus depth. `read`, when given, is the clock until
 * the next reset, so a test can hold the window still instead of following wall time.
 */
export function resetReaderFocusForTests(read: () => number = () => performance.now()): void {
  lastInput = null;
  appFocus = 0;
  readNow = read;
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
 * A focusin that followed the person's latest input when that input was Tab, or a click
 * on `focused` (or an ancestor of it). A later key or click replaces the earlier one.
 */
export function readerMovedFocus(focused: EventTarget | null = null, at = readNow()): boolean {
  if (appFocus !== 0 || lastInput === null || at - lastInput.at > READER_FOCUS_WINDOW_MS) {
    return false;
  }

  if (lastInput.kind === "key") {
    return lastInput.key === "Tab";
  }

  return (
    focused instanceof Node &&
    lastInput.target instanceof Node &&
    lastInput.target.contains(focused)
  );
}
