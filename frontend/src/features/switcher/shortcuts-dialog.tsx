import { useId, useState } from "react";
import { SHORTCUTS } from "../../lib/shortcuts.ts";
import { Dialog } from "../../ui/dialog.tsx";
import { Icon } from "../../ui/icons/icon.tsx";
import { Kbd } from "../../ui/kbd.tsx";
import { groupShortcuts } from "./shortcut-groups.ts";
// The search field is the design system's `.input`; its styles ship with the text field.
import "../../ui/text-field.css";
import "./shortcuts.css";

interface ShortcutsDialogProps {
  readonly open: boolean;
  readonly onOpenChange: (open: boolean) => void;
}

/**
 * Every keyboard shortcut (⌘/), grouped Navigation, Messages, Composer, Formatting, with the
 * keys as this platform prints them (⌘ and ⌥ on a Mac, Ctrl and Alt elsewhere). Type to filter
 * by name or key ("shift", "enter").
 */
export default function ShortcutsDialog({ open, onOpenChange }: ShortcutsDialogProps) {
  const id = useId();
  const [query, setQuery] = useState("");
  const [wasOpen, setWasOpen] = useState(open);
  const sections = groupShortcuts(SHORTCUTS, query);

  if (open !== wasOpen) {
    setWasOpen(open);

    if (open) {
      setQuery("");
    }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange} title="Keyboard shortcuts">
      <div className="shortcuts">
        <div className="shortcuts-search">
          <Icon name="search" size={16} className="shortcuts-search-icon" />
          <input
            className="input shortcuts-input"
            type="search"
            aria-label="Search shortcuts"
            placeholder="Search shortcuts"
            autoComplete="off"
            spellCheck={false}
            value={query}
            data-autofocus
            onChange={(event) => setQuery(event.target.value)}
          />
        </div>
        <div className="shortcuts-list">
          {sections.map((section) => (
            <section
              key={section.group}
              className="shortcuts-group"
              aria-labelledby={`${id}-${section.group}`}
            >
              <h3 id={`${id}-${section.group}`} className="shortcuts-group-title">
                {section.group}
              </h3>
              <dl className="shortcuts-rows">
                {section.shortcuts.map((shortcut) => (
                  <div key={shortcut.id} className="shortcuts-row">
                    <dt>{shortcut.label}</dt>
                    <dd>
                      <Kbd keys={shortcut.keys} />
                    </dd>
                  </div>
                ))}
              </dl>
            </section>
          ))}
          {sections.length === 0 ? (
            <p className="shortcuts-empty">No shortcuts match “{query.trim()}”.</p>
          ) : null}
        </div>
      </div>
    </Dialog>
  );
}
