/**
 * React's view of global search: the results of one query (loaded the first time it's shown,
 * reused when revisited) and the viewer's recent searches (loaded once, on first use).
 */
import { useEffect, useEffectEvent } from "react";
import { actions } from "../sync/runtime.ts";
import {
  emptyResultList,
  type RecentSearchesState,
  type SearchResultList,
  searchKey,
  useSearchStore,
} from "./search.ts";

export interface SearchResultsView extends SearchResultList {
  /** The query the view is for (whitespace collapsed); "" for none. */
  readonly query: string;
  readonly hasMore: boolean;
  /** Loads the next older page; a no-op while one loads or when none is left. */
  readonly loadMore: () => void;
  /** Loads the first page again (Retry, or the same query submitted again). */
  readonly reload: () => void;
}

/**
 * The results for `query`. The first page loads when a query is first shown (or shown again after
 * failing); a list already loaded this session comes back as it was, scroll depth included. A
 * blank query loads nothing.
 */
export function useSearchResults(query: string): SearchResultsView {
  const key = searchKey(query);
  const list = useSearchStore((state) => state.results[key]);
  const needsLoad = key !== "" && (list === undefined || list.status === "error");

  const loadIfNeeded = useEffectEvent((shownKey: string) => {
    if (needsLoad) {
      void actions.search.run(shownKey);
    }
  });

  // Each new query (and a revisit of one that failed) loads; Retry calls `reload`.
  useEffect(() => loadIfNeeded(key), [key]);

  const shown = list ?? (key === "" ? emptyResultList : { ...emptyResultList, status: "loading" });

  return {
    ...shown,
    query: key,
    hasMore: shown.nextCursor !== null,
    loadMore: () => void actions.search.loadMore(key),
    reload: () => void actions.search.run(key),
  };
}

/** The viewer's recent searches, loaded the first time anything asks for them. */
export function useRecentSearches(): RecentSearchesState {
  const recents = useSearchStore((state) => state.recents);

  useEffect(() => {
    if (recents.status === "idle") {
      void actions.search.loadRecents();
    }
  }, [recents.status]);

  return recents;
}
