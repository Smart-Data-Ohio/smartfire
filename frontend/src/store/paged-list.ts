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
  /**
   * The `generation` the list was last made stale in. A reload answers only what made it stale
   * before the reload started (an older generation); a failed one leaves it stale.
   */
  readonly staleGeneration: number;
  /** Bumped by every first-page load; a load lands only in the generation it started in. */
  readonly generation: number;
  /** The last failure was the first page's (a reload), so a retry asks for it again. */
  readonly firstPageFailed: boolean;
  /** A first page (a first load or a reload) is on its way in the current `generation`. */
  readonly firstPageLoading: boolean;
  /**
   * Counts the invalidations: each one bumps it, even of a list already stale, so a hook that
   * reloaded for one mark can tell a later one from it (see `pagedReloadMark`).
   */
  readonly staleMark: number;
}

export const emptyPagedList: PagedList = {
  ids: [],
  nextCursor: null,
  status: "idle",
  loadingMore: false,
  error: null,
  stale: false,
  staleGeneration: 0,
  generation: 0,
  firstPageFailed: false,
  firstPageLoading: false,
  staleMark: 0,
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
    firstPageFailed: false,
    firstPageLoading: true,
  };
}

/**
 * The loads on their way were abandoned (their view left and retired their reads): none of them
 * lands, so the list says nothing is on its way, and a first load never finished loads again.
 */
export function pagedAbandoned(list: PagedList): PagedList {
  if (!list.firstPageLoading && !list.loadingMore) {
    return list;
  }

  return {
    ...list,
    status: list.status === "ready" ? "ready" : "idle",
    loadingMore: false,
    firstPageLoading: false,
    generation: list.generation + 1,
  };
}

/** Whether a load that started in `generation` may still land. */
export function pagedCurrent(list: PagedList, generation: number): boolean {
  return generation === list.generation;
}

/** The next page is on its way. */
export function pagedLoadingMore(list: PagedList): PagedList {
  return { ...list, loadingMore: true, error: null, firstPageFailed: false };
}

/**
 * A load failed: a list never shown says so; a shown one keeps its rows and the message. A load
 * from an older `generation` changes nothing. `mode` is the request's own: a failed first page
 * (`replace`) is what a retry asks for again, even while a next page is still on its way.
 */
export function pagedFailed(
  list: PagedList,
  error: string,
  generation: number,
  mode: "replace" | "more",
): PagedList {
  if (!pagedCurrent(list, generation)) {
    return list;
  }

  return {
    ...list,
    status: list.status === "ready" ? "ready" : "error",
    loadingMore: false,
    error,
    firstPageFailed: mode === "replace",
    firstPageLoading: mode === "replace" ? false : list.firstPageLoading,
  };
}

/**
 * A page landed: `replace` makes it the list (a first load or a reload); `more` adds it at the
 * end, skipping ids already held (one may have moved in live meanwhile). A page from an older
 * `generation` changes nothing. A reload clears `stale`, unless the list was made stale again
 * while it was on its way (in its own `generation`).
 */
export function pagedLanded(
  list: PagedList,
  ids: readonly number[],
  nextCursor: Cursor | null,
  mode: "replace" | "more",
  generation: number,
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
    stale: mode === "replace" ? list.stale && list.staleGeneration >= generation : list.stale,
    staleGeneration: list.staleGeneration,
    generation: list.generation,
    firstPageFailed: false,
    firstPageLoading: mode === "replace" ? false : list.firstPageLoading,
    staleMark: list.staleMark,
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

/**
 * An invalidation: marks a loaded list for a reload, including one whose reload is on its way (it
 * lands still stale), with a new `staleMark` so a shown list reloads for it.
 */
export function pagedStale(list: PagedList): PagedList {
  if (list.status === "idle") {
    return list;
  }

  return {
    ...list,
    stale: true,
    staleGeneration: list.generation,
    staleMark: list.staleMark + 1,
  };
}

/**
 * Stale, but not for a new invalidation (a page that landed already out of date): reloaded when
 * next shown, or for the next invalidation, not again now, so a server that keeps sending such
 * pages can't make a shown list reload in a loop.
 */
export function pagedOutdated(list: PagedList): PagedList {
  if (list.status === "idle") {
    return list;
  }

  return { ...list, stale: true, staleGeneration: list.generation };
}

/**
 * The invalidation a shown list should reload for now: its `staleMark` while it's loaded and stale
 * with no first page on its way, else `null`. A hook reloads once per mark (`useListReload`), so a
 * reload that lands still stale (made stale again meanwhile) brings the next one, and a failed one
 * waits for a new invalidation, a Retry or the list shown again.
 */
export function pagedReloadMark(list: PagedList): number | null {
  return list.stale && list.status === "ready" && !list.firstPageLoading ? list.staleMark : null;
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
