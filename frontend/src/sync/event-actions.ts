/** Calendar screens own their loaded data; these actions return the server's current facts. */
import { Effect } from "effect";
import { eventAttendance } from "../api/cards-endpoints.ts";
import * as api from "../api/event-endpoints.ts";
import type { AttendanceResponse } from "../gen/AttendanceResponse.ts";
import type { CancelEvent } from "../gen/CancelEvent.ts";
import type { CreateEvent } from "../gen/CreateEvent.ts";
import type { SyncEvent } from "../gen/SyncEvent.ts";
import type { UpdateEvent } from "../gen/UpdateEvent.ts";
import * as cards from "./card-actions.ts";
import { Topics } from "./topics.ts";

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

/**
 * Whether a sync event says an event in `roomId` changed elsewhere: the room's `events.changed`
 * (any event scheduled, edited, cancelled or removed, linked from a message or not), or a message
 * linking one was posted (a new event's announcement) or had its cards replaced (an edit, a
 * cancel, a reminder). The server sends none when someone answers.
 */
export function changesRoomEvents(event: SyncEvent, roomId: number): boolean {
  switch (event.type) {
    case "events.changed":
      return event.data.roomId === roomId;
    case "message.created":
    case "message.cards":
      return event.data.cards.some((card) => card.kind === "event" && card.data.roomId === roomId);
    default:
      return false;
  }
}

/** Keeps a room's topic subscribed while a calendar screen shows it: its card changes come there. */
export const holdRoom = (roomId: number) =>
  Topics.use((topics) => topics.acquire(`room:${roomId}`));

export const releaseRoom = (roomId: number) =>
  Topics.use((topics) => topics.release(`room:${roomId}`));
