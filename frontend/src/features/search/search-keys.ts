/**
 * Which key press opens search, if any. Pure, so the mapping is tested apart from the listener;
 * the keys mirror SHORTCUTS ("search", "search-slash"). ⌘F and Ctrl+F are left to the browser's
 * own find: search takes ⌘⇧F (Ctrl+Shift+F), and `/` only when no text field has focus.
 */
import type { KeyPress } from "../switcher/global-keys.ts";

export type SearchShortcut = "search" | "search-slash";

/** `event` as a search shortcut; `editing` says whether a text field (or editor) has focus. */
export function searchShortcut(
  event: KeyPress,
  apple: boolean,
  editing: boolean,
): SearchShortcut | null {
  const mod = apple ? event.metaKey && !event.ctrlKey : event.ctrlKey && !event.metaKey;

  if (
    mod &&
    event.shiftKey &&
    !event.altKey &&
    (event.key.toLowerCase() === "f" || event.code === "KeyF")
  ) {
    return "search";
  }

  const bare = !event.metaKey && !event.ctrlKey && !event.altKey;

  return bare && !editing && event.key === "/" ? "search-slash" : null;
}

/** Whether `target` takes typing: a text input, a textarea, a select or an editable region. */
export function isEditable(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) {
    return false;
  }

  if (target.isContentEditable || target instanceof HTMLTextAreaElement) {
    return true;
  }

  if (target instanceof HTMLSelectElement) {
    return true;
  }

  return (
    target instanceof HTMLInputElement &&
    !["button", "checkbox", "radio", "range", "reset", "submit", "color", "file"].includes(
      target.type,
    )
  );
}
