/** The Work page's URL query, parsed in the route tree (the page itself loads lazily). */
import type { WorkFilter } from "../../gen/WorkFilter.ts";

/** The page's filters, in the classic page's order. */
export const WORK_FILTERS = [
  "open",
  "done",
  "all",
  "agents",
  "boards",
] as const satisfies readonly WorkFilter[];

/** The page's query as the URL has it. */
export interface RawWorkSearch {
  readonly state?: unknown;
}

/** The page's query: the filter, left out at its default (open). */
export interface WorkSearch {
  readonly state?: Exclude<WorkFilter, "open"> | undefined;
}

export function isWorkFilter(value: string): value is WorkFilter {
  return WORK_FILTERS.some((filter) => filter === value);
}

/** Reads `?state=`; anything unknown is the default, as `work_threads#index` reads it. */
export function parseWorkSearch(search: RawWorkSearch): WorkSearch {
  const state = WORK_FILTERS.find((filter) => filter === search.state);

  return { state: state === undefined || state === "open" ? undefined : state };
}
