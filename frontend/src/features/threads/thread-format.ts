/** Words for threads: reply counts, "Last reply 3 hours ago", statuses. */
import { toMillis } from "../../lib/time.ts";
import type { Thread } from "../../store/model.ts";

const MINUTE_MS = 60_000;

const HOUR_MS = 60 * MINUTE_MS;

const DAY_MS = 24 * HOUR_MS;

const relative = new Intl.RelativeTimeFormat("en", { numeric: "auto" });

const shortDate = new Intl.DateTimeFormat(undefined, { month: "short", day: "numeric" });

const longDate = new Intl.DateTimeFormat(undefined, {
  month: "short",
  day: "numeric",
  year: "numeric",
});

/** "1 reply", "12 replies". */
export function replyCountLabel(count: number): string {
  return `${count} ${count === 1 ? "reply" : "replies"}`;
}

/**
 * How long ago, the way a chat says it: "just now", "5 minutes ago", "3 hours ago",
 * "yesterday", "4 days ago", then the date ("Sep 28", or "Sep 28, 2025" in another year).
 */
export function timeAgo(timestamp: string, now: number): string {
  const millis = toMillis(timestamp);
  const elapsed = Math.max(0, now - millis);

  if (elapsed < MINUTE_MS) {
    return "just now";
  }

  if (elapsed < HOUR_MS) {
    return relative.format(-Math.floor(elapsed / MINUTE_MS), "minute");
  }

  if (elapsed < DAY_MS) {
    return relative.format(-Math.floor(elapsed / HOUR_MS), "hour");
  }

  if (elapsed < 7 * DAY_MS) {
    return relative.format(-Math.floor(elapsed / DAY_MS), "day");
  }

  const sameYear = new Date(millis).getFullYear() === new Date(now).getFullYear();

  return (sameYear ? shortDate : longDate).format(millis);
}

/** "Last reply 3 hours ago" (dates read "Last reply Sep 28"). */
export function lastReplyLabel(timestamp: string, now: number): string {
  return `Last reply ${timeAgo(timestamp, now)}`;
}

export const THREAD_STATUS_LABEL = {
  active: "Active",
  closed: "Closed",
  locked: "Locked",
} as const satisfies Record<Thread["status"], string>;

/** A thread's title: its name, or "Thread" while the name is unknown or blank. */
export function threadTitle(thread: Thread | undefined): string {
  const name = thread?.name.trim() ?? "";

  return name === "" ? "Thread" : name;
}
