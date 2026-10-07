/**
 * Every keyboard shortcut the app answers, in one catalogue: the shortcuts dialog lists it and
 * features read their keys from it, so the two never disagree. `mod` is ⌘ on Apple platforms and
 * Ctrl elsewhere; keys read as they are printed on the keycap.
 */
export const IS_APPLE =
  typeof navigator !== "undefined" && /Mac|iPhone|iPad/.test(navigator.platform);

export const MOD = IS_APPLE ? "⌘" : "Ctrl";

export const ALT = IS_APPLE ? "⌥" : "Alt";

export type ShortcutGroup = "Navigation" | "Messages" | "Lists" | "Composer" | "Formatting";

export interface Shortcut {
  readonly id: string;
  readonly group: ShortcutGroup;
  readonly keys: readonly string[];
  readonly label: string;
}

export const SHORTCUTS = [
  { id: "switcher", group: "Navigation", keys: [MOD, "K"], label: "Jump to a conversation" },
  { id: "shortcuts", group: "Navigation", keys: [MOD, "/"], label: "Show keyboard shortcuts" },
  { id: "new-direct", group: "Navigation", keys: [MOD, "⇧", "K"], label: "New direct message" },
  {
    id: "sidebar-menu",
    group: "Navigation",
    keys: ["⇧", "F10"],
    label: "Open a sidebar conversation's menu",
  },
  {
    id: "sidebar-move",
    group: "Navigation",
    keys: ["Space"],
    label: "Pick up a sidebar conversation or category to move it",
  },
  { id: "previous-room", group: "Navigation", keys: [ALT, "↑"], label: "Previous conversation" },
  { id: "next-room", group: "Navigation", keys: [ALT, "↓"], label: "Next conversation" },
  {
    id: "previous-unread",
    group: "Navigation",
    keys: [ALT, "⇧", "↑"],
    label: "Previous unread conversation",
  },
  {
    id: "next-unread",
    group: "Navigation",
    keys: [ALT, "⇧", "↓"],
    label: "Next unread conversation",
  },
  { id: "close-pane", group: "Navigation", keys: ["Esc"], label: "Close the side pane" },
  { id: "search", group: "Navigation", keys: [MOD, "⇧", "F"], label: "Search messages" },
  {
    id: "search-slash",
    group: "Navigation",
    keys: ["/"],
    label: "Search messages (outside a text field)",
  },
  {
    id: "focus-messages",
    group: "Messages",
    keys: ["↑"],
    label: "Select messages (empty composer)",
  },
  { id: "message-up", group: "Messages", keys: ["↑"], label: "Previous message" },
  { id: "message-down", group: "Messages", keys: ["↓"], label: "Next message" },
  { id: "message-edit", group: "Messages", keys: ["E"], label: "Edit your message" },
  { id: "message-react", group: "Messages", keys: ["R"], label: "Add a reaction" },
  { id: "message-thread", group: "Messages", keys: ["T"], label: "Reply in thread" },
  { id: "message-pin", group: "Messages", keys: ["P"], label: "Pin or unpin" },
  { id: "message-save", group: "Messages", keys: ["S"], label: "Save for later" },
  { id: "message-forward", group: "Messages", keys: ["F"], label: "Forward" },
  { id: "message-link", group: "Messages", keys: ["L"], label: "Copy link" },
  { id: "message-delete", group: "Messages", keys: ["⌫"], label: "Delete your message" },
  { id: "message-menu", group: "Messages", keys: ["⇧", "F10"], label: "Open the message menu" },
  {
    id: "list-up",
    group: "Lists",
    keys: ["↑"],
    label: "Previous item (Activity, Saved, Scheduled)",
  },
  { id: "list-down", group: "Lists", keys: ["↓"], label: "Next item" },
  { id: "list-open", group: "Lists", keys: ["⏎"], label: "Open the item" },
  { id: "list-read", group: "Lists", keys: ["U"], label: "Mark read or unread (Activity)" },
  { id: "list-done", group: "Lists", keys: ["E"], label: "Mark handled or done" },
  { id: "list-remove", group: "Lists", keys: ["⌫"], label: "Remove from saved, or cancel" },
  { id: "list-menu", group: "Lists", keys: ["⇧", "F10"], label: "Open the item's menu" },
  {
    id: "edit-last",
    group: "Composer",
    keys: ["↑"],
    label: "Edit your last message (empty composer)",
  },
  { id: "send", group: "Composer", keys: ["⏎"], label: "Send" },
  { id: "newline", group: "Composer", keys: ["⇧", "⏎"], label: "New line" },
  { id: "upload", group: "Composer", keys: [MOD, "U"], label: "Upload a file" },
  { id: "bold", group: "Formatting", keys: [MOD, "B"], label: "Bold" },
  { id: "italic", group: "Formatting", keys: [MOD, "I"], label: "Italic" },
  { id: "strike", group: "Formatting", keys: [MOD, "⇧", "X"], label: "Strikethrough" },
  { id: "code", group: "Formatting", keys: [MOD, "E"], label: "Code" },
  { id: "link", group: "Formatting", keys: [MOD, "⇧", "U"], label: "Link" },
] as const satisfies readonly Shortcut[];

export type ShortcutId = (typeof SHORTCUTS)[number]["id"];

/** The keys for one shortcut, for `<Kbd keys=…>` and tooltips. */
export function shortcutKeys(id: ShortcutId): readonly string[] {
  return SHORTCUTS.find((shortcut) => shortcut.id === id)?.keys ?? [];
}
