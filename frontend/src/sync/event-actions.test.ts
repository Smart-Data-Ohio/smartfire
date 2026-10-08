import { describe, expect, it } from "vitest";
import { messageFixture } from "../api/testing.ts";
import type { EventCard } from "../gen/EventCard.ts";
import type { MessageCard } from "../gen/MessageCard.ts";
import type { SyncEvent } from "../gen/SyncEvent.ts";
import { changesRoomEvents } from "./event-actions.ts";

const ROOM = 4;

const eventCard = (roomId: number): MessageCard => {
  const data: EventCard = {
    eventId: 8001,
    roomId,
    title: "Team check-in",
    organizerId: 7,
    startsAt: "2026-10-08T15:00:00.000Z",
    endsAt: null,
    timeZone: "UTC",
    recurring: false,
    cancelled: true,
    venueRoomId: null,
    venueName: null,
    meetLink: null,
  };

  return { kind: "event", data };
};

const cardsChanged = (cards: MessageCard[]): SyncEvent => ({
  type: "message.cards",
  seq: 2,
  topic: `room:${ROOM}`,
  data: { messageId: 11, roomId: ROOM, threadId: null, cards, asOf: "2026-10-07T12:00:00.000Z" },
});

describe("changesRoomEvents", () => {
  it("hears an edit or cancel (its cards replaced) and a new event's announcement", () => {
    expect(changesRoomEvents(cardsChanged([eventCard(ROOM)]), ROOM)).toBe(true);

    const announced: SyncEvent = {
      type: "message.created",
      seq: 3,
      topic: `room:${ROOM}`,
      data: messageFixture(12, ROOM, { cards: [eventCard(ROOM)] }),
    };

    expect(changesRoomEvents(announced, ROOM)).toBe(true);
  });

  it("hears the room's events changing, linked from a message or not", () => {
    const changed = (roomId: number): SyncEvent => ({
      type: "events.changed",
      seq: 5,
      topic: `room:${roomId}`,
      data: { roomId },
    });

    expect(changesRoomEvents(changed(ROOM), ROOM)).toBe(true);
    expect(changesRoomEvents(changed(ROOM + 1), ROOM)).toBe(false);
  });

  it("ignores other rooms' events, other cards and other news", () => {
    expect(changesRoomEvents(cardsChanged([eventCard(ROOM + 1)]), ROOM)).toBe(false);
    expect(changesRoomEvents(cardsChanged([]), ROOM)).toBe(false);

    const typing: SyncEvent = {
      type: "typing",
      seq: 4,
      topic: `room:${ROOM}`,
      data: { userId: 7, on: true },
    };

    expect(changesRoomEvents(typing, ROOM)).toBe(false);
  });
});
