/**
 * Scheduled-send times: the split send button's presets (Slack's: in an hour, tomorrow morning,
 * next Monday morning), the custom picker's `datetime-local` value, and how a send time reads.
 * Everything is in the viewer's local time; `now` is a parameter so tests pin the clock.
 */

export type PresetId = "hour" | "tomorrow" | "monday";

export interface SchedulePreset {
  readonly id: PresetId;
  readonly label: string;
  readonly at: Date;
}

/** The morning hour the day presets land on. */
export const MORNING_HOUR = 9;

function atMorning(day: Date): Date {
  const at = new Date(day);

  at.setHours(MORNING_HOUR, 0, 0, 0);

  return at;
}

/** The three presets, from `now`. "Monday" is next week's when today is Monday. */
export function schedulePresets(now: Date, locale?: string): readonly SchedulePreset[] {
  const hour = new Date(now);

  hour.setSeconds(0, 0);
  hour.setMinutes(hour.getMinutes() + 60);

  const tomorrow = new Date(now);

  tomorrow.setDate(tomorrow.getDate() + 1);

  const monday = new Date(now);
  const daysToMonday = (8 - monday.getDay()) % 7 || 7;

  monday.setDate(monday.getDate() + daysToMonday);

  const morning = timeLabel(atMorning(now), locale);

  return [
    { id: "hour", label: "In 1 hour", at: hour },
    { id: "tomorrow", label: `Tomorrow at ${morning}`, at: atMorning(tomorrow) },
    { id: "monday", label: `Monday at ${morning}`, at: atMorning(monday) },
  ];
}

/** "9:00 AM" (or "09:00" where the locale says so). */
export function timeLabel(date: Date, locale?: string): string {
  return new Intl.DateTimeFormat(locale, { hour: "numeric", minute: "2-digit" }).format(date);
}

function sameDay(a: Date, b: Date): boolean {
  return (
    a.getFullYear() === b.getFullYear() &&
    a.getMonth() === b.getMonth() &&
    a.getDate() === b.getDate()
  );
}

/** "Today at 3:00 PM", "Tomorrow at 9:00 AM", "Mon, Oct 12 at 9:00 AM". */
export function sendAtLabel(at: Date, now: Date, locale?: string): string {
  const tomorrow = new Date(now);

  tomorrow.setDate(tomorrow.getDate() + 1);

  const time = timeLabel(at, locale);

  if (sameDay(at, now)) {
    return `Today at ${time}`;
  }

  if (sameDay(at, tomorrow)) {
    return `Tomorrow at ${time}`;
  }

  const day = new Intl.DateTimeFormat(locale, {
    weekday: "short",
    month: "short",
    day: "numeric",
    year: at.getFullYear() === now.getFullYear() ? undefined : "numeric",
  }).format(at);

  return `${day} at ${time}`;
}

function pad(value: number): string {
  return String(value).padStart(2, "0");
}

/** A `datetime-local` input's value for `date`, in local time, to the minute. */
export function toLocalInput(date: Date): string {
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}T${pad(
    date.getHours(),
  )}:${pad(date.getMinutes())}`;
}

/** The local time a `datetime-local` value names, or `null` when it's empty or malformed. */
export function fromLocalInput(value: string): Date | null {
  const match = /^(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2})$/.exec(value);

  if (match === null) {
    return null;
  }

  const [, year, month, day, hours, minutes] = match.map(Number);
  const date = new Date(year ?? 0, (month ?? 1) - 1, day ?? 1, hours ?? 0, minutes ?? 0);

  return Number.isNaN(date.getTime()) ? null : date;
}

/**
 * Why a custom time can't be used, or `null` when it can: the server wants it in the future, and
 * the scheduler only checks every 30 s, so a minute ahead is the useful minimum.
 */
export function customTimeProblem(at: Date | null, now: Date): string | null {
  if (at === null) {
    return "Pick a date and time.";
  }

  return at.getTime() - now.getTime() < 60_000 ? "Pick a time at least a minute from now." : null;
}
