/**
 * A small stand-in for the server's natural-language time parser, enough for the slash
 * commands: `in 20 minutes`, `2h`, `tomorrow 9am`, `friday 5pm`, `2026-10-09`, `17:30`, read in
 * the viewer's time zone. It also writes times the way the commands quote them back.
 */

const MINUTE = 60_000;

const HOUR = 60 * MINUTE;

const DAY = 24 * HOUR;

const UNIT_MS = new Map([
  ["m", MINUTE],
  ["h", HOUR],
  ["d", DAY],
  ["w", 7 * DAY],
]);

const WEEKDAYS = ["sunday", "monday", "tuesday", "wednesday", "thursday", "friday", "saturday"];

/** What a day with no clock time means: 9:00 (reminders, events) or the day's end (`/ooo`). */
export type BareDay = "morning" | "end_of_day";

interface LocalParts {
  readonly year: number;
  /** 0-based. */
  readonly month: number;
  readonly day: number;
  readonly hour: number;
  readonly minute: number;
  readonly second: number;
  readonly weekday: number;
}

const formatters = new Map<string, Intl.DateTimeFormat>();

function formatter(zone: string): Intl.DateTimeFormat {
  const cached = formatters.get(zone);

  if (cached !== undefined) return cached;

  const created = new Intl.DateTimeFormat("en-US", {
    timeZone: zone,
    hourCycle: "h23",
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
    weekday: "long",
  });

  formatters.set(zone, created);

  return created;
}

function localParts(ms: number, zone: string): LocalParts {
  const parts = new Map(
    formatter(zone)
      .formatToParts(ms)
      .map((part) => [part.type, part.value]),
  );

  const number = (type: Intl.DateTimeFormatPartTypes) => Number(parts.get(type) ?? "0");

  return {
    year: number("year"),
    month: number("month") - 1,
    day: number("day"),
    hour: number("hour") % 24,
    minute: number("minute"),
    second: number("second"),
    weekday: WEEKDAYS.indexOf((parts.get("weekday") ?? "").toLowerCase()),
  };
}

/** The instant a wall-clock time in `zone` names. */
export function zonedTime(
  zone: string,
  year: number,
  month: number,
  day: number,
  hour: number,
  minute: number,
  second = 0,
): number {
  const wall = Date.UTC(year, month, day, hour, minute, second);

  const offsetAt = (ms: number) => {
    const local = localParts(ms, zone);

    return (
      Date.UTC(local.year, local.month, local.day, local.hour, local.minute, local.second) -
      Math.floor(ms / 1000) * 1000
    );
  };

  const guess = wall - offsetAt(wall);

  return wall - offsetAt(guess);
}

/** `%B %d, %Y %H:%M` in `zone`, as the commands quote a time. */
export function longTime(ms: number, zone: string): string {
  return `${longDate(ms, zone)} ${clock(ms, zone)}`;
}

/** `%B %d, %Y` in `zone`. */
export function longDate(ms: number, zone: string): string {
  const local = localParts(ms, zone);

  const month = new Date(Date.UTC(2000, local.month, 1)).toLocaleString("en-US", {
    month: "long",
    timeZone: "UTC",
  });

  return `${month} ${String(local.day).padStart(2, "0")}, ${local.year}`;
}

function clock(ms: number, zone: string): string {
  const local = localParts(ms, zone);

  return `${String(local.hour).padStart(2, "0")}:${String(local.minute).padStart(2, "0")}`;
}

/** A clock time like `9am`, `9:30 pm`, `17:00` or `noon`, as hours and minutes. */
function parseClock(text: string): readonly [number, number] | null {
  const word = text.toLowerCase();

  if (word === "noon") return [12, 0];

  if (word === "midnight") return [0, 0];

  const match = /^(\d{1,2})(?::(\d{2}))?\s*(am|pm)?$/i.exec(text);

  if (match === null) return null;

  let hour = Number(match[1]);
  const minute = Number(match[2] ?? "0");
  const meridiem = match[3]?.toLowerCase();

  if (match[2] === undefined && meridiem === undefined) return null;

  if (meridiem !== undefined) {
    if (hour < 1 || hour > 12) return null;

    hour = (hour % 12) + (meridiem === "pm" ? 12 : 0);
  }

  return hour < 24 && minute < 60 ? [hour, minute] : null;
}

/** A parsed time and the words after it. */
export interface Leading {
  readonly at: number;
  readonly rest: string;
}

/** A time at the start of `text`, and what follows it; `null` when it doesn't start with one. */
export function leadingTime(
  text: string,
  now: number,
  zone: string,
  bare: BareDay,
): Leading | null {
  const input = text.trim();

  const duration =
    /^(?:in\s+)?(\d+)\s*(m(?:ins?|inutes?)?|h(?:rs?|ours?)?|d(?:ays?)?|w(?:eeks?)?)\b\s*(.*)$/is.exec(
      input,
    );

  if (duration !== null) {
    const amount = Number(duration[1]);
    const unit = UNIT_MS.get((duration[2] ?? "m").charAt(0).toLowerCase()) ?? MINUTE;

    return amount > 0 ? { at: now + amount * unit, rest: (duration[3] ?? "").trim() } : null;
  }

  const words = input.split(/\s+/);
  const first = (words[0] ?? "").toLowerCase();
  const today = localParts(now, zone);
  let day: readonly [number, number, number] | null = null;
  let used = 1;

  const shifted = (days: number): readonly [number, number, number] => {
    const date = new Date(Date.UTC(today.year, today.month, today.day + days));

    return [date.getUTCFullYear(), date.getUTCMonth(), date.getUTCDate()];
  };

  const iso = /^(\d{4})-(\d{2})-(\d{2})$/.exec(first);

  if (first === "today") {
    day = shifted(0);
  } else if (first === "tomorrow") {
    day = shifted(1);
  } else if (WEEKDAYS.includes(first)) {
    const ahead = (WEEKDAYS.indexOf(first) - today.weekday + 7) % 7 || 7;

    day = shifted(ahead);
  } else if (iso !== null) {
    day = [Number(iso[1]), Number(iso[2]) - 1, Number(iso[3])];
  } else {
    used = 0;
  }

  const clockWords = [words.slice(used, used + 2).join(" "), words[used] ?? ""];
  let time: readonly [number, number] | null = null;

  for (const candidate of clockWords) {
    time = parseClock(candidate);

    if (time !== null) {
      used += candidate.split(/\s+/).length;
      break;
    }
  }

  if (day === null && time === null) return null;

  const rest = words.slice(used).join(" ");

  if (day === null && time !== null) {
    const at = zonedTime(zone, today.year, today.month, today.day, time[0], time[1]);
    const [y, m, d] = shifted(1);

    return {
      at: at > now ? at : zonedTime(zone, y, m, d, time[0], time[1]),
      rest,
    };
  }

  const [year, month, date] = day ?? shifted(0);

  if (time !== null) return { at: zonedTime(zone, year, month, date, time[0], time[1]), rest };

  return {
    at:
      bare === "morning"
        ? zonedTime(zone, year, month, date, 9, 0)
        : zonedTime(zone, year, month, date, 23, 59, 59),
    rest,
  };
}

/** A title with an optional time after it. */
export interface Trailing {
  readonly title: string;
  readonly at: number | null;
}

/** A time at the end of `text` (`/event Launch party friday 5pm`): the title and the time. */
export function trailingTime(text: string, now: number, zone: string): Trailing {
  const words = text.trim().split(/\s+/);

  for (let start = 1; start < words.length; start++) {
    const tail = words.slice(start).join(" ");
    const parsed = leadingTime(tail, now, zone, "morning");

    if (parsed !== null && parsed.rest === "") {
      return { title: words.slice(0, start).join(" "), at: parsed.at };
    }
  }

  return { title: text.trim(), at: null };
}

/** The last second of the local day `ms` falls on, in `zone`. */
export function endOfLocalDay(ms: number, zone: string): number {
  const local = localParts(ms, zone);

  return zonedTime(zone, local.year, local.month, local.day, 23, 59, 59);
}
