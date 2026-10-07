/**
 * The words and numbers the cards show: relative times ("closes in 2 hours", "3 hours ago"),
 * event times, compact counts ("1.2K") and a link's host.
 */

const MINUTE = 60_000;

const HOUR = 60 * MINUTE;

const DAY = 24 * HOUR;

const relative = new Intl.RelativeTimeFormat(undefined, { numeric: "auto" });

const compact = new Intl.NumberFormat(undefined, { notation: "compact", maximumFractionDigits: 1 });

const eventDay = new Intl.DateTimeFormat(undefined, {
  weekday: "short",
  month: "short",
  day: "numeric",
});

const eventMonth = new Intl.DateTimeFormat(undefined, { month: "short" });

const eventDate = new Intl.DateTimeFormat(undefined, { day: "numeric" });

const eventTime = new Intl.DateTimeFormat(undefined, { hour: "numeric", minute: "2-digit" });

const postedDate = new Intl.DateTimeFormat(undefined, {
  month: "short",
  day: "numeric",
  year: "numeric",
});

/** The largest whole unit of a span, signed: "in 2 hours", "3 days ago", "now". */
function inUnits(deltaMs: number): string {
  const size = Math.abs(deltaMs);

  if (size < MINUTE) {
    return relative.format(0, "second");
  }

  if (size < HOUR) {
    return relative.format(Math.round(deltaMs / MINUTE), "minute");
  }

  if (size < DAY) {
    return relative.format(Math.round(deltaMs / HOUR), "hour");
  }

  return relative.format(Math.round(deltaMs / DAY), "day");
}

/** "3 hours ago" for a past timestamp. */
export function ago(timestamp: string, now: number): string {
  return inUnits(Math.min(0, Date.parse(timestamp) - now));
}

/** "Closes in 2 hours" while open; "Closed" once past. */
export function closesLabel(closesAt: string, now: number): string {
  const left = Date.parse(closesAt) - now;

  return left <= 0 ? "Closed" : `Closes ${inUnits(Math.max(left, MINUTE))}`;
}

/** "1.2K" for counts on a post. */
export function compactCount(value: number): string {
  return compact.format(value);
}

/** An event's date tile: "Oct" over "8". */
export interface EventTile {
  readonly month: string;
  readonly day: string;
}

/** The month and day for an event's date tile. */
export function eventTile(timestamp: string): EventTile {
  const millis = Date.parse(timestamp);

  return { month: eventMonth.format(millis), day: eventDate.format(millis) };
}

/** "Thu, Oct 8 · 11:00 AM – 11:45 AM" (the end's date too when it falls on another day). */
export function eventWhen(startsAt: string, endsAt: string | null): string {
  const start = Date.parse(startsAt);
  const head = `${eventDay.format(start)} · ${eventTime.format(start)}`;

  if (endsAt === null) {
    return head;
  }

  const end = Date.parse(endsAt);
  const sameDay = eventDay.format(start) === eventDay.format(end);

  return `${head} – ${sameDay ? "" : `${eventDay.format(end)} · `}${eventTime.format(end)}`;
}

/** "Oct 5, 2026": when a post went up. */
export function formatPosted(timestamp: string): string {
  return postedDate.format(Date.parse(timestamp));
}

/** A URL's host without `www.`, or the URL itself when it doesn't parse. */
export function hostOf(url: string): string {
  try {
    return new URL(url).host.replace(/^www\./, "");
  } catch {
    return url;
  }
}

/** A share of a total as a whole percentage (0 when nothing was counted). */
export function percent(part: number, total: number): number {
  return total === 0 ? 0 : Math.round((part / total) * 100);
}
