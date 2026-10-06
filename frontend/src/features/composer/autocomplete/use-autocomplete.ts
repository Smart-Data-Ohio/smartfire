import { useEffect, useId, useState } from "react";
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

function firstSelectable(items: readonly Suggestion[]): number {
  const index = items.findIndex(selectable);

  return index < 0 ? 0 : index;
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
  const sidebar = useStore((state) => state.sidebar);
  const [results, setResults] = useState<Results | null>(null);
  const [activeIndex, setActive] = useState(0);
  const [dismissedAt, setDismissedAt] = useState<number | null>(null);
  const kind = trigger?.kind ?? null;
  const query = trigger?.query ?? "";

  useEffect(() => {
    if (kind === null || (kind === "command" && !commandsEnabled)) {
      setResults(null);

      return;
    }

    let live = true;

    const land = (items: readonly Suggestion[]) => {
      if (live) {
        setResults({ kind, query, items });
        setActive(firstSelectable(items));
      }
    };

    if (kind === "room") {
      land(roomSuggestions(sidebar, query));

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
  const index = Math.min(activeIndex, Math.max(0, items.length - 1));

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
        setActive(next);
      }
    },
    move: (delta) => setActive(step(items, index, delta)),
    dismiss: () => setDismissedAt(trigger?.start ?? null),
  };
}
