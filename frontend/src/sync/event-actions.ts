/** Calendar screens own their loaded data; these actions return the server's current facts. */
import { Effect } from "effect";
import { eventAttendance } from "../api/cards-endpoints.ts";
import * as api from "../api/event-endpoints.ts";
import type { AttendanceResponse } from "../gen/AttendanceResponse.ts";
import type { CancelEvent } from "../gen/CancelEvent.ts";
import type { CreateEvent } from "../gen/CreateEvent.ts";
import type { UpdateEvent } from "../gen/UpdateEvent.ts";
import * as cards from "./card-actions.ts";

export const list = api.events;

export const newForm = api.newEvent;

export const read = api.event;

export const editForm = api.editEvent;

export const create = (roomId: number, body: CreateEvent) => api.createEvent(roomId, body);

export const update = (roomId: number, eventId: number, body: UpdateEvent) =>
  api.updateEvent(roomId, eventId, body);

export const cancel = (roomId: number, eventId: number, body: CancelEvent) =>
  api.cancelEvent(roomId, eventId, body);

export const attendance = eventAttendance;

/** Reuse the card action's per-event queue and store update, then read the show page's facts. */
export const respond = Effect.fn("events.respond")(function* (
  roomId: number,
  eventId: number,
  response: AttendanceResponse,
  applyToFuture: boolean,
) {
  yield* cards.respond(roomId, eventId, response, applyToFuture);

  return yield* api.event(roomId, eventId);
});
