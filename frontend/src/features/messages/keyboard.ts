/**
 * Keyboard message navigation: ↑/↓ move a roving focus across the rows of a message list, and a
 * focused row answers single keys (the `message-*` ids in the shortcut catalogue). The decisions
 * are pure so they're tested without a DOM; `focusAdjacentRow` and friends do the moving.
 */

/** What a key on a focused row asks for. */
export type RowCommand =
  | "up"
  | "down"
  | "first"
  | "last"
  | "edit"
  | "react"
  | "reply"
  | "thread"
  | "pin"
  | "save"
  | "forward"
  | "link"
  | "delete"
  | "menu"
  | "composer";

/** The parts of a keyboard event the decision reads. */
export interface KeyPress {
  readonly key: string;
  readonly shiftKey: boolean;
  readonly altKey: boolean;
  readonly ctrlKey: boolean;
  readonly metaKey: boolean;
}

const LETTERS = new Map<string, RowCommand>([
  ["e", "edit"],
  ["r", "react"],
  ["+", "react"],
  ["q", "reply"],
  ["t", "thread"],
  ["p", "pin"],
  ["s", "save"],
  ["f", "forward"],
  ["l", "link"],
  ["k", "up"],
  ["j", "down"],
]);

/**
 * The command for a key pressed while a row itself has focus, or `null` to let it through.
 * Chords with ⌘/Ctrl/Alt belong to the app (Alt+↑ switches conversations), so they pass.
 */
export function rowCommand(press: KeyPress): RowCommand | null {
  if (press.key === "F10" && press.shiftKey) {
    return "menu";
  }

  if (press.key === "ContextMenu") {
    return "menu";
  }

  if (press.ctrlKey || press.metaKey || press.altKey) {
    return null;
  }

  switch (press.key) {
    case "ArrowUp":
      return "up";
    case "ArrowDown":
      return "down";
    case "Home":
      return "first";
    case "End":
      return "last";
    case "Delete":
    case "Backspace":
      return "delete";
    case "Escape":
      return "composer";
    default:
      return press.shiftKey && press.key !== "+"
        ? null
        : (LETTERS.get(press.key.toLowerCase()) ?? null);
  }
}

/** Where the roving focus goes from `index` among `count` rows; `null` when it can't move. */
export function nextRowIndex(index: number, count: number, command: RowCommand): number | null {
  if (count === 0) {
    return null;
  }

  switch (command) {
    case "up":
      return index > 0 ? index - 1 : null;
    case "down":
      return index < count - 1 ? index + 1 : null;
    case "first":
      return 0;
    case "last":
      return count - 1;
    default:
      return null;
  }
}

const ROW = "[data-message-row]";

const LIST = '[data-message-list], [role="log"]';

/** The focusable message rows of the list `element` sits in, in document order. */
export function rowsAround(element: Element): HTMLElement[] {
  const list = element.closest(LIST);

  return list === null ? [] : [...list.querySelectorAll<HTMLElement>(ROW)];
}

/** Moves focus from `row` to a neighbour; answers whether it moved. */
export function focusAdjacentRow(row: HTMLElement, command: RowCommand): boolean {
  const rows = rowsAround(row);
  const next = nextRowIndex(rows.indexOf(row), rows.length, command);
  const target = next === null ? undefined : rows[next];

  if (target === undefined) {
    return false;
  }

  target.focus();
  target.scrollIntoView({ block: "nearest" });

  return true;
}

/**
 * Focuses the composer that belongs with `element`'s list: the nearest enclosing pane that has
 * one (the thread pane's for a thread, the room's for the timeline).
 */
export function focusComposerNear(element: Element): boolean {
  for (let node = element.parentElement; node !== null; node = node.parentElement) {
    const input = node.querySelector<HTMLTextAreaElement>(".composer-input");

    if (input !== null) {
      input.focus();

      return true;
    }
  }

  return false;
}

/**
 * Focuses the newest row of the message list beside `element` (a composer): the composer's ↑
 * with an empty box ("Select messages"). Answers whether a row took focus.
 */
export function focusLastRowNear(element: Element): boolean {
  for (let node = element.parentElement; node !== null; node = node.parentElement) {
    const rows = node.querySelectorAll<HTMLElement>(ROW);
    const last = rows[rows.length - 1];

    if (last !== undefined) {
      last.focus();
      last.scrollIntoView({ block: "nearest" });

      return true;
    }
  }

  return false;
}
