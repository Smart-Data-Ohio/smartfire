/**
 * Timestamps as people read them in a chat: "9:41 AM" on a row, "Today" / "Yesterday" /
 * "Monday, October 5" on a day divider. Wire timestamps are RFC 3339 strings; these helpers
 * parse them once and format in the browser's locale and zone.
 */

const timeFormat = new Intl.DateTimeFormat(undefined, { hour: "numeric", minute: "2-digit" });

const weekdayFormat = new Intl.DateTimeFormat(undefined, {
  weekday: "long",
  month: "long",
  day: "numeric",
});

const yearFormat = new Intl.DateTimeFormat(undefined, {
  weekday: "short",
  month: "long",
  day: "numeric",
  year: "numeric",
});

const fullFormat = new Intl.DateTimeFormat(undefined, { dateStyle: "full", timeStyle: "short" });

const shortWeekdayFormat = new Intl.DateTimeFormat(undefined, { weekday: "short" });

const shortDateFormat = new Intl.DateTimeFormat(undefined, { month: "short", day: "numeric" });

const shortYearFormat = new Intl.DateTimeFormat(undefined, {
  month: "short",
  day: "numeric",
  year: "numeric",
});

const DAY_MS = 86_400_000;

/** Milliseconds since the epoch for a wire timestamp. */
export function toMillis(timestamp: string): number {
  return Date.parse(timestamp);
}

/** "9:41 AM" (or "09:41" where the locale says so). */
export function formatTime(timestamp: string): string {
  return timeFormat.format(toMillis(timestamp));
}

/** "Tuesday, October 6, 2026 at 9:41 AM": the tooltip on a row's time. */
export function formatFull(timestamp: string): string {
  return fullFormat.format(toMillis(timestamp));
}

/** The local calendar day a timestamp falls on, as a sortable key ("2026-10-06"). */
export function dayKey(millis: number): string {
  const date = new Date(millis);
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");

  return `${date.getFullYear()}-${month}-${day}`;
}

function startOfDay(millis: number): number {
  const date = new Date(millis);

  date.setHours(0, 0, 0, 0);

  return date.getTime();
}

/** A day divider's label: "Today", "Yesterday", the weekday and date, or the full date in another year. */
export function formatDay(millis: number, now: number): string {
  const days = Math.round((startOfDay(now) - startOfDay(millis)) / DAY_MS);

  if (days === 0) {
    return "Today";
  }

  if (days === 1) {
    return "Yesterday";
  }

  return new Date(millis).getFullYear() === new Date(now).getFullYear()
    ? weekdayFormat.format(millis)
    : yearFormat.format(millis);
}

/** A list row's time: "9:41 AM" today, then "Yesterday", the weekday within a week, "Oct 3". */
export function formatListTime(timestamp: string, now: number): string {
  const millis = toMillis(timestamp);
  const days = Math.round((startOfDay(now) - startOfDay(millis)) / DAY_MS);

  if (days <= 0) {
    return timeFormat.format(millis);
  }

  if (days === 1) {
    return "Yesterday";
  }

  if (days < 7) {
    return shortWeekdayFormat.format(millis);
  }

  return new Date(millis).getFullYear() === new Date(now).getFullYear()
    ? shortDateFormat.format(millis)
    : shortYearFormat.format(millis);
}
