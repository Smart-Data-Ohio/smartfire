/**
 * How search results word their times and statuses. Hits span days, so each carries its day:
 * "Today at 9:41 AM", "Yesterday at 9:41 AM", "Oct 5 at 9:41 AM", "Oct 5, 2025 at 9:41 AM".
 */
import type { SearchChip } from "../../gen/SearchChip.ts";
import type { SearchSectionKind } from "../../gen/SearchSectionKind.ts";
import type { WorkStatus } from "../../gen/WorkStatus.ts";
import { formatTime, toMillis } from "../../lib/time.ts";
import type { SearchResultsView } from "../../store/search-hooks.ts";

const monthDay = new Intl.DateTimeFormat(undefined, { month: "short", day: "numeric" });

const monthDayYear = new Intl.DateTimeFormat(undefined, {
  month: "short",
  day: "numeric",
  year: "numeric",
});

const DAY_MS = 86_400_000;

function startOfDay(millis: number): number {
  const date = new Date(millis);

  date.setHours(0, 0, 0, 0);

  return date.getTime();
}

/** The day part alone: "Today", "Yesterday", "Tomorrow", "Oct 5" or "Oct 5, 2025". */
export function hitDay(timestamp: string, now: number): string {
  const millis = toMillis(timestamp);
  const days = Math.round((startOfDay(now) - startOfDay(millis)) / DAY_MS);

  if (days === 0) return "Today";

  if (days === 1) return "Yesterday";

  if (days === -1) return "Tomorrow";

  return new Date(millis).getFullYear() === new Date(now).getFullYear()
    ? monthDay.format(millis)
    : monthDayYear.format(millis);
}

/** A hit's time with its day. */
export function hitTime(timestamp: string, now: number): string {
  return `${hitDay(timestamp, now)} at ${formatTime(timestamp)}`;
}

export const WORK_STATUS_LABEL = {
  planned: "Planned",
  in_progress: "In progress",
  blocked: "Blocked",
  done: "Done",
} as const satisfies Record<WorkStatus, string>;

export const SECTION_TITLE = {
  board_posts: "Board posts",
  work_threads: "Work threads",
  events: "Events",
} as const satisfies Record<SearchSectionKind, string>;

/** The count beside the Messages heading: "14", or "40+" while older pages remain. */
export function messageCount(count: number, more: boolean): string {
  return `${count}${more ? "+" : ""}`;
}

/** A chip's label; the server labels `is:thread` by its value ("is: true"), which reads oddly. */
export function chipLabel(chip: SearchChip): string {
  return chip.operator === "is" ? "is: thread" : chip.label;
}

/** What a screen reader hears about the results as they change; "" when nothing is searched. */
export function resultsAnnouncement(
  results: Pick<
    SearchResultsView,
    "query" | "status" | "error" | "messages" | "sections" | "hasMore" | "loadingMore" | "moreError"
  >,
): string {
  if (results.query === "") {
    return "";
  }

  if (results.status === "error") {
    return `Search failed. ${results.error ?? "Something went wrong."}`;
  }

  if (results.status !== "ready") {
    return "Searching…";
  }

  if (results.loadingMore) {
    return "Loading older messages…";
  }

  if (results.moreError !== null) {
    return "Couldn't load more results.";
  }

  const others = results.sections.reduce((total, section) => total + section.rows.length, 0);
  const count = results.messages.length;

  if (count === 0 && others === 0) {
    return `No results for “${results.query}”`;
  }

  const noun = count === 1 && !results.hasMore ? "message" : "messages";
  const messages = `${messageCount(count, results.hasMore)} ${noun}`;

  const extra =
    others === 0 ? "" : `, ${others} ${others === 1 ? "other result" : "other results"}`;

  const stale = results.error === null ? "" : `. Showing earlier results: ${results.error}`;

  return `${messages}${extra}${stale}`;
}
