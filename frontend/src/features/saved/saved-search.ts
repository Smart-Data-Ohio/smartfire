/** The Saved page's URL query, parsed in the route tree (the page itself loads lazily). */

/** The page's query as the URL has it. */
export interface RawSavedSearch {
  readonly status?: unknown;
}

/** The page's query: the filter, left out at its default (in progress). */
export interface SavedSearch {
  readonly status?: "done" | "all" | undefined;
}

/** Reads `?status=`; anything unknown is the default. */
export function parseSavedSearch(search: RawSavedSearch): SavedSearch {
  const status = String(search.status ?? "");

  return { status: status === "done" || status === "all" ? status : undefined };
}
