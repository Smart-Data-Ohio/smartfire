import "./kbd.css";

interface KbdProps {
  /** One entry per key, as it should read: ["⌘", "K"], ["Ctrl", "Shift", "M"]. */
  readonly keys: readonly string[];
  readonly className?: string;
}

const ARIA_KEY_NAMES = new Map([
  ["⌘", "Meta"],
  ["⌥", "Alt"],
  ["⇧", "Shift"],
  ["⌃", "Control"],
  ["Ctrl", "Control"],
  ["Esc", "Escape"],
  ["⏎", "Enter"],
  ["↑", "ArrowUp"],
  ["↓", "ArrowDown"],
]);

/** The `aria-keyshortcuts` value for a key combination written the way Kbd shows it. */
export function ariaKeyShortcuts(keys: readonly string[]): string {
  return keys.map((key) => ARIA_KEY_NAMES.get(key) ?? key).join("+");
}

/** A keyboard shortcut, as nested <kbd> elements (the HTML way to write a key combination). */
export function Kbd({ keys, className }: KbdProps) {
  return (
    <kbd className={className === undefined ? "kbd" : `kbd ${className}`}>
      {keys.map((key) => (
        <kbd key={key} className="kbd-key">
          {key}
        </kbd>
      ))}
    </kbd>
  );
}
