/**
 * Which app-wide shortcut a key press is, if any. Pure, so the mapping is tested apart from the
 * listener; the keys mirror SHORTCUTS (lib/shortcuts.ts), where the dialog reads them.
 */
import type { ShortcutId } from "../../lib/shortcuts.ts";

/** The parts of a KeyboardEvent the mapping reads. */
export interface KeyPress {
  readonly key: string;
  readonly code: string;
  readonly metaKey: boolean;
  readonly ctrlKey: boolean;
  readonly altKey: boolean;
  readonly shiftKey: boolean;
}

export type GlobalShortcut = Extract<
  ShortcutId,
  | "switcher"
  | "shortcuts"
  | "new-direct"
  | "previous-room"
  | "next-room"
  | "previous-unread"
  | "next-unread"
>;

/** ⌘ alone on Apple platforms, Ctrl alone elsewhere. */
function modOnly(event: KeyPress, apple: boolean): boolean {
  return apple ? event.metaKey && !event.ctrlKey : event.ctrlKey && !event.metaKey;
}

function arrowShortcut(event: KeyPress): GlobalShortcut | null {
  if (event.key === "ArrowUp") {
    return event.shiftKey ? "previous-unread" : "previous-room";
  }

  if (event.key === "ArrowDown") {
    return event.shiftKey ? "next-unread" : "next-room";
  }

  return null;
}

/**
 * ⌘K the switcher, ⌘⇧K a new DM, ⌘/ the shortcuts; Alt+↑/↓ the previous or next conversation,
 * with Shift the previous or next unread one. Anything else is `null`. Physical key codes back
 * up the characters, so ⌘K still works on a non-Latin layout.
 */
export function globalShortcut(event: KeyPress, apple: boolean): GlobalShortcut | null {
  if (event.altKey) {
    return event.metaKey || event.ctrlKey ? null : arrowShortcut(event);
  }

  if (!modOnly(event, apple)) {
    return null;
  }

  if (event.key.toLowerCase() === "k" || event.code === "KeyK") {
    return event.shiftKey ? "new-direct" : "switcher";
  }

  if (event.key === "/" || event.code === "Slash") {
    return "shortcuts";
  }

  return null;
}
