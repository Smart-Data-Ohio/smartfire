/** The work page's URL query, parsed in the route tree (the page itself loads lazily). */
import type { WorkFilter } from "../../gen/WorkFilter.ts";

/** The page's query as the URL has it. */
export interface RawWorkSearch {
  readonly state?: unknown;
}

/** The page's query: the tab, left out at its default (open). */
export interface WorkSearch {
  readonly state?: Exclude<WorkFilter, "open"> | undefined;
}

const OTHER_TABS: readonly Exclude<WorkFilter, "open">[] = ["done", "all", "agents", "boards"];

/** Reads `?state=`; anything unknown is the default, as the server reads it. */
export function parseWorkSearch(search: RawWorkSearch): WorkSearch {
  const state = String(search.state ?? "");

  return { state: OTHER_TABS.find((tab) => tab === state) };
}
