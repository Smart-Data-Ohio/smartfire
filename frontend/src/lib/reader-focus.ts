/**
 * Whether a focus move inside the timeline belongs to the reader.
 *
 * Tab (and any other key) changes focus after keydown, so the viewport anchor cannot see it on
 * the key itself. A keydown or pointerdown in this window, followed by focusin, is the reader.
 * `focus()` the app calls — dialog restoration, placement, permalink focus — is marked for the
 * synchronous focusin it causes, and does not count.
 */

/** How recently a key or pointer press still explains the focus that follows it. */
const READER_FOCUS_WINDOW_MS = 100;

let lastUserInputAt = Number.NEGATIVE_INFINITY;

let appFocus = 0;

let listeners = 0;

function stampUserInput(): void {
  lastUserInputAt = performance.now();
}

/**
 * Notices key and pointer presses on `doc` until the returned function runs. Several timelines
 * share one pair of listeners. The last release forgets the press, so a later test or screen
 * cannot inherit it.
 */
export function bindReaderInput(doc: Document): () => void {
  if (listeners === 0) {
    doc.addEventListener("keydown", stampUserInput, true);
    doc.addEventListener("pointerdown", stampUserInput, true);
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
      doc.removeEventListener("keydown", stampUserInput, true);
      doc.removeEventListener("pointerdown", stampUserInput, true);
      lastUserInputAt = Number.NEGATIVE_INFINITY;
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

/** A focusin that followed the person's own key or pointer, and not an app `focus()` call. */
export function readerMovedFocus(at = performance.now()): boolean {
  return appFocus === 0 && at - lastUserInputAt <= READER_FOCUS_WINDOW_MS;
}
