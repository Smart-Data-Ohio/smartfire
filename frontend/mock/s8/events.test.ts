import { describe, expect, it } from "vitest";
import type { ApiError } from "../../src/gen/ApiError.ts";
import type { CreateEvent } from "../../src/gen/CreateEvent.ts";
import type { EventAttendance } from "../../src/gen/EventAttendance.ts";
import type { EventDetail } from "../../src/gen/EventDetail.ts";
import type { EventForm } from "../../src/gen/EventForm.ts";
import type { EventList } from "../../src/gen/EventList.ts";
import type { MessagePage } from "../../src/gen/MessagePage.ts";
import type { UpdateEvent } from "../../src/gen/UpdateEvent.ts";
import { field } from "../json.ts";
import { collect, expectStatus, get, harness, send } from "../s2/testing.ts";
import { CARD_IDS } from "../s3/cards.ts";
import { ROOM_IDS } from "../seed.ts";
import { EVENT_IDS } from "./events.ts";

const base = (roomId: number = ROOM_IDS.general) => `/api/v1/rooms/${roomId}/events`;

const VALIDATION = "Validation" satisfies ApiError["_tag"];

const createBody = (change: Partial<CreateEvent> = {}): CreateEvent => ({
  title: "Planning",
  description: "Bring notes.\nQuestions welcome.",
  startsAt: "2026-10-08T10:00",
  endsAt: "2026-10-08T11:00",
  timeZone: "America/New_York",
  venueRoomId: null,
  recurrenceRule: null,
  recurrenceUntil: null,
  meetLinkRequested: false,
  ...change,
});

const updateBody = (form: EventForm, change: UpdateEvent = {}): UpdateEvent => ({
  title: form.values.title,
  description: form.values.description,
  startsAt: form.values.startsAt,
  endsAt: form.values.endsAt,
  venueRoomId: form.values.venueRoomId,
  meetLinkRequested: form.values.meetLinkRequested,
  ...change,
});

describe("S8 channel events mock", () => {
  it("lists upcoming, past and cancelled events and groups the weekly series", async () => {
    const { server } = harness();
    const list = await get<EventList>(server, base());

    expect(list.roomName).toBe("general");
    expect(list.roomKind).toBe("open");
    expect(list.mayCreate).toBe(true);
    expect(list.upcoming.map((event) => event.id)).toEqual([EVENT_IDS.upcoming]);
    expect(list.past.map((event) => event.id)).toEqual([EVENT_IDS.past]);
    expect(list.cancelled.map((event) => event.id)).toEqual([EVENT_IDS.cancelled]);

    const weekly = await get<EventList>(server, base(ROOM_IDS.engineering));

    expect(weekly.upcoming.map((event) => event.id)).toEqual([EVENT_IDS.weeklyHead]);
    expect(weekly.upcoming[0]?.remainingOccurrences).toBe(4);
    expect(weekly.upcoming[0]?.recurrenceLabel).toBe("Repeats weekly");
  });

  it("ships form defaults, a stage venue, Meet link and show-page attendance", async () => {
    const { server } = harness();
    const form = await get<EventForm>(server, `${base()}/new`);

    expect(form.values.timeZone).toBe("UTC");
    expect(form.values.startsAt).toBeNull();
    expect(form.meetAvailable).toBe(true);
    expect(form.repeatOptions.map((option) => option.value)).toEqual([
      null,
      "daily",
      "weekly",
      "biweekly",
      "monthly",
    ]);
    expect(form.limits).toEqual({ titleMaxLength: 255, maxOccurrences: 52, maxRecurrenceYears: 1 });
    expect(form.venues.map((venue) => venue.roomId)).toEqual([ROOM_IDS.lounge, ROOM_IDS.townHall]);

    const stage = await get<EventDetail>(server, `${base(ROOM_IDS.design)}/${EVENT_IDS.stage}`);

    expect(stage.event.venue).toMatchObject({
      roomId: ROOM_IDS.townHall,
      kind: "stage",
      member: true,
    });
    expect(stage.event.timeZone).toBe("America/New_York");
    expect(stage.descriptionHtml).toBe(
      "<p>Bring your questions.\n<br />We will share updates.</p>",
    );
    expect(stage.event.counts).toEqual({ going: 1, maybe: 0, declined: 0 });

    const meet = await get<EventDetail>(
      server,
      `${base(ROOM_IDS.launchPlanning)}/${EVENT_IDS.meet}`,
    );

    expect(meet.meetLink).toBe("https://meet.google.com/abc-defg-hij");
    expect(meet.attendees).toEqual([{ name: "Maya Okafor", response: "going" }]);
  });

  it("prefills a new form's title and start, shown in the zone asked for", async () => {
    const { server } = harness();

    const query = new URLSearchParams({
      title: "  Launch party  ",
      startsAt: "2030-03-08T22:00:00Z",
      timeZone: "Europe/Berlin",
    });

    const form = await get<EventForm>(server, `${base()}/new?${query}`);

    expect(form.values).toMatchObject({
      title: "Launch party",
      startsAt: "2030-03-08T23:00",
      timeZone: "Europe/Berlin",
    });

    const unknown = await get<EventForm>(
      server,
      `${base()}/new?${new URLSearchParams({ startsAt: "2030-03-08T22:00:00Z", timeZone: "Mars" })}`,
    );

    expect(unknown.values).toMatchObject({
      title: "",
      startsAt: "2030-03-08T22:00",
      timeZone: "UTC",
    });
  });

  it("creates, edits and cancels an event, posting and refreshing its message card", async () => {
    const { server } = harness();
    const frames = collect(server, [`room:${ROOM_IDS.general}`]);
    const created = await expectStatus<EventDetail>(server, "POST", base(), createBody(), 201);

    expect(created.event.startsAt).toBe("2026-10-08T14:00:00.000Z");
    expect(created.currentResponse).toBe("going");
    expect(created.event.counts.going).toBe(1);

    const form = await get<EventForm>(server, `${base()}/${created.event.id}/edit`);

    expect(form.values.startsAt).toBe("2026-10-08T10:00");

    const edited = await expectStatus<EventDetail>(
      server,
      "PATCH",
      `${base()}/${created.event.id}`,
      updateBody(form, { title: "Updated planning", timeZone: "Asia/Tokyo" }),
      200,
    );

    expect(edited.event.timeZone).toBe("America/New_York");
    expect(edited.event.startsAt).toBe(created.event.startsAt);

    const cancelled = await expectStatus<EventDetail>(
      server,
      "PATCH",
      `${base()}/${created.event.id}/cancel`,
      { cancelScope: "this_event" },
      200,
    );

    expect(cancelled.event.cancelled).toBe(true);
    expect(cancelled.respondable).toBe(false);
    expect(frames.some((frame) => frame.type === "message.created")).toBe(true);
    expect(frames.filter((frame) => frame.type === "message.cards").length).toBe(3);

    const messages = await get<MessagePage>(server, `/api/v1/rooms/${ROOM_IDS.general}/messages`);

    const posted = messages.messages.find((message) =>
      message.cards.some((card) => card.kind === "event" && card.data.eventId === created.event.id),
    );

    expect(posted?.cards).toEqual([
      {
        kind: "event",
        data: {
          eventId: created.event.id,
          roomId: ROOM_IDS.general,
          title: "Updated planning",
          organizerId: 1,
          startsAt: "2026-10-08T14:00:00.000Z",
          endsAt: "2026-10-08T15:00:00.000Z",
          timeZone: "America/New_York",
          recurring: false,
          cancelled: true,
          venueRoomId: null,
          venueName: null,
          meetLink: null,
        },
      },
    ]);
  });

  it("tells the room about every change, to occurrences no message links and removed ones", async () => {
    const { server } = harness();
    const path = base(ROOM_IDS.engineering);
    const frames = collect(server, [`room:${ROOM_IDS.engineering}`]);

    const told = () =>
      frames.filter(
        (frame) => frame.type === "events.changed" && frame.data.roomId === ROOM_IDS.engineering,
      ).length;

    const third = await get<EventForm>(server, `${path}/${EVENT_IDS.weeklyThird}/edit`);

    await expectStatus<EventDetail>(
      server,
      "PATCH",
      `${path}/${EVENT_IDS.weeklyThird}`,
      updateBody(third, { title: "Engineering weekly (moved)", updateScope: "this_event" }),
      200,
    );
    expect(told()).toBe(1);

    await expectStatus<EventDetail>(
      server,
      "PATCH",
      `${path}/${EVENT_IDS.weeklySecond}/cancel`,
      { cancelScope: "this_event" },
      200,
    );
    expect(told()).toBe(2);

    // A week earlier, the series has no slot left for its last occurrence.
    const head = await get<EventForm>(server, `${path}/${EVENT_IDS.weeklyHead}/edit`);
    const until = head.values.recurrenceUntil ?? "";
    const earlier = new Date(Date.parse(`${until}T00:00:00Z`) - 7 * 86_400_000);

    await expectStatus<EventDetail>(
      server,
      "PATCH",
      `${path}/${EVENT_IDS.weeklyHead}`,
      updateBody(head, {
        recurrenceRule: head.values.recurrenceRule,
        recurrenceUntil: earlier.toISOString().slice(0, 10),
        updateScope: "this_and_following",
      }),
      200,
    );
    expect(told()).toBe(3);
    expect(
      (await server.handle({ method: "GET", path: `${path}/${EVENT_IDS.weeklyLast}` })).status,
    ).toBe(404);

    await expectStatus<EventDetail>(server, "POST", path, createBody(), 201);
    expect(told()).toBe(4);
  });

  it("responds to heads automatically and followers with the future checkbox", async () => {
    const { server } = harness();
    const path = base(ROOM_IDS.engineering);

    await expectStatus<EventAttendance>(
      server,
      "PUT",
      `${path}/${EVENT_IDS.weeklyHead}/attendance`,
      { response: "maybe", applyToFuture: false },
      200,
    );

    const third = await get<EventDetail>(server, `${path}/${EVENT_IDS.weeklyThird}`);

    expect(third.currentResponse).toBe("maybe");

    await expectStatus<EventAttendance>(
      server,
      "PUT",
      `${path}/${EVENT_IDS.weeklySecond}/attendance`,
      { response: "declined", applyToFuture: true },
      200,
    );

    expect(
      (await get<EventDetail>(server, `${path}/${EVENT_IDS.weeklyHead}`)).currentResponse,
    ).toBe("maybe");
    expect(
      (await get<EventDetail>(server, `${path}/${EVENT_IDS.weeklyLast}`)).currentResponse,
    ).toBe("declined");
    expect(
      (
        await get<EventAttendance>(
          server,
          `/api/v1/rooms/${CARD_IDS.room}/events/${CARD_IDS.events.recurring}/attendance`,
        )
      ).eventId,
    ).toBe(CARD_IDS.events.recurring);
  });

  it("keeps follower overrides and duration when unchanged controls are applied to the series", async () => {
    const { server } = harness();
    const path = base(ROOM_IDS.engineering);
    const follower = await get<EventForm>(server, `${path}/${EVENT_IDS.weeklySecond}/edit`);

    expect(follower.ruleEditable).toBe(false);

    await expectStatus<EventDetail>(
      server,
      "PATCH",
      `${path}/${EVENT_IDS.weeklySecond}`,
      updateBody(follower, {
        title: "Local title",
        description: "Local notes",
        endsAt: "2026-10-14T18:30",
      }),
      200,
    );

    const head = await get<EventForm>(server, `${path}/${EVENT_IDS.weeklyHead}/edit`);

    await expectStatus<EventDetail>(
      server,
      "PATCH",
      `${path}/${EVENT_IDS.weeklyHead}`,
      updateBody(head, { updateScope: "this_and_following" }),
      200,
    );

    const preserved = await get<EventDetail>(server, `${path}/${EVENT_IDS.weeklySecond}`);

    expect(preserved.event.title).toBe("Local title");
    expect(preserved.descriptionHtml).toBe("<p>Local notes</p>");
    expect(preserved.event.endsAt).toBe("2026-10-14T18:30:00.000Z");
  });

  it("rejects an invalid follower duration atomically before changing any occurrence", async () => {
    const { server } = harness();
    const path = base(ROOM_IDS.engineering);
    const follower = await get<EventForm>(server, `${path}/${EVENT_IDS.weeklySecond}/edit`);

    await expectStatus<EventDetail>(
      server,
      "PATCH",
      `${path}/${EVENT_IDS.weeklySecond}`,
      updateBody(follower, { endsAt: "2026-10-14T16:45" }),
      200,
    );

    const originalHead = await get<EventDetail>(server, `${path}/${EVENT_IDS.weeklyHead}`);
    const originalSecond = await get<EventDetail>(server, `${path}/${EVENT_IDS.weeklySecond}`);
    const originalThird = await get<EventDetail>(server, `${path}/${EVENT_IDS.weeklyThird}`);
    const head = await get<EventForm>(server, `${path}/${EVENT_IDS.weeklyHead}/edit`);

    const response = await send(
      server,
      "PATCH",
      `${path}/${EVENT_IDS.weeklyHead}`,
      updateBody(head, {
        endsAt: "2026-10-07T17:00",
        title: "Would change",
        updateScope: "this_and_following",
      }),
    );

    expect(response.status).toBe(422);
    expect(field(field(response.json, "error"), "fields")).toEqual({
      endsAt: ["must be after the start time"],
    });
    expect(await get<EventDetail>(server, `${path}/${EVENT_IDS.weeklyHead}`)).toEqual(originalHead);
    expect(await get<EventDetail>(server, `${path}/${EVENT_IDS.weeklySecond}`)).toEqual(
      originalSecond,
    );
    expect(await get<EventDetail>(server, `${path}/${EVENT_IDS.weeklyThird}`)).toEqual(
      originalThird,
    );
  });

  it("rebuilds repeats while preserving a follower's differing response and id", async () => {
    const { server } = harness();
    const path = base(ROOM_IDS.engineering);

    await expectStatus<EventAttendance>(
      server,
      "PUT",
      `${path}/${EVENT_IDS.weeklySecond}/attendance`,
      { response: "declined", applyToFuture: false },
      200,
    );

    const head = await get<EventForm>(server, `${path}/${EVENT_IDS.weeklyHead}/edit`);

    await expectStatus<EventDetail>(
      server,
      "PATCH",
      `${path}/${EVENT_IDS.weeklyHead}`,
      updateBody(head, {
        updateScope: "this_and_following",
        recurrenceRule: "biweekly",
        recurrenceUntil: "2026-10-28",
      }),
      200,
    );

    const protectedEvent = await get<EventDetail>(server, `${path}/${EVENT_IDS.weeklySecond}`);

    expect(protectedEvent.currentResponse).toBe("declined");
    expect(protectedEvent.event.startsAt).toBe("2026-10-21T16:30:00.000Z");
    expect((await get<EventList>(server, path)).upcoming[0]?.remainingOccurrences).toBe(2);
  });

  it("keeps cancelled neighbours and makes repeat cancellation of the selected row a no-op", async () => {
    const { server } = harness();
    const path = base(ROOM_IDS.engineering);

    await expectStatus<EventDetail>(
      server,
      "PATCH",
      `${path}/${EVENT_IDS.weeklyHead}/cancel`,
      { cancelScope: "this_event" },
      200,
    );
    await expectStatus<EventDetail>(
      server,
      "PATCH",
      `${path}/${EVENT_IDS.weeklyHead}/cancel`,
      { cancelScope: "this_and_following" },
      200,
    );

    const second = await get<EventDetail>(server, `${path}/${EVENT_IDS.weeklySecond}`);

    expect(second.previousOccurrenceId).toBe(EVENT_IDS.weeklyHead);
    expect(second.event.cancelled).toBe(false);

    await expectStatus<EventDetail>(
      server,
      "PATCH",
      `${path}/${EVENT_IDS.weeklyLast}/cancel`,
      {},
      200,
    );

    const third = await get<EventDetail>(server, `${path}/${EVENT_IDS.weeklyThird}`);

    expect(third.nextOccurrenceId).toBe(EVENT_IDS.weeklyLast);
    expect(third.canApplyToFuture).toBe(true);
  });

  it("allows a last follower to move after the unchanged series until date", async () => {
    const { server } = harness();
    const path = base(ROOM_IDS.engineering);
    const form = await get<EventForm>(server, `${path}/${EVENT_IDS.weeklyLast}/edit`);

    const moved = await expectStatus<EventDetail>(
      server,
      "PATCH",
      `${path}/${EVENT_IDS.weeklyLast}`,
      updateBody(form, { startsAt: "2026-10-30T16:30", endsAt: "2026-10-30T17:30" }),
      200,
    );

    expect(moved.event.startsAt).toBe("2026-10-30T16:30:00.000Z");
    expect(moved.recurrenceUntil).toBe("October 28, 2026");
  });

  it("rejects forbidden rule edits and moving an occurrence across its neighbour", async () => {
    const { server } = harness();
    const path = base(ROOM_IDS.engineering);
    const form = await get<EventForm>(server, `${path}/${EVENT_IDS.weeklySecond}/edit`);

    const rule = await send(
      server,
      "PATCH",
      `${path}/${EVENT_IDS.weeklySecond}`,
      updateBody(form, { recurrenceRule: "daily", updateScope: "this_and_following" }),
    );

    expect(rule.status).toBe(422);
    expect(field(field(rule.json, "error"), "fields")).toEqual({
      recurrenceRule: [
        "can only be changed from the first event in the series using This and following",
      ],
    });

    const moved = await send(
      server,
      "PATCH",
      `${path}/${EVENT_IDS.weeklySecond}`,
      updateBody(form, { startsAt: "2026-10-21T16:30", endsAt: null }),
    );

    expect(moved.status).toBe(422);
    expect(field(field(moved.json, "error"), "fields")).toEqual({
      startsAt: ["must stay between the neighbouring occurrences in its series"],
    });
  });

  it("validates body fields, cap, venue and attendance with structured errors", async () => {
    const { server } = harness();
    const missing = await send(server, "POST", base(), {});

    expect(missing.status).toBe(422);
    expect(field(field(missing.json, "error"), "fields")).toEqual({
      title: ["can't be blank"],
      startsAt: ["can't be blank"],
    });

    const cap = await send(
      server,
      "POST",
      base(),
      createBody({ recurrenceRule: "daily", recurrenceUntil: "2026-12-08" }),
    );

    expect(cap.status).toBe(422);
    expect(field(field(cap.json, "error"), "fields")).toEqual({
      recurrenceUntil: ["would create 62 occurrences (maximum 52); pick an earlier end date"],
    });

    const venue = await send(server, "POST", base(), createBody({ venueRoomId: ROOM_IDS.general }));

    expect(venue.status).toBe(422);
    expect(field(field(venue.json, "error"), "fields")).toEqual({
      venueRoomId: ["must be a voice or Stage channel you belong to"],
    });

    const cancelled = await send(server, "PUT", `${base()}/${EVENT_IDS.cancelled}/attendance`, {
      response: "going",
      applyToFuture: false,
    });

    expect(cancelled.status).toBe(422);
    expect(field(cancelled.json, "error")).toEqual({
      _tag: VALIDATION,
      message: "This event is no longer open for responses.",
      fields: { response: ["This event is no longer open for responses."] },
    });
    expect((await server.handle({ method: "GET", path: base(9999) })).status).toBe(404);
  });

  it("restores event seeds when the mock world resets", async () => {
    const { server } = harness();

    await expectStatus<EventDetail>(
      server,
      "PATCH",
      `${base()}/${EVENT_IDS.upcoming}/cancel`,
      {},
      200,
    );
    server.reset();

    expect(
      (await get<EventDetail>(server, `${base()}/${EVENT_IDS.upcoming}`)).event.cancelled,
    ).toBe(false);
  });
});
