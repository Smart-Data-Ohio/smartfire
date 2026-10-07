/** The shortcuts dialog's content: the SHORTCUTS catalogue, grouped and filtered. */
import type { Shortcut, ShortcutGroup } from "../../lib/shortcuts.ts";
import { normalizeQuery } from "./match.ts";

export const GROUP_ORDER = [
  "Navigation",
  "Messages",
  "Lists",
  "Composer",
  "Formatting",
] as const satisfies readonly ShortcutGroup[];

export interface ShortcutSection {
  readonly group: ShortcutGroup;
  readonly shortcuts: readonly Shortcut[];
}

/** Readable names for the key glyphs, so "shift" or "enter" finds them too. */
const KEY_WORDS = new Map([
  ["⌘", "cmd command"],
  ["⌥", "option alt"],
  ["⇧", "shift"],
  ["⏎", "enter return"],
  ["⌫", "delete backspace"],
  ["↑", "up arrow"],
  ["↓", "down arrow"],
  ["Esc", "escape"],
]);

function searchable(shortcut: Shortcut): string {
  const keys = shortcut.keys.map((key) => KEY_WORDS.get(key) ?? key).join(" ");

  return `${shortcut.label} ${keys} ${shortcut.group}`;
}

/**
 * The catalogue in dialog order (Navigation, Messages, Lists, Composer, Formatting), each group in
 * catalogue order. A query keeps the shortcuts whose label, keys or group match it, by word:
 * every word of the query must match somewhere.
 */
export function groupShortcuts(
  shortcuts: readonly Shortcut[],
  query: string,
): readonly ShortcutSection[] {
  const words = normalizeQuery(query).split(/\s+/).filter(Boolean);

  const kept = shortcuts.filter((shortcut) => {
    const text = normalizeQuery(searchable(shortcut));

    return words.every((word) => text.includes(word));
  });

  return GROUP_ORDER.flatMap((group) => {
    const inGroup = kept.filter((shortcut) => shortcut.group === group);

    return inGroup.length === 0 ? [] : [{ group, shortcuts: inGroup }];
  });
}
