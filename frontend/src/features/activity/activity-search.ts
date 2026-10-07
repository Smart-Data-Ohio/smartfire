/** The inbox's URL query, parsed in the route tree (kept apart so the page itself loads lazily). */
import type { ActivityTab } from "../../gen/ActivityTab.ts";
import { ACTIVITY_TABS } from "../../store/activity.ts";

/** The inbox's query as the URL has it. */
export interface RawActivitySearch {
  readonly tab?: unknown;
  readonly status?: unknown;
}

/** The inbox's query: a type tab and a state, each left out at its default (all, unread). */
export interface ActivitySearch {
  readonly tab?: Exclude<ActivityTab, "all"> | undefined;
  readonly status?: "read" | "handled" | undefined;
}

/** Reads `?tab=&status=`; anything unknown is the default. */
export function parseActivitySearch(search: RawActivitySearch): ActivitySearch {
  const tab = ACTIVITY_TABS.find((each) => each === String(search.tab ?? ""));
  const status = String(search.status ?? "");

  return {
    tab: tab === undefined || tab === "all" ? undefined : tab,
    status: status === "read" || status === "handled" ? status : undefined,
  };
}
