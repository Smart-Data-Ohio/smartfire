/**
 * The words and numbers the cards show: relative times ("closes in 2 hours", "3 hours ago"),
 * event times, compact counts ("1.2K") and a link's host.
 */

const MINUTE = 60_000;

const HOUR = 60 * MINUTE;

const DAY = 24 * HOUR;

const relative = new Intl.RelativeTimeFormat(undefined, { numeric: "auto" });

const compact = new Intl.NumberFormat(undefined, { notation: "compact", maximumFractionDigits: 1 });

/** An event's formatters, all in the zone its times were set in. */
interface EventFormats {
  /** "Thu, Oct 8, 2026" (classic always shows the year). */
  readonly day: Intl.DateTimeFormat;
  readonly month: Intl.DateTimeFormat;
  readonly date: Intl.DateTimeFormat;
  /** "11:00 AM". */
  readonly time: Intl.DateTimeFormat;
  /** "11:45 AM EDT": the last time shown carries the zone. */
  readonly zonedTime: Intl.DateTimeFormat;
}

const eventFormats = new Map<string, EventFormats>();

function makeEventFormats(timeZone: string | undefined): EventFormats {
  return {
    day: new Intl.DateTimeFormat(undefined, {
      weekday: "short",
      month: "short",
      day: "numeric",
      year: "numeric",
      timeZone,
    }),
    month: new Intl.DateTimeFormat(undefined, { month: "short", timeZone }),
    date: new Intl.DateTimeFormat(undefined, { day: "numeric", timeZone }),
    time: new Intl.DateTimeFormat(undefined, { hour: "numeric", minute: "2-digit", timeZone }),
    zonedTime: new Intl.DateTimeFormat(undefined, {
      hour: "numeric",
      minute: "2-digit",
      timeZoneName: "short",
      timeZone,
    }),
  };
}

/** The formatters for `timeZone`, made once; the viewer's own zone if this browser doesn't know it. */
function formatsIn(timeZone: string): EventFormats {
  const held = eventFormats.get(timeZone);

  if (held !== undefined) {
    return held;
  }

  let made: EventFormats;

  try {
    made = makeEventFormats(timeZone);
  } catch {
    made = makeEventFormats(undefined);
  }

  eventFormats.set(timeZone, made);

  return made;
}

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

/** The month and day for an event's date tile, in the event's zone. */
export function eventTile(timestamp: string, timeZone: string): EventTile {
  const millis = Date.parse(timestamp);
  const formats = formatsIn(timeZone);

  return { month: formats.month.format(millis), day: formats.date.format(millis) };
}

/**
 * "Thu, Oct 8, 2026 · 11:00 AM – 11:45 AM EDT": when an event is, in the zone its organizer set
 * it in (as classic shows it), with that zone named; the end's date too when it falls on another
 * day.
 */
export function eventWhen(startsAt: string, endsAt: string | null, timeZone: string): string {
  const formats = formatsIn(timeZone);
  const start = Date.parse(startsAt);
  const startDay = formats.day.format(start);

  if (endsAt === null) {
    return `${startDay} · ${unbroken(formats.zonedTime.format(start))}`;
  }

  const end = Date.parse(endsAt);
  const endDay = formats.day.format(end);
  const endTime = unbroken(formats.zonedTime.format(end));
  const endPart = `${startDay === endDay ? "" : `${endDay} · `}${endTime}`;

  return `${startDay} · ${unbroken(formats.time.format(start))} – ${endPart}`;
}

/** A time that wraps as one piece ("11:45 AM EDT"), so a narrow card breaks between parts. */
function unbroken(text: string): string {
  return text.replaceAll(" ", "\u00a0");
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
