/**
 * Global search as Effect programs: run a query (its first page, sections included), page
 * through older matches, and keep the viewer's recent searches. Loads never fail: their outcome
 * lands in the search store. Writes to recents fail with the API error.
 */
import { Effect, Result } from "effect";
import * as api from "../api/search-endpoints.ts";
import type { RecentSearch } from "../gen/RecentSearch.ts";
import { searchKey, searchMutations, searchStore } from "../store/search.ts";
import { mutations } from "../store/store.ts";

/**
 * Loads `query`'s first page into the store (a held list keeps its rows until it lands). The
 * people the hits name join the live store. A blank query answers an empty page.
 */
export const run = Effect.fn("search.run")(function* (query: string) {
  const key = searchKey(query);

  searchMutations.startSearch(key);

  const generation = searchStore.getState().results[key]?.generation ?? 0;
  const reply = yield* Effect.result(api.search(key, null));

  if (Result.isFailure(reply)) {
    searchMutations.failFirstPage(key, generation, reply.failure.message);

    return;
  }

  mutations.mergeUsers(reply.success.users);
  searchMutations.applyFirstPage(key, generation, reply.success);
});

/** The next older page of `query`'s matches; a no-op while one is loading or when none is left. */
export const loadMore = Effect.fn("search.loadMore")(function* (query: string) {
  const key = searchKey(query);
  const list = searchStore.getState().results[key];
  const cursor = list?.nextCursor ?? null;

  if (list === undefined || cursor === null || list.loadingMore || list.status !== "ready") {
    return;
  }

  searchMutations.startMore(key);

  const reply = yield* Effect.result(api.search(key, cursor));

  if (Result.isFailure(reply)) {
    searchMutations.failMore(key, reply.failure.message);

    return;
  }

  mutations.mergeUsers(reply.success.users);
  searchMutations.applyMorePage(key, cursor, reply.success);
});

/** Loads the viewer's recent searches; a failure keeps whatever was shown. */
export const loadRecents = Effect.fn("search.loadRecents")(function* () {
  searchMutations.setRecentsLoading();

  const reply = yield* Effect.result(api.recentSearches());

  if (Result.isFailure(reply)) {
    searchMutations.setRecentsFailed();

    return;
  }

  searchMutations.setRecents(reply.success.searches);
});

/**
 * Remembers a submitted query: it moves to the top of the recents at once, then the server's
 * list replaces the guess. A blank query records nothing.
 */
export const record = Effect.fn("search.record")(function* (query: string) {
  const key = searchKey(query);

  if (key === "") {
    return;
  }

  const held = searchStore.getState().recents.searches;
  const existing = held.find((search) => search.query === key);

  const guess: RecentSearch = {
    id: existing?.id ?? -1,
    query: key,
    searchedAt: new Date().toISOString(),
  };

  searchMutations.setRecents(
    [guess, ...held.filter((search) => search.query !== key)].slice(0, 10),
  );

  const reply = yield* api
    .recordSearch(key)
    .pipe(Effect.tapError(() => Effect.sync(() => searchMutations.setRecents(held))));

  searchMutations.setRecents(reply.searches);
});

/** Forgets every recent search: the list empties at once and comes back if the server refuses. */
export const clearRecents = Effect.fn("search.clearRecents")(function* () {
  const held = searchStore.getState().recents.searches;

  searchMutations.setRecents([]);

  yield* api
    .clearRecentSearches()
    .pipe(Effect.tapError(() => Effect.sync(() => searchMutations.setRecents(held))));
});
