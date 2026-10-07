/**
 * A keyset-paged list of ids, as the S3 inbox, saved and scheduled lists keep them: the pages
 * loaded so far, in the server's order, and whether more lie beyond. Live changes place an id
 * without a refetch: an id that now belongs joins at its sorted place when that place is inside
 * the loaded window (past the last loaded row it belongs to a later page, which brings it then);
 * an id that no longer belongs leaves.
 *
 * Every first-page load (or reload) starts a new `generation`. A load remembers the generation it
 * started in, and its page (or failure) lands only if no reload started since: a late next page
 * can't append to, or set the cursor of, a list that was reloaded meanwhile.
 */
import type { LoadStatus } from "./model.ts";

/** A server's opaque `nextCursor`, passed back as `before`; never parsed here. */
export type Cursor = string;

export interface PagedList {
  /** In the list's order. */
  readonly ids: readonly number[];
  /** The `before` cursor for the next page; `null` when the last page is loaded. */
  readonly nextCursor: Cursor | null;
  /** The first page: `idle` until asked for, `error` when it failed. */
  readonly status: LoadStatus;
  /** A next page is on its way. */
  readonly loadingMore: boolean;
  /** Why the last load (first page or more) failed; cleared by the next success. */
  readonly error: string | null;
  /** Something happened the list can't place without the server: reload it when next shown. */
  readonly stale: boolean;
  /** Bumped by every first-page load; a load lands only in the generation it started in. */
  readonly generation: number;
}

export const emptyPagedList: PagedList = {
  ids: [],
  nextCursor: null,
  status: "idle",
  loadingMore: false,
  error: null,
  stale: false,
  generation: 0,
};

/** Sorts two ids in the list's order: negative when `left` comes first. */
export type IdOrder = (left: number, right: number) => number;

/**
 * The first page (or a reload) is on its way, in a new generation: a next page still on its way
 * won't land. What's shown stays until the reload lands.
 */
export function pagedLoading(list: PagedList): PagedList {
  return {
    ...list,
    status: list.status === "ready" ? "ready" : "loading",
    error: null,
    generation: list.generation + 1,
  };
}

/** Whether a load that started in `generation` may still land (`undefined`: no check). */
export function pagedCurrent(list: PagedList, generation: number | undefined): boolean {
  return generation === undefined || generation === list.generation;
}

/** The next page is on its way. */
export function pagedLoadingMore(list: PagedList): PagedList {
  return { ...list, loadingMore: true, error: null };
}

/**
 * A load failed: a list never shown says so; a shown one keeps its rows and the message. A load
 * from an older `generation` changes nothing.
 */
export function pagedFailed(list: PagedList, error: string, generation?: number): PagedList {
  if (!pagedCurrent(list, generation)) {
    return list;
  }

  return {
    ...list,
    status: list.status === "ready" ? "ready" : "error",
    loadingMore: false,
    error,
  };
}

/**
 * A page landed: `replace` makes it the list (a first load or a reload); `more` adds it at the
 * end, skipping ids already held (one may have moved in live meanwhile). A page from an older
 * `generation` changes nothing.
 */
export function pagedLanded(
  list: PagedList,
  ids: readonly number[],
  nextCursor: Cursor | null,
  mode: "replace" | "more",
  generation?: number,
): PagedList {
  if (!pagedCurrent(list, generation)) {
    return list;
  }

  const merged = mode === "replace" ? [...new Set(ids)] : [...new Set([...list.ids, ...ids])];

  return {
    ids: merged,
    nextCursor,
    status: "ready",
    loadingMore: false,
    error: null,
    stale: mode === "replace" ? false : list.stale,
    generation: list.generation,
  };
}

/** Drops `id`; the same list when it wasn't there. */
export function pagedWithout(list: PagedList, id: number): PagedList {
  return list.ids.includes(id) ? { ...list, ids: list.ids.filter((held) => held !== id) } : list;
}

/**
 * Places `id` after a live change: out when it no longer `belongs`, else at its sorted place,
 * unless that's past the last loaded row of a list with more pages (a later page brings it). A
 * list that hasn't loaded only loses the id.
 */
export function pagedPlaced(
  list: PagedList,
  id: number,
  belongs: boolean,
  order: IdOrder,
): PagedList {
  const without = pagedWithout(list, id);

  if (!belongs || list.status !== "ready") {
    return without;
  }

  const others = without.ids;
  const last = others.at(-1);

  if (list.nextCursor !== null && last !== undefined && order(id, last) > 0) {
    return without;
  }

  const index = others.findIndex((held) => order(id, held) < 0);
  const ids = index < 0 ? [...others, id] : others.toSpliced(index, 0, id);

  return ids.every((held, at) => list.ids[at] === held) && ids.length === list.ids.length
    ? list
    : { ...list, ids };
}

/** Marks a loaded list for a reload when next shown. */
export function pagedStale(list: PagedList): PagedList {
  return list.status === "idle" || list.stale ? list : { ...list, stale: true };
}

/** Each list in `lists` changed by `change`, keeping the same record when none changed. */
export function eachPaged(
  lists: Readonly<Record<string, PagedList>>,
  change: (list: PagedList, key: string) => PagedList,
): Readonly<Record<string, PagedList>> {
  let changed = false;

  const next = Object.fromEntries(
    Object.entries(lists).map(([key, list]) => {
      const updated = change(list, key);

      changed ||= updated !== list;

      return [key, updated] as const;
    }),
  );

  return changed ? next : lists;
}
