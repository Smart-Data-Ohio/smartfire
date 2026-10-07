/**
 * The event form's pure logic: the draft a form starts from, the bodies it sends (create and the
 * classic edit's fields), and the words for the server's field errors.
 */
import type { CreateEvent } from "../../gen/CreateEvent.ts";
import type { EventForm } from "../../gen/EventForm.ts";
import type { EventScope } from "../../gen/EventScope.ts";
import type { EventVenueOption } from "../../gen/EventVenueOption.ts";
import type { UpdateEvent } from "../../gen/UpdateEvent.ts";
import { toLocalInput } from "../composer/schedule/presets.ts";

/** What the form edits; selects hold strings ("" for none), as their controls do. */
export interface EventDraft {
  readonly title: string;
  readonly description: string;
  readonly startsAt: string;
  readonly endsAt: string;
  readonly venueRoomId: string;
  readonly recurrenceRule: string;
  readonly recurrenceUntil: string;
  readonly meetLinkRequested: boolean;
  readonly scope: EventScope;
}

/** The browser's zone, which a new event is scheduled in (classic's form script does the same). */
export function browserZone(): string {
  try {
    return Intl.DateTimeFormat().resolvedOptions().timeZone || "UTC";
  } catch {
    return "UTC";
  }
}

/** The next whole hour from `now`, as a `datetime-local` value in the browser's zone. */
export function nextHour(now: Date): string {
  const at = new Date(now);

  at.setMinutes(0, 0, 0);
  at.setHours(at.getHours() + 1);

  return toLocalInput(at);
}

/**
 * The draft a form opens with: the event's values, or for a new one the server's (blank) values
 * with a start at the next whole hour, so the common case is a title away.
 */
export function initialDraft(form: EventForm, now: Date): EventDraft {
  const { values } = form;

  return {
    title: values.title,
    description: values.description ?? "",
    startsAt: values.startsAt ?? (form.eventId === null ? nextHour(now) : ""),
    endsAt: values.endsAt ?? "",
    venueRoomId: values.venueRoomId === null ? "" : `${values.venueRoomId}`,
    recurrenceRule: values.recurrenceRule ?? "",
    recurrenceUntil: values.recurrenceUntil?.slice(0, 10) ?? "",
    meetLinkRequested: values.meetLinkRequested,
    scope: "this_event",
  };
}

const blankToNull = (value: string): string | null => (value.trim() === "" ? null : value);

const venueOf = (value: string): number | null => (value === "" ? null : Number(value));

/** Whether the repeat controls show: a new event, or a series' first event. */
export function showsRepeat(form: EventForm): boolean {
  return form.eventId === null || form.ruleEditable;
}

/** `POST .../events`: every control, in the zone the form schedules it in. */
export function createBody(draft: EventDraft, timeZone: string): CreateEvent {
  const rule = blankToNull(draft.recurrenceRule);

  return {
    title: draft.title.trim(),
    description: blankToNull(draft.description),
    startsAt: blankToNull(draft.startsAt),
    endsAt: blankToNull(draft.endsAt),
    timeZone,
    venueRoomId: venueOf(draft.venueRoomId),
    recurrenceRule: rule,
    recurrenceUntil: rule === null ? null : blankToNull(draft.recurrenceUntil),
    meetLinkRequested: draft.meetLinkRequested,
  };
}

/**
 * `PATCH .../events/:id`: the controls the classic edit form shows. Its times always go (the
 * server reads a missing one as cleared); the repeat controls only on a series' first event, the
 * Meet box only when it's offered, and the scope only for a series.
 */
export function updateBody(form: EventForm, draft: EventDraft): UpdateEvent {
  const body: UpdateEvent = {
    title: draft.title.trim(),
    description: blankToNull(draft.description),
    startsAt: blankToNull(draft.startsAt),
    endsAt: blankToNull(draft.endsAt),
    venueRoomId: venueOf(draft.venueRoomId),
  };

  if (form.meetAvailable) {
    body.meetLinkRequested = draft.meetLinkRequested;
  }

  if (form.ruleEditable) {
    body.recurrenceRule = blankToNull(draft.recurrenceRule);
    body.recurrenceUntil = blankToNull(draft.recurrenceUntil);
  }

  if (form.series) {
    body.updateScope = draft.scope;
  }

  return body;
}

/** Whether the draft differs from what the form opened with. */
export function isDirty(initial: EventDraft, draft: EventDraft): boolean {
  return DRAFT_KEYS.some((key) => initial[key] !== draft[key]);
}

const DRAFT_KEYS = [
  "title",
  "description",
  "startsAt",
  "endsAt",
  "venueRoomId",
  "recurrenceRule",
  "recurrenceUntil",
  "meetLinkRequested",
  "scope",
] as const satisfies readonly (keyof EventDraft)[];

/** Voice channels, then stages: the classic select's two groups (an empty one isn't shown). */
export function venueGroups(venues: readonly EventVenueOption[]) {
  return [
    { label: "Voice", options: venues.filter((venue) => venue.kind !== "stage") },
    { label: "Stage", options: venues.filter((venue) => venue.kind === "stage") },
  ].filter((group) => group.options.length > 0);
}

/** The controls the server names in its field errors. */
export type EventField =
  | "title"
  | "description"
  | "startsAt"
  | "endsAt"
  | "timeZone"
  | "venueRoomId"
  | "recurrenceRule"
  | "recurrenceUntil";

const FIELD_NAME = {
  title: "Title",
  description: "Description",
  startsAt: "Start",
  endsAt: "End",
  timeZone: "Time zone",
  venueRoomId: "Where",
  recurrenceRule: "Repeats",
  recurrenceUntil: "Repeat until",
} as const satisfies Record<EventField, string>;

const isField = (key: string): key is EventField => key in FIELD_NAME;

/** "Title can't be blank.": a field error as a sentence, the field named the way the form does. */
export function fieldMessage(field: EventField, message: string): string {
  return `${FIELD_NAME[field]} ${message}.`;
}

/** The server's field errors split into the ones a control shows and the rest (a summary). */
export function splitErrors(fields: Readonly<Record<string, readonly string[]>>) {
  const byField: Partial<Record<EventField, string>> = {};
  const other: string[] = [];

  for (const [key, messages] of Object.entries(fields)) {
    const first = messages[0];

    if (first === undefined) {
      continue;
    }

    if (isField(key)) {
      byField[key] = fieldMessage(key, first);
    } else {
      other.push(...messages.map((message) => `${key} ${message}.`));
    }
  }

  return { byField, other };
}
