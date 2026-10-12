/**
 * When a shown list loads, for every list a page reads: the paged ones (activity, saved,
 * scheduled, approvals, the ledger) and the snapshots (work, the agent directory).
 *
 * A list loads when it's shown and never loaded (or `everyShow`: each time it's shown), and again
 * for each invalidation while it's shown: once per `staleMark` (`pagedReloadMark`), so a reload
 * that lands still stale (made stale again while it was on its way) brings the next one. A failed
 * reload leaves the list at the mark it was asked for, so it doesn't reload again by itself: a new
 * invalidation, a Retry, or showing the list again does.
 */
import { useEffect, useRef } from "react";
import type { LoadStatus } from "./model.ts";
import { type PagedList, pagedReloadMark } from "./paged-list.ts";

/** What a list's hook reloads by. */
export interface ReloadSignal {
  /** Never loaded (or reset): load it. */
  readonly idle: boolean;
  /** The invalidation to reload for now, `null` for none (`pagedReloadMark`). */
  readonly staleMark: number | null;
}

/** A paged list's signal. */
export function pagedReloadSignal(list: PagedList): ReloadSignal {
  return { idle: list.status === "idle", staleMark: pagedReloadMark(list) };
}

/** A snapshot's signal: it has no invalidations, it loads when shown. */
export function snapshotReloadSignal(status: LoadStatus): ReloadSignal {
  return { idle: status === "idle", staleMark: null };
}

/**
 * Loads a shown list as the module doc says. `load` stands for the list: a new one (another
 * list, or one now enabled) counts as the list shown again.
 */
export function useListReload(signal: ReloadSignal, load: () => void, everyShow = false): void {
  const answered = useRef<{ readonly load: () => void; readonly mark: number | null } | null>(null);

  const { idle, staleMark } = signal;

  // Leaving (or StrictMode's rehearsal of it) may retire the reads on their way: shown again, the
  // list loads as if new.
  useEffect(
    () => () => {
      answered.current = null;
    },
    [],
  );

  useEffect(() => {
    const last = answered.current;
    const shown = last?.load !== load;

    const due =
      idle || (shown && everyShow) || (staleMark !== null && (shown || last?.mark !== staleMark));

    if (shown || due) {
      answered.current = { load, mark: staleMark };
    }

    if (due) {
      load();
    }
  }, [idle, staleMark, load, everyShow]);
}
