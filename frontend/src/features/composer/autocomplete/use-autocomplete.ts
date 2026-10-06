import { useEffect, useId, useRef, useState } from "react";
import { useStore } from "../../../store/store.ts";
import {
  commandSuggestions,
  emojiSuggestions,
  loadCommands,
  mentionSuggestions,
  roomSuggestions,
  type Suggestion,
  selectable,
} from "./suggestions.ts";
import type { Trigger, TriggerKind } from "./trigger.ts";

/** Network lookups wait this long after the last keystroke. */
const DEBOUNCE_MS = 120;

interface Results {
  readonly kind: TriggerKind;
  readonly query: string;
  readonly items: readonly Suggestion[];
}

export interface Autocomplete {
  readonly open: boolean;
  readonly kind: TriggerKind | null;
  readonly items: readonly Suggestion[];
  readonly activeIndex: number;
  readonly listboxId: string;
  readonly optionId: (index: number) => string;
  readonly setActiveIndex: (index: number) => void;
  /** Moves the highlight by `delta`, skipping rows that can't be chosen, wrapping around. */
  readonly move: (delta: number) => void;
  /** Esc: closes until the caret leaves this trigger. */
  readonly dismiss: () => void;
}

/** The next selectable index from `from` in direction `delta`, wrapping; `from` if none. */
function step(items: readonly Suggestion[], from: number, delta: number): number {
  const count = items.length;

  for (let offset = 1; offset <= count; offset += 1) {
    const index = (((from + delta * offset) % count) + count) % count;
    const item = items[index];

    if (item !== undefined && selectable(item)) {
      return index;
    }
  }

  return from;
}

function firstSelectableKey(items: readonly Suggestion[]): string | null {
  return items.find(selectable)?.key ?? null;
}

/**
 * The autocomplete for the composer's current trigger: fetches (debounced) or filters the
 * suggestions, tracks the highlighted row, and closes on Esc until the caret moves to another
 * trigger. While a new query loads, the previous rows stay up, so the menu never flickers.
 */
export function useAutocomplete(
  roomId: number,
  threadId: number | null,
  trigger: Trigger | null,
  commandsEnabled: boolean,
): Autocomplete {
  const listboxId = useId();
  const kind = trigger?.kind ?? null;
  const query = trigger?.query ?? "";
  // Only `#` reads the sidebar; other triggers mustn't refire on every unread change.
  const sidebar = useStore((state) => (kind === "room" ? state.sidebar : null));
  const [results, setResults] = useState<Results | null>(null);
  // The highlight follows a row, not a position, so refreshed rows keep it in place.
  const [activeKey, setActiveKey] = useState<string | null>(null);
  const [dismissedAt, setDismissedAt] = useState<number | null>(null);
  const landedFor = useRef<string | null>(null);

  useEffect(() => {
    if (kind === null || (kind === "command" && !commandsEnabled)) {
      setResults(null);

      return;
    }

    let live = true;

    const land = (items: readonly Suggestion[]) => {
      if (!live) {
        return;
      }

      const same = landedFor.current === `${kind}:${query}`;

      landedFor.current = `${kind}:${query}`;
      setResults({ kind, query, items });
      // The same query refreshed keeps the highlighted row when it's still there.
      setActiveKey((current) =>
        same && items.some((item) => item.key === current && selectable(item))
          ? current
          : firstSelectableKey(items),
      );
    };

    if (kind === "room") {
      if (sidebar !== null) {
        land(roomSuggestions(sidebar, query));
      }

      return;
    }

    if (kind === "command") {
      loadCommands(roomId, threadId).then(
        (commands) => land(commandSuggestions(commands, query)),
        () => land([]),
      );

      return () => {
        live = false;
      };
    }

    const timer = window.setTimeout(() => {
      const lookup =
        kind === "mention" ? mentionSuggestions(roomId, query) : emojiSuggestions(query);

      lookup.then(land, () => land([]));
    }, DEBOUNCE_MS);

    return () => {
      live = false;
      window.clearTimeout(timer);
    };
  }, [kind, query, roomId, threadId, sidebar, commandsEnabled]);

  // A different trigger (or none) clears an Esc.
  const triggerStart = trigger?.start ?? null;

  useEffect(() => {
    setDismissedAt((current) => (current === triggerStart ? current : null));
  }, [triggerStart]);

  const items = results !== null && results.kind === kind ? results.items : [];
  const open = trigger !== null && dismissedAt !== trigger.start && items.length > 0;
  const found = items.findIndex((item) => item.key === activeKey);
  const index = found < 0 ? Math.max(0, items.findIndex(selectable)) : found;

  return {
    open,
    kind: open ? kind : null,
    items: open ? items : [],
    activeIndex: index,
    listboxId,
    optionId: (option) => `${listboxId}-option-${option}`,
    setActiveIndex: (next) => {
      const item = items[next];

      if (item !== undefined && selectable(item)) {
        setActiveKey(item.key);
      }
    },
    move: (delta) => setActiveKey(items[step(items, index, delta)]?.key ?? null),
    dismiss: () => setDismissedAt(trigger?.start ?? null),
  };
}
