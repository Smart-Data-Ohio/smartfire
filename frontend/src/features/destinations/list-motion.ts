/**
 * Motion for a live list (the activity inbox, saved items, scheduled messages): rows that arrive
 * live at the top slide in, rows that leave (handled, removed, cancelled) play a short exit in
 * place before they go. Nothing animates on the first load, on paging (rows appended after the
 * last known one), when the list's scope changes (another tab or filter), or when more than a
 * screenful arrives at once (a reconnect's batch).
 */
import { useEffect, useState } from "react";
import { readDurationMs } from "../../motion/durations.ts";

export type RowMotion = "enter" | "leave" | undefined;

export interface MotionRow<T> {
  readonly key: number;
  readonly value: T;
  readonly motion: RowMotion;
}

/** More arrivals than this at once appear without motion (plan §3.7). */
const MAX_ANIMATED = 20;

/** How long a row keeps its enter flag: long enough to play, short enough not to replay on scroll. */
const ENTER_HOLD_MS = 600;

interface Leaving<T> {
  readonly key: number;
  readonly value: T;
  /** The row it followed, to keep its place while it exits; `null` for the top. */
  readonly after: number | null;
}

interface MotionState<T> {
  readonly scope: string;
  readonly items: readonly T[] | null;
  readonly entering: ReadonlySet<number>;
  readonly leaving: readonly Leaving<T>[];
}

/** The keys new since `previous` that sit above every row `previous` had: live arrivals. */
export function arrivals(previous: readonly number[], next: readonly number[]): readonly number[] {
  const known = new Set(previous);
  const firstKnown = next.findIndex((key) => known.has(key));
  const top = firstKnown === -1 ? next : next.slice(0, firstKnown);

  return top.length > MAX_ANIMATED ? [] : top;
}

/** The rows in `previous` that `next` dropped, each with the row it followed. */
export function departures<T>(
  previous: readonly T[],
  next: readonly T[],
  keyOf: (item: T) => number,
): readonly Leaving<T>[] {
  const kept = new Set(next.map(keyOf));
  const gone: Leaving<T>[] = [];

  previous.forEach((item, index) => {
    const key = keyOf(item);

    if (!kept.has(key)) {
      const before = previous[index - 1];

      gone.push({ key, value: item, after: before === undefined ? null : keyOf(before) });
    }
  });

  return gone.length > MAX_ANIMATED ? [] : gone;
}

/** `rows` with each leaving row put back after the row it followed (or at the top). */
export function withLeaving<T>(
  rows: readonly MotionRow<T>[],
  leaving: readonly Leaving<T>[],
): readonly MotionRow<T>[] {
  if (leaving.length === 0) {
    return rows;
  }

  const out = [...rows];

  for (const gone of leaving) {
    if (out.some((row) => row.key === gone.key)) {
      continue;
    }

    const at = gone.after === null ? 0 : out.findIndex((row) => row.key === gone.after) + 1;

    out.splice(at, 0, { key: gone.key, value: gone.value, motion: "leave" });
  }

  return out;
}

/**
 * The list to render, with each row's motion. `items` is `null` while the first page loads;
 * `scope` names what the list shows (a tab and filter), so switching it never animates.
 */
export function useListMotion<T>(
  items: readonly T[] | null,
  keyOf: (item: T) => number,
  scope: string,
): readonly MotionRow<T>[] {
  const [state, setState] = useState<MotionState<T>>(() => ({
    scope,
    items,
    entering: new Set(),
    leaving: [],
  }));

  if (state.scope !== scope || state.items !== items) {
    const previous = state.items;
    const sameScope = state.scope === scope && previous !== null && items !== null;

    const entering = sameScope
      ? new Set([...state.entering, ...arrivals(previous.map(keyOf), items.map(keyOf))])
      : new Set<number>();

    const leaving = sameScope
      ? [
          ...state.leaving.filter((row) => !items.some((item) => keyOf(item) === row.key)),
          ...departures(previous, items, keyOf),
        ]
      : [];

    setState({ scope, items, entering, leaving });
  }

  const { entering, leaving } = state;

  useEffect(() => {
    if (entering.size === 0) {
      return;
    }

    const timer = window.setTimeout(
      () => setState((current) => ({ ...current, entering: new Set() })),
      ENTER_HOLD_MS,
    );

    return () => window.clearTimeout(timer);
  }, [entering]);

  useEffect(() => {
    if (leaving.length === 0) {
      return;
    }

    const timer = window.setTimeout(
      () => setState((current) => ({ ...current, leaving: [] })),
      readDurationMs("--duration-medium-exit") + 40,
    );

    return () => window.clearTimeout(timer);
  }, [leaving]);

  const rows = (items ?? []).map(
    (item): MotionRow<T> => ({
      key: keyOf(item),
      value: item,
      motion: entering.has(keyOf(item)) ? "enter" : undefined,
    }),
  );

  return withLeaving(rows, leaving);
}
