/** Global search and the viewer's recent searches (S3). */
import { Effect } from "effect";
import type { RecentSearchList } from "../gen/RecentSearchList.ts";
import type { SearchFilters } from "../gen/SearchFilters.ts";
import type { SearchResults } from "../gen/SearchResults.ts";
import { call, get, noContent } from "./call.ts";
import {
  RecentSearchList as RecentSearchListSchema,
  SearchResults as SearchResultsSchema,
} from "./schema/search.ts";
import { wire } from "./wire.ts";

/**
 * `GET /search?q=&before=`: one page of results, 40 messages newest-first by page (each page
 * oldest first), plus the sections on the first page. `before` is the previous page's opaque
 * `nextCursor`; a cursor the server can't read is a 422. Doesn't record the query.
 */
export const search = Effect.fn("api.search")(function* (
  q: string,
  before: string | null,
  filters?: SearchFilters,
) {
  const query: Record<string, string> = before === null ? { q } : { q, before };

  if (filters !== undefined) {
    if (filters.authorId !== null) query.authorId = String(filters.authorId);

    if (filters.channelId !== null) query.channelId = String(filters.channelId);

    if (filters.has.length > 0) query.has = filters.has.join(",");

    if (filters.mentionsMe) query.mentionsMe = "true";
    query.sort = filters.sort;
  }

  return yield* call(get("/search", query), wire<SearchResults>(SearchResultsSchema));
});

/** `GET /search/recents`: at most 10, most recently searched first. */
export const recentSearches = Effect.fn("api.recentSearches")(function* () {
  return yield* call(get("/search/recents"), wire<RecentSearchList>(RecentSearchListSchema));
});

/**
 * `POST /search/recents`: remembers a submitted query (a repeat moves to the top) and answers the
 * list. A blank query is a 422.
 */
export const recordSearch = Effect.fn("api.recordSearch")(function* (query: string) {
  return yield* call(
    { method: "POST", path: "/search/recents", body: { query } },
    wire<RecentSearchList>(RecentSearchListSchema),
  );
});

/** `DELETE /search/recents`: forgets them all (204). */
export const clearRecentSearches = Effect.fn("api.clearRecentSearches")(function* () {
  return yield* call({ method: "DELETE", path: "/search/recents" }, noContent);
});
