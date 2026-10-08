/** Channel calendar mock. State follows the current World, so server.reset restores the seed. */

import type { ApiError } from "../../src/gen/ApiError.ts";
import type { ChannelEvent } from "../../src/gen/ChannelEvent.ts";
import type { EventAttendance } from "../../src/gen/EventAttendance.ts";
import type { EventCard } from "../../src/gen/EventCard.ts";
import type { EventDetail } from "../../src/gen/EventDetail.ts";
import type { EventForm } from "../../src/gen/EventForm.ts";
import type { EventList } from "../../src/gen/EventList.ts";
import type { EventRepeatOption } from "../../src/gen/EventRepeatOption.ts";
import type { EventValues } from "../../src/gen/EventValues.ts";
import { forbidden, HttpError, notFound, ok, validation } from "../http.ts";
import { booleanField, type Json, stringField } from "../json.ts";
import { firstId, route, type S2Context } from "../s2/context.ts";
import { plainDraft } from "../s2/model.ts";
import { ROOM_IDS, timestamp, USER_IDS, VIEWER_ID, type World } from "../seed.ts";
import {
  type CalendarRecord,
  DAY,
  descriptionHtml,
  displayDate,
  formValues,
  instant,
  knownZone,
  localTime,
  phrase,
  repeatSlots,
  validateValues,
  zoneLabel,
} from "./event-model.ts";

export const EVENT_IDS = {
  upcoming: 8001,
  past: 8002,
  cancelled: 8003,
  weeklyHead: 8100,
  weeklySecond: 8101,
  weeklyThird: 8102,
  weeklyLast: 8103,
  stage: 8201,
  meet: 8301,
} as const;

interface CalendarWorld {
  readonly events: Map<number, CalendarRecord>;
  nextId: number;
}

const VALIDATION = "Validation" satisfies ApiError["_tag"];

const REPEATS: EventRepeatOption[] = [
  { value: "daily", label: "Daily" },
  { value: "weekly", label: "Weekly" },
  { value: "biweekly", label: "Every two weeks" },
  { value: "monthly", label: "Monthly" },
];

function emptyValues(): EventValues {
  return {
    title: "",
    description: null,
    startsAt: null,
    endsAt: null,
    timeZone: "UTC",
    venueRoomId: null,
    recurrenceRule: null,
    recurrenceUntil: null,
    meetLinkRequested: false,
    meetLink: null,
  };
}

function seed(now: number): CalendarWorld {
  const events = new Map<number, CalendarRecord>();

  const put = (
    id: number,
    roomId: number,
    title: string,
    days: number,
    changes: Partial<EventValues> = {},
    seriesId: number | null = null,
  ) => {
    const startsAt = timestamp(now + days * DAY);
    const endsAt = timestamp(now + days * DAY + 3_600_000);

    events.set(id, {
      id,
      roomId,
      organizerId: VIEWER_ID,
      seriesId,
      values: {
        ...emptyValues(),
        title,
        description: "Bring your questions.\nWe will share updates.",
        startsAt: localTime(startsAt, changes.timeZone ?? "UTC"),
        endsAt: localTime(endsAt, changes.timeZone ?? "UTC"),
        ...changes,
      },
      startsAt,
      endsAt,
      cancelled: id === EVENT_IDS.cancelled,
      responses: new Map([[USER_IDS.maya, "going"]]),
      calendarCopies: new Set(),
    });
  };

  put(EVENT_IDS.upcoming, ROOM_IDS.general, "Team check-in", 1);
  put(EVENT_IDS.past, ROOM_IDS.general, "Last week's check-in", -7);
  put(EVENT_IDS.cancelled, ROOM_IDS.general, "Cancelled coffee chat", 2);

  for (let index = 0; index < 4; index += 1) {
    put(
      EVENT_IDS.weeklyHead + index,
      ROOM_IDS.engineering,
      "Engineering weekly",
      1 + index * 7,
      {
        recurrenceRule: "weekly",
        recurrenceUntil: timestamp(now + 22 * DAY).slice(0, 10),
        venueRoomId: ROOM_IDS.lounge,
      },
      EVENT_IDS.weeklyHead,
    );
  }

  put(EVENT_IDS.stage, ROOM_IDS.design, "Design town hall", 3, {
    venueRoomId: ROOM_IDS.townHall,
    timeZone: "America/New_York",
  });
  put(EVENT_IDS.meet, ROOM_IDS.launchPlanning, "Launch planning", 4, {
    meetLinkRequested: true,
    meetLink: "https://meet.google.com/abc-defg-hij",
  });

  return { events, nextId: 9000 };
}

export function createEvents(ctx: S2Context) {
  const worlds = new WeakMap<World, CalendarWorld>();

  const state = () => {
    const world = ctx.world();
    let current = worlds.get(world);

    if (current === undefined) {
      current = seed(ctx.now());
      worlds.set(world, current);
    }

    return current;
  };

  const room = (roomId: number) => {
    const record = ctx.roomOr404(roomId);
    const user = ctx.world().users.get(VIEWER_ID);

    if (user === undefined || user.role === "bot" || user.status !== "active") throw forbidden();

    return record;
  };

  const find = (roomId: number, eventId: number) => {
    room(roomId);

    const event = state().events.get(eventId);

    if (event === undefined || event.roomId !== roomId) throw notFound("Event not found");

    return event;
  };

  const siblings = (event: CalendarRecord) =>
    [...state().events.values()]
      .filter((candidate) => event.seriesId !== null && candidate.seriesId === event.seriesId)
      .sort((a, b) => a.startsAt.localeCompare(b.startsAt) || a.id - b.id);

  const future = (event: CalendarRecord) =>
    siblings(event).filter(
      (candidate) =>
        candidate.startsAt > event.startsAt ||
        (candidate.startsAt === event.startsAt && candidate.id > event.id),
    );

  const canManage = (event: CalendarRecord) =>
    event.organizerId === VIEWER_ID || ctx.world().users.get(VIEWER_ID)?.role === "administrator";

  const counts = (event: CalendarRecord) => {
    const result = { going: 0, maybe: 0, declined: 0 };

    for (const response of event.responses.values()) result[response] += 1;

    return result;
  };

  const row = (event: CalendarRecord, remaining: number | null = null): ChannelEvent => {
    const venue =
      event.values.venueRoomId === null
        ? undefined
        : ctx.world().rooms.get(event.values.venueRoomId);

    return {
      id: event.id,
      roomId: event.roomId,
      title: event.values.title,
      startsAt: event.startsAt,
      endsAt: event.endsAt,
      timeZone: event.values.timeZone,
      zoneLabel: zoneLabel(event.startsAt, event.endsAt, event.values.timeZone),
      organizerName: ctx.world().users.get(event.organizerId)?.name ?? "Someone",
      venue:
        venue === undefined
          ? null
          : {
              roomId: venue.room.id,
              name: ctx.displayName(venue),
              kind: venue.room.kind,
              member: venue.memberIds.includes(VIEWER_ID),
              liveUser: null,
            },
      counts: counts(event),
      recurrenceLabel:
        event.values.recurrenceRule === null
          ? null
          : `Repeats ${phrase(event.values.recurrenceRule)}`,
      remainingOccurrences: remaining,
      cancelled: event.cancelled,
      series: event.seriesId !== null,
    };
  };

  const detail = (event: CalendarRecord): EventDetail => {
    const all = siblings(event);

    const previous = all
      .filter(
        (candidate) =>
          candidate.startsAt < event.startsAt ||
          (candidate.startsAt === event.startsAt && candidate.id < event.id),
      )
      .at(-1);

    const next = future(event)[0];
    const attendees = [];

    for (const [id, response] of event.responses) {
      const user = ctx.world().users.get(id);

      if (user !== undefined) attendees.push({ name: user.name, response });
    }

    attendees.sort((a, b) => a.response.localeCompare(b.response));

    return {
      roomId: event.roomId,
      roomName: ctx.displayName(room(event.roomId)),
      event: row(event),
      descriptionHtml: descriptionHtml(event.values.description),
      manageable: !event.cancelled && canManage(event),
      respondable: !event.cancelled,
      currentResponse: event.responses.get(VIEWER_ID) ?? null,
      head: event.seriesId === event.id,
      canApplyToFuture:
        event.seriesId !== null && (event.seriesId === event.id || next !== undefined),
      previousOccurrenceId: previous?.id ?? null,
      nextOccurrenceId: next?.id ?? null,
      recurrencePhrase:
        event.values.recurrenceRule === null ? null : phrase(event.values.recurrenceRule),
      recurrenceUntil:
        event.values.recurrenceUntil === null ? null : displayDate(event.values.recurrenceUntil),
      meetLink: event.values.meetLink,
      calendarCopy: event.calendarCopies.has(VIEWER_ID),
      attendees,
    };
  };

  /**
   * A new form's values with a prefilled link's `title`, `startsAt` and `timeZone` (what the
   * `/event` command fills in), as the server reads them: the title trimmed to 255 characters,
   * an unknown zone left at UTC, and the start shown in the zone as a `datetime-local` value.
   */
  const prefilled = (query: URLSearchParams): EventValues => {
    const values = emptyValues();
    const zone = query.get("timeZone");
    const title = query.get("title")?.trim() ?? "";

    if (zone !== null && knownZone(zone)) values.timeZone = zone;

    if (title !== "") values.title = title.slice(0, 255);

    const start = instant(query.get("startsAt"), values.timeZone);

    if (start !== null) values.startsAt = localTime(start, values.timeZone);

    return values;
  };

  const form = (
    roomId: number,
    event: CalendarRecord | null,
    query = new URLSearchParams(),
  ): EventForm => {
    const record = room(roomId);
    const venues = [];

    for (const candidate of ctx.world().rooms.values()) {
      if (
        ["voice", "stage"].includes(candidate.room.kind) &&
        (candidate.memberIds.includes(VIEWER_ID) || candidate.room.id === event?.values.venueRoomId)
      )
        venues.push({
          roomId: candidate.room.id,
          name: ctx.displayName(candidate),
          kind: candidate.room.kind,
        });
    }

    venues.sort((a, b) => a.name.toLowerCase().localeCompare(b.name.toLowerCase()));

    return {
      roomId,
      roomName: ctx.displayName(record),
      eventId: event?.id ?? null,
      values: event === null ? prefilled(query) : { ...event.values },
      venues,
      meetAvailable: event?.values.meetLink === null || event === null,
      repeatOptions:
        event === null ? [{ value: null, label: "Does not repeat" }, ...REPEATS] : REPEATS,
      limits: { titleMaxLength: 255, maxOccurrences: 52, maxRecurrenceYears: 1 },
      series: event?.seriesId !== null && event !== null,
      head: event !== null && event.seriesId === event.id,
      ruleEditable: event === null || event.seriesId === event.id,
      scopeOptions:
        event !== null && event.seriesId !== null ? ["this_event", "this_and_following"] : [],
    };
  };

  const card = (event: CalendarRecord): EventCard => ({
    eventId: event.id,
    roomId: event.roomId,
    title: event.values.title,
    organizerId: event.organizerId,
    startsAt: event.startsAt,
    endsAt: event.endsAt,
    timeZone: event.values.timeZone,
    recurring: event.seriesId !== null,
    cancelled: event.cancelled,
    venueRoomId: event.values.venueRoomId,
    venueName: row(event).venue?.name ?? null,
    meetLink: event.values.meetLink,
  });

  const publishCards = (event: CalendarRecord) => {
    const record = room(event.roomId);

    for (let index = 0; index < record.messages.length; index += 1) {
      const message = record.messages[index];

      if (message === undefined) continue;

      if (!message.cards.some((entry) => entry.kind === "event" && entry.data.eventId === event.id))
        continue;

      const asOf = timestamp(Math.max(ctx.now(), Date.parse(message.cardsAsOf) + 1));

      const cards = message.cards.map((entry) =>
        entry.kind === "event" && entry.data.eventId === event.id
          ? ({ kind: "event", data: card(event) } as const)
          : entry,
      );

      record.messages[index] = { ...message, cards, cardsAsOf: asOf };
      ctx.publish([
        {
          topic: `room:${event.roomId}`,
          type: "message.cards",
          data: { messageId: message.id, roomId: event.roomId, threadId: null, cards, asOf },
        },
      ]);
    }
  };

  /**
   * The room's `events.changed`: every write sends it, so screens hear about events no message
   * links (a series' later occurrences) and ones a shortened series removed.
   */
  const publishChanged = (roomId: number) => {
    ctx.publish([{ topic: `room:${roomId}`, type: "events.changed", data: { roomId } }]);
  };

  const create = (roomId: number, body: Json | undefined) => {
    const record = room(roomId);
    const values = formValues(body, null);

    validateValues(values, body, ctx.world(), VIEWER_ID);

    const start = instant(values.startsAt, values.timeZone);
    const end = instant(values.endsAt, values.timeZone);

    if (start === null) throw validation("startsAt", "can't be blank");

    const firstId = state().nextId;
    const slots = values.recurrenceRule === null ? [start] : repeatSlots(values);
    const duration = end === null ? null : Date.parse(end) - Date.parse(start);
    const created: CalendarRecord[] = [];

    for (const startsAt of slots) {
      const endsAt = duration === null ? null : timestamp(Date.parse(startsAt) + duration);

      const event: CalendarRecord = {
        id: state().nextId++,
        roomId,
        organizerId: VIEWER_ID,
        seriesId: values.recurrenceRule === null ? null : firstId,
        values: {
          ...values,
          startsAt: localTime(startsAt, values.timeZone),
          endsAt: endsAt === null ? null : localTime(endsAt, values.timeZone),
        },
        startsAt,
        endsAt,
        cancelled: false,
        responses: new Map([[VIEWER_ID, "going"]]),
        calendarCopies: new Set(),
      };

      state().events.set(event.id, event);
      created.push(event);
    }

    const event = created[0];

    if (event === undefined) throw validation("recurrenceUntil", "can't be blank");

    const message = ctx.postToRoom(
      record,
      plainDraft(VIEWER_ID, `/rooms/${roomId}/events/${event.id}`, ctx.uuid()),
    );

    const index = record.messages.findIndex((held) => held.id === message.id);

    record.messages[index] = { ...message, cards: [{ kind: "event", data: card(event) }] };
    publishCards(event);
    publishChanged(roomId);

    return ok(detail(event), 201);
  };

  const update = (event: CalendarRecord, body: Json | undefined) => {
    if (event.cancelled || !canManage(event)) throw forbidden();

    const values = formValues(body, event.values);
    const following = stringField(body, "updateScope") === "this_and_following";

    const ruleChanged =
      values.recurrenceRule !== event.values.recurrenceRule ||
      values.recurrenceUntil !== event.values.recurrenceUntil;

    if (event.seriesId === event.id && values.recurrenceRule === null)
      throw validation("recurrenceRule", "can't be removed from a repeating event");

    if (ruleChanged && !(event.seriesId === event.id && following))
      throw validation(
        "recurrenceRule",
        event.seriesId === null
          ? "can only be set when scheduling a new event"
          : "can only be changed from the first event in the series using This and following",
      );

    validateValues(values, body, ctx.world(), event.organizerId, event);

    const start = instant(values.startsAt, values.timeZone);
    const end = instant(values.endsAt, values.timeZone);

    if (start === null) throw validation("startsAt", "can't be blank");

    if (event.seriesId === event.id && start !== event.startsAt && !following)
      throw validation(
        "startsAt",
        "moves the whole series: choose This and following or the entire series",
      );

    if (event.seriesId !== null && start !== event.startsAt) {
      const active = siblings(event).filter(
        (candidate) => !candidate.cancelled && candidate.id !== event.id,
      );

      const previous = active.filter((candidate) => candidate.startsAt < event.startsAt).at(-1);
      const next = active.find((candidate) => candidate.startsAt > event.startsAt);

      if (
        (previous !== undefined && start <= previous.startsAt) ||
        (!following && next !== undefined && start >= next.startsAt)
      )
        throw validation(
          "startsAt",
          "must stay between the neighbouring occurrences in its series",
        );
    }

    const targets = following ? [event, ...future(event)] : [event];
    const old = { ...event.values };
    const shift = Date.parse(start) - Date.parse(event.startsAt);
    const duration = end === null ? null : Date.parse(end) - Date.parse(start);

    const endShift =
      end !== null && event.endsAt !== null && end !== event.endsAt
        ? Date.parse(end) - Date.parse(event.endsAt)
        : shift;

    const endsAdded = end !== null && event.endsAt === null;
    const endsRemoved = end === null && event.endsAt !== null;

    const plans = targets.map((target) => {
      const startsAt = timestamp(Date.parse(target.startsAt) + shift);

      const endsAt =
        target.id === event.id
          ? end
          : endsRemoved
            ? null
            : endsAdded
              ? timestamp(Date.parse(startsAt) + (duration ?? 0))
              : target.endsAt === null
                ? null
                : timestamp(Date.parse(target.endsAt) + endShift);

      if (endsAt !== null && endsAt <= startsAt)
        throw validation("endsAt", "must be after the start time");

      const plannedValues =
        target.id === event.id
          ? { ...values }
          : {
              ...target.values,
              title: old.title === values.title ? target.values.title : values.title,
              description:
                old.description === values.description
                  ? target.values.description
                  : values.description,
              venueRoomId:
                old.venueRoomId === values.venueRoomId
                  ? target.values.venueRoomId
                  : values.venueRoomId,
              meetLinkRequested:
                old.meetLinkRequested === values.meetLinkRequested
                  ? target.values.meetLinkRequested
                  : values.meetLinkRequested,
              recurrenceRule: ruleChanged ? values.recurrenceRule : target.values.recurrenceRule,
              recurrenceUntil: ruleChanged ? values.recurrenceUntil : target.values.recurrenceUntil,
            };

      return {
        target,
        startsAt,
        endsAt,
        values: {
          ...plannedValues,
          startsAt: localTime(startsAt, values.timeZone),
          endsAt: endsAt === null ? null : localTime(endsAt, values.timeZone),
        },
      };
    });

    for (const plan of plans) {
      plan.target.startsAt = plan.startsAt;
      plan.target.endsAt = plan.endsAt;
      plan.target.values = plan.values;
    }

    if (ruleChanged) {
      const slots = [...repeatSlots(values).slice(1)];
      const protectedRows: CalendarRecord[] = [];
      const regenerableRows: CalendarRecord[] = [];

      const matchingResponses = (candidate: CalendarRecord) =>
        candidate.responses.size === event.responses.size &&
        [...candidate.responses].every(([id, response]) => event.responses.get(id) === response);

      const claim = (candidate: CalendarRecord) => {
        const index = slots.indexOf(candidate.startsAt);

        if (index < 0) return false;

        slots.splice(index, 1);

        return true;
      };

      for (const follower of targets.slice(1)) {
        if (follower.cancelled) claim(follower);
        else if (matchingResponses(follower)) regenerableRows.push(follower);
        else protectedRows.push(follower);
      }

      const place = (candidate: CalendarRecord, protectedResponse: boolean) => {
        const slot = slots.shift();

        if (slot === undefined) {
          if (protectedResponse) candidate.cancelled = true;
          else state().events.delete(candidate.id);

          publishCards(candidate);

          return;
        }

        candidate.startsAt = slot;
        candidate.endsAt = duration === null ? null : timestamp(Date.parse(slot) + duration);
        candidate.values = {
          ...candidate.values,
          startsAt: localTime(slot, values.timeZone),
          endsAt: candidate.endsAt === null ? null : localTime(candidate.endsAt, values.timeZone),
        };
      };

      const unmatchedProtected = protectedRows.filter((candidate) => !claim(candidate));

      for (const follower of unmatchedProtected) place(follower, true);

      const unmatchedRegenerable = regenerableRows.filter((candidate) => !claim(candidate));

      for (const follower of unmatchedRegenerable) place(follower, false);

      for (const slot of slots) {
        const id = state().nextId++;
        const endsAt = duration === null ? null : timestamp(Date.parse(slot) + duration);

        state().events.set(id, {
          ...event,
          id,
          values: {
            ...values,
            startsAt: localTime(slot, values.timeZone),
            endsAt: endsAt === null ? null : localTime(endsAt, values.timeZone),
          },
          startsAt: slot,
          endsAt,
          responses: new Map(event.responses),
          calendarCopies: new Set(),
        });
      }
    }

    for (const target of targets) publishCards(target);

    publishChanged(event.roomId);

    return ok(detail(event));
  };

  const cancel = (event: CalendarRecord, body: Json | undefined) => {
    if (!canManage(event)) throw forbidden();

    if (event.cancelled) return ok(detail(event));

    const targets =
      stringField(body, "cancelScope") === "this_and_following"
        ? [event, ...future(event)]
        : [event];

    for (const target of targets) {
      target.cancelled = true;
      publishCards(target);
    }

    publishChanged(event.roomId);

    return ok(detail(event));
  };

  const list = (roomId: number): EventList => {
    const record = room(roomId);

    const events = [...state().events.values()]
      .filter((event) => event.roomId === roomId)
      .sort((a, b) => a.startsAt.localeCompare(b.startsAt) || a.id - b.id);

    const upcoming = events.filter(
      (event) => !event.cancelled && Date.parse(event.endsAt ?? event.startsAt) >= ctx.now(),
    );

    const seen = new Set<number>();
    const rows = [];

    for (const event of upcoming) {
      if (event.seriesId !== null && seen.has(event.seriesId)) continue;

      if (event.seriesId !== null) seen.add(event.seriesId);

      rows.push(
        row(
          event,
          event.seriesId === null
            ? null
            : upcoming.filter((candidate) => candidate.seriesId === event.seriesId).length,
        ),
      );
    }

    return {
      roomId,
      roomName: ctx.displayName(record),
      roomKind: record.room.kind,
      mayCreate: true,
      upcoming: rows,
      past: events
        .filter(
          (event) => !event.cancelled && Date.parse(event.endsAt ?? event.startsAt) < ctx.now(),
        )
        .reverse()
        .map((event) => row(event)),
      cancelled: events
        .filter((event) => event.cancelled)
        .reverse()
        .map((event) => row(event)),
    };
  };

  const attendance = (event: CalendarRecord): EventAttendance => {
    const current = detail(event);

    return {
      eventId: event.id,
      response: current.currentResponse,
      goingCount: current.event.counts.going,
      maybeCount: current.event.counts.maybe,
      declinedCount: current.event.counts.declined,
      respondable: current.respondable,
      canApplyToFuture: current.canApplyToFuture,
    };
  };

  return {
    routes: [
      route("GET", /^\/rooms\/(\d+)\/events$/, (request) => ok(list(firstId(request)))),
      route("GET", /^\/rooms\/(\d+)\/events\/new$/, (request) =>
        ok(form(firstId(request), null, request.query)),
      ),
      route("POST", /^\/rooms\/(\d+)\/events$/, (request) =>
        create(firstId(request), request.body),
      ),
      route("GET", /^\/rooms\/(\d+)\/events\/(\d+)$/, ({ ids: [roomId = 0, eventId = 0] }) =>
        ok(detail(find(roomId, eventId))),
      ),
      route(
        "GET",
        /^\/rooms\/(\d+)\/events\/(\d+)\/edit$/,
        ({ ids: [roomId = 0, eventId = 0] }) => {
          const event = find(roomId, eventId);

          if (event.cancelled || !canManage(event)) throw forbidden();

          return ok(form(roomId, event));
        },
      ),
      route(
        "PATCH",
        /^\/rooms\/(\d+)\/events\/(\d+)$/,
        ({ ids: [roomId = 0, eventId = 0], body }) => update(find(roomId, eventId), body),
      ),
      route(
        "PATCH",
        /^\/rooms\/(\d+)\/events\/(\d+)\/cancel$/,
        ({ ids: [roomId = 0, eventId = 0], body }) => cancel(find(roomId, eventId), body),
      ),
    ],
    readAttendance(roomId: number, eventId: number) {
      if (!state().events.has(eventId)) return null;

      return ok(attendance(find(roomId, eventId)));
    },
    respond(roomId: number, eventId: number, body: Json | undefined) {
      if (!state().events.has(eventId)) return null;

      const event = find(roomId, eventId);
      const response = stringField(body, "response");

      if (response !== "going" && response !== "maybe" && response !== "declined")
        throw new HttpError(422, {
          _tag: VALIDATION,
          message: "Choose going, maybe, or declined.",
          fields: { response: ["Choose going, maybe, or declined."] },
        });

      if (event.cancelled)
        throw new HttpError(422, {
          _tag: VALIDATION,
          message: "This event is no longer open for responses.",
          fields: { response: ["This event is no longer open for responses."] },
        });

      const targets =
        event.seriesId === event.id || booleanField(body, "applyToFuture") === true
          ? [event, ...future(event)]
          : [event];

      for (const target of targets) {
        if (!target.cancelled) target.responses.set(VIEWER_ID, response);
      }

      return ok(attendance(event));
    },
  };
}
