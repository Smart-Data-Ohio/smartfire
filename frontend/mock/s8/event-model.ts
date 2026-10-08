import type { ApiError } from "../../src/gen/ApiError.ts";
import type { AttendanceResponse } from "../../src/gen/AttendanceResponse.ts";
import type { EventRecurrenceRule } from "../../src/gen/EventRecurrenceRule.ts";
import type { EventValues } from "../../src/gen/EventValues.ts";
import { HttpError } from "../http.ts";
import { booleanField, field, intField, type Json, stringField } from "../json.ts";
import { escapeHtml } from "../markdown.ts";
import type { World } from "../seed.ts";

export const DAY = 86_400_000;

const VALIDATION = "Validation" satisfies ApiError["_tag"];

export interface CalendarRecord {
  readonly id: number;
  readonly roomId: number;
  readonly organizerId: number;
  readonly seriesId: number | null;
  values: EventValues;
  startsAt: string;
  endsAt: string | null;
  cancelled: boolean;
  readonly responses: Map<number, AttendanceResponse>;
  readonly calendarCopies: Set<number>;
}

export function recurrenceRule(value: string | null): EventRecurrenceRule | null {
  switch (value) {
    case "daily":
    case "weekly":
    case "biweekly":
    case "monthly":
      return value;
    default:
      return null;
  }
}

export function phrase(rule: EventRecurrenceRule): string {
  return rule === "biweekly" ? "every two weeks" : rule;
}

/** Rails aliases needed by the fixtures; the browser and new screens normally send IANA zones. */
function zoneName(zone: string): string {
  switch (zone) {
    case "Eastern Time (US & Canada)":
      return "America/New_York";
    case "Pacific Time (US & Canada)":
      return "America/Los_Angeles";
    default:
      return zone;
  }
}

export function knownZone(zone: string): boolean {
  try {
    new Intl.DateTimeFormat("en-US", { timeZone: zoneName(zone) }).format(0);

    return zone.trim() !== "";
  } catch {
    return false;
  }
}

export function localTime(instant: string, zone: string): string {
  const parts = new Intl.DateTimeFormat("en-CA", {
    timeZone: zoneName(zone),
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    hourCycle: "h23",
  }).formatToParts(new Date(instant));

  const part = (kind: string) => parts.find((value) => value.type === kind)?.value ?? "";

  return `${part("year")}-${part("month")}-${part("day")}T${part("hour")}:${part("minute")}`;
}

/** Convert local clock input using the event's zone, never the Node process's zone. */
export function instant(value: string | null, zone: string): string | null {
  if (value === null || value.trim() === "" || !knownZone(zone)) return null;

  if (/([zZ]|[+-]\d{2}:?\d{2})$/.test(value)) {
    const parsed = Date.parse(value);

    return Number.isFinite(parsed) ? new Date(parsed).toISOString() : null;
  }

  if (!/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}(:\d{2})?$/.test(value)) return null;

  const wanted = Date.parse(`${value}Z`);

  if (!Number.isFinite(wanted)) return null;

  let guess = wanted;

  for (let attempt = 0; attempt < 4; attempt += 1) {
    const shown = Date.parse(`${localTime(new Date(guess).toISOString(), zone)}Z`);
    const shift = wanted - shown;

    if (shift === 0) break;

    guess += shift;
  }

  return new Date(guess).toISOString();
}

export function zoneLabel(start: string, end: string | null, zone: string): string {
  const clock = (value: string) =>
    new Intl.DateTimeFormat("en-US", {
      timeZone: zoneName(zone),
      hour: "numeric",
      minute: "2-digit",
    }).format(new Date(value));

  const abbreviation = new Intl.DateTimeFormat("en-US", {
    timeZone: zoneName(zone),
    timeZoneName: "short",
  })
    .formatToParts(new Date(start))
    .find((value) => value.type === "timeZoneName")?.value;

  return `(${clock(start)}${end === null ? "" : `–${clock(end)}`} ${abbreviation ?? zone})`;
}

export function displayDate(date: string): string {
  return new Intl.DateTimeFormat("en-US", {
    timeZone: "UTC",
    year: "numeric",
    month: "long",
    day: "numeric",
  }).format(new Date(`${date}T12:00:00Z`));
}

/** Monthly repeats retain the original day, clamped to the destination month's last day. */
export function repeatSlots(values: EventValues): readonly string[] {
  const start = instant(values.startsAt, values.timeZone);
  const until = values.recurrenceUntil;
  const rule = values.recurrenceRule;

  if (start === null || until === null || rule === null) return [];

  const local = localTime(start, values.timeZone);
  const first = new Date(`${local.slice(0, 10)}T12:00:00Z`);
  const result: string[] = [];

  for (let index = 0; index < 367; index += 1) {
    const date = new Date(first);

    if (rule === "monthly") {
      date.setUTCDate(1);
      date.setUTCMonth(first.getUTCMonth() + index);

      const last = new Date(date);

      last.setUTCMonth(last.getUTCMonth() + 1, 0);
      date.setUTCDate(Math.min(first.getUTCDate(), last.getUTCDate()));
    } else {
      let days = 7;

      if (rule === "daily") days = 1;

      if (rule === "biweekly") days = 14;

      date.setUTCDate(first.getUTCDate() + index * days);
    }

    const day = date.toISOString().slice(0, 10);

    if (day > until) break;

    const slot = instant(`${day}${local.slice(10)}`, values.timeZone);

    if (slot !== null) result.push(slot);
  }

  return result;
}

/** Ordinary descriptions mirror classic paragraph and line-break formatting, safely escaped. */
export function descriptionHtml(description: string | null): string | null {
  if (description === null || description.trim() === "") return null;

  return escapeHtml(description.replace(/\r\n?/g, "\n"))
    .split(/\n\n+/)
    .map((paragraph) => `<p>${paragraph.replaceAll("\n", "\n<br />")}</p>`)
    .join("\n\n");
}

export function formValues(body: Json | undefined, stored: EventValues | null): EventValues {
  const text = (key: string, fallback: string | null) =>
    field(body, key) === undefined ? fallback : stringField(body, key);

  return {
    title: text("title", stored?.title ?? "") ?? "",
    description: text("description", stored?.description ?? null),
    startsAt: stringField(body, "startsAt"),
    endsAt: stringField(body, "endsAt"),
    timeZone: stored?.timeZone ?? text("timeZone", "UTC") ?? "UTC",
    venueRoomId:
      field(body, "venueRoomId") === undefined
        ? (stored?.venueRoomId ?? null)
        : intField(body, "venueRoomId"),
    recurrenceRule: recurrenceRule(text("recurrenceRule", stored?.recurrenceRule ?? null)),
    recurrenceUntil: text("recurrenceUntil", stored?.recurrenceUntil ?? null),
    meetLinkRequested:
      booleanField(body, "meetLinkRequested") ?? stored?.meetLinkRequested ?? false,
    meetLink: stored?.meetLink ?? null,
  };
}

export function validateValues(
  values: EventValues,
  body: Json | undefined,
  world: World,
  organizerId: number,
  current: CalendarRecord | null = null,
): void {
  const fields: Record<string, string[]> = {};

  const add = (key: string, message: string) => {
    fields[key] ??= [];
    fields[key].push(message);
  };

  const start = instant(values.startsAt, values.timeZone);
  const end = instant(values.endsAt, values.timeZone);

  if (values.title.trim() === "") add("title", "can't be blank");

  if (start === null) add("startsAt", "can't be blank");

  if (!knownZone(values.timeZone)) add("timeZone", "is invalid");

  if (start !== null && end !== null && end <= start) add("endsAt", "must be after the start time");

  const rawRule = stringField(body, "recurrenceRule");

  if (rawRule !== null && rawRule.trim() !== "" && recurrenceRule(rawRule) === null)
    add("recurrenceRule", "is not included in the list");

  if (values.venueRoomId !== null && current?.values.venueRoomId !== values.venueRoomId) {
    const venue = world.rooms.get(values.venueRoomId);

    if (
      venue === undefined ||
      !["voice", "stage"].includes(venue.room.kind) ||
      !venue.memberIds.includes(organizerId)
    )
      add("venueRoomId", "must be a voice or Stage channel you belong to");
  }

  if (
    values.recurrenceRule !== null &&
    start !== null &&
    (current === null || current.seriesId === null || current.seriesId === current.id)
  ) {
    const startDay = localTime(start, values.timeZone).slice(0, 10);
    const until = values.recurrenceUntil;

    if (until === null || !/^\d{4}-\d{2}-\d{2}$/.test(until)) {
      add("recurrenceUntil", "can't be blank");
    } else {
      const max = new Date(`${startDay}T12:00:00Z`);

      max.setUTCFullYear(max.getUTCFullYear() + 1);

      if (until <= startDay) add("recurrenceUntil", "must be after the start date");

      if (until > max.toISOString().slice(0, 10))
        add("recurrenceUntil", "must be at most one year after the start date");

      const count = repeatSlots(values).length;

      if (count > 52)
        add(
          "recurrenceUntil",
          `would create ${count} occurrences (maximum 52); pick an earlier end date`,
        );
    }
  }

  if (Object.keys(fields).length > 0) {
    throw new HttpError(422, { _tag: VALIDATION, message: "Validation failed", fields });
  }
}
