import { describe, expect, it } from "@effect/vitest";
import { Schema } from "effect";
import { get, harness } from "../../../mock/s2/testing.ts";
import { EVENT_IDS } from "../../../mock/s8/events.ts";
import { ROOM_IDS } from "../../../mock/seed.ts";
import type { EventDetail as GeneratedEventDetail } from "../../gen/EventDetail.ts";
import type { EventForm as GeneratedEventForm } from "../../gen/EventForm.ts";
import type { EventList as GeneratedEventList } from "../../gen/EventList.ts";
import {
  CancelEvent,
  CreateEvent,
  EventDetail,
  EventForm,
  EventList,
  EventRecurrenceRule,
  EventScope,
  UpdateEvent,
} from "./events.ts";

const roundTrips = <S extends Schema.Codec<unknown, unknown>>(schema: S, wire: S["Encoded"]) =>
  expect(Schema.encodeSync(schema)(Schema.decodeUnknownSync(schema)(wire))).toEqual(wire);

describe("S8 event schemas", () => {
  it("round-trip list groups and details including venues, Meet, recurrence and attendance", async () => {
    const { server } = harness();

    for (const roomId of [
      ROOM_IDS.general,
      ROOM_IDS.engineering,
      ROOM_IDS.design,
      ROOM_IDS.launchPlanning,
      ROOM_IDS.quiet,
    ]) {
      roundTrips(
        EventList,
        await get<GeneratedEventList>(server, `/api/v1/rooms/${roomId}/events`),
      );
    }

    for (const [roomId, eventId] of [
      [ROOM_IDS.general, EVENT_IDS.upcoming],
      [ROOM_IDS.general, EVENT_IDS.past],
      [ROOM_IDS.general, EVENT_IDS.cancelled],
      [ROOM_IDS.engineering, EVENT_IDS.weeklyHead],
      [ROOM_IDS.engineering, EVENT_IDS.weeklyLast],
      [ROOM_IDS.design, EVENT_IDS.stage],
      [ROOM_IDS.launchPlanning, EVENT_IDS.meet],
    ]) {
      roundTrips(
        EventDetail,
        await get<GeneratedEventDetail>(server, `/api/v1/rooms/${roomId}/events/${eventId}`),
      );
    }
  });

  it("round-trip new, head, follower and Meet form metadata and local clock values", async () => {
    const { server } = harness();

    roundTrips(
      EventForm,
      await get<GeneratedEventForm>(server, `/api/v1/rooms/${ROOM_IDS.general}/events/new`),
    );

    for (const [roomId, eventId] of [
      [ROOM_IDS.engineering, EVENT_IDS.weeklyHead],
      [ROOM_IDS.engineering, EVENT_IDS.weeklySecond],
      [ROOM_IDS.design, EVENT_IDS.stage],
      [ROOM_IDS.launchPlanning, EVENT_IDS.meet],
    ]) {
      roundTrips(
        EventForm,
        await get<GeneratedEventForm>(server, `/api/v1/rooms/${roomId}/events/${eventId}/edit`),
      );
    }
  });

  it("round-trip create and update controls, preserving omitted keys separately from null", () => {
    const body = {
      title: "Planning",
      description: null,
      startsAt: "2026-10-08T10:00",
      endsAt: null,
      timeZone: "America/New_York",
      venueRoomId: null,
      recurrenceRule: "weekly",
      recurrenceUntil: "2026-11-08",
      meetLinkRequested: false,
    };

    roundTrips(CreateEvent, body);
    roundTrips(CreateEvent, { ...body, startsAt: null, timeZone: null, recurrenceRule: "unknown" });
    roundTrips(UpdateEvent, {});
    roundTrips(UpdateEvent, {
      description: null,
      startsAt: null,
      endsAt: null,
      venueRoomId: null,
      recurrenceRule: null,
      recurrenceUntil: null,
    });
    roundTrips(UpdateEvent, { ...body, updateScope: "this_and_following" });
    roundTrips(CancelEvent, { cancelScope: null });
    roundTrips(CancelEvent, { cancelScope: "this_event" });
    roundTrips(CancelEvent, { cancelScope: "unknown" });
  });

  it("pins every recurrence and scope and rejects invalid response data", () => {
    for (const rule of ["daily", "weekly", "biweekly", "monthly"] as const)
      roundTrips(EventRecurrenceRule, rule);

    for (const scope of ["this_event", "this_and_following"] as const)
      roundTrips(EventScope, scope);

    expect(() => Schema.decodeUnknownSync(EventRecurrenceRule)("yearly")).toThrow();
    expect(() => Schema.decodeUnknownSync(EventScope)("all")).toThrow();
    expect(() => Schema.decodeUnknownSync(EventList)({ roomId: "1" })).toThrow();
  });
});
