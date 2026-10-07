/** Channel calendar reads and writes. Times in form bodies use the event's scheduled zone. */
import { Effect } from "effect";
import type { CancelEvent } from "../gen/CancelEvent.ts";
import type { CreateEvent } from "../gen/CreateEvent.ts";
import type { EventDetail } from "../gen/EventDetail.ts";
import type { EventForm } from "../gen/EventForm.ts";
import type { EventList } from "../gen/EventList.ts";
import type { UpdateEvent } from "../gen/UpdateEvent.ts";
import { call, get } from "./call.ts";
import {
  EventDetail as EventDetailSchema,
  EventForm as EventFormSchema,
  EventList as EventListSchema,
} from "./schema/events.ts";
import { wire } from "./wire.ts";

export const events = Effect.fn("api.events")(function* (roomId: number) {
  return yield* call(get(`/rooms/${roomId}/events`), wire<EventList>(EventListSchema));
});

export const newEvent = Effect.fn("api.newEvent")(function* (roomId: number) {
  return yield* call(get(`/rooms/${roomId}/events/new`), wire<EventForm>(EventFormSchema));
});

export const event = Effect.fn("api.event")(function* (roomId: number, eventId: number) {
  return yield* call(
    get(`/rooms/${roomId}/events/${eventId}`),
    wire<EventDetail>(EventDetailSchema),
  );
});

export const editEvent = Effect.fn("api.editEvent")(function* (roomId: number, eventId: number) {
  return yield* call(
    get(`/rooms/${roomId}/events/${eventId}/edit`),
    wire<EventForm>(EventFormSchema),
  );
});

export const createEvent = Effect.fn("api.createEvent")(function* (
  roomId: number,
  body: CreateEvent,
) {
  return yield* call(
    { method: "POST", path: `/rooms/${roomId}/events`, body },
    wire<EventDetail>(EventDetailSchema),
  );
});

export const updateEvent = Effect.fn("api.updateEvent")(function* (
  roomId: number,
  eventId: number,
  body: UpdateEvent,
) {
  return yield* call(
    { method: "PATCH", path: `/rooms/${roomId}/events/${eventId}`, body },
    wire<EventDetail>(EventDetailSchema),
  );
});

export const cancelEvent = Effect.fn("api.cancelEvent")(function* (
  roomId: number,
  eventId: number,
  body: CancelEvent,
) {
  return yield* call(
    { method: "PATCH", path: `/rooms/${roomId}/events/${eventId}/cancel`, body },
    wire<EventDetail>(EventDetailSchema),
  );
});
