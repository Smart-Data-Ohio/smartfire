/**
 * Global search as Effect programs: run a query (its first page, sections included), page
 * through older matches, and keep the viewer's recent searches. Loads never fail: their outcome
 * lands in the search store. Writes to recents fail with the API error.
 */
import { Effect, Result, Semaphore } from "effect";
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

/** Changes to the recents go one at a time, so each rolls back to a list no other is holding. */
const recentsLock = Semaphore.makeUnsafe(1);

/**
 * Shows `guess` at once, then runs `write`. A failure (or an interruption, whose outcome is
 * unknown) puts back the list it started from, but only while `guess` is still what's shown:
 * a newer list from the server stays.
 */
const changeRecents = <A, E, R>(
  guessFrom: (held: readonly RecentSearch[]) => readonly RecentSearch[],
  write: Effect.Effect<A, E, R>,
) =>
  Semaphore.withPermit(
    recentsLock,
    Effect.suspend(() => {
      const held = searchStore.getState().recents.searches;
      const guess = guessFrom(held);

      searchMutations.setRecents(guess);

      return write.pipe(
        Effect.onError(() =>
          Effect.sync(() => {
            if (searchStore.getState().recents.searches === guess) {
              searchMutations.setRecents(held);
            }
          }),
        ),
      );
    }),
  );

/**
 * Remembers a submitted query: it moves to the top of the recents at once, then the server's
 * list replaces the guess. A blank query records nothing.
 */
export const record = Effect.fn("search.record")(function* (query: string) {
  const key = searchKey(query);

  if (key === "") {
    return;
  }

  const searchedAt = new Date().toISOString();

  const reply = yield* changeRecents((held) => {
    const guess: RecentSearch = {
      id: held.find((search) => search.query === key)?.id ?? -1,
      query: key,
      searchedAt,
    };

    return [guess, ...held.filter((search) => search.query !== key)].slice(0, 10);
  }, api.recordSearch(key));

  searchMutations.setRecents(reply.searches);
});

/** Forgets every recent search: the list empties at once and comes back if the server refuses. */
export const clearRecents = Effect.fn("search.clearRecents")(function* () {
  yield* changeRecents(() => [], api.clearRecentSearches());
});
