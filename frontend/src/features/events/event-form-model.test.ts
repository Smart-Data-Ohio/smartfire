import { describe, expect, it } from "vitest";
import type { EventForm } from "../../gen/EventForm.ts";
import {
  createBody,
  initialDraft,
  isDirty,
  newEventPrefill,
  showsRepeat,
  splitErrors,
  updateBody,
  venueGroups,
} from "./event-form-model.ts";
import { relativeDay } from "./event-format.tsx";

function form(patch: Partial<EventForm> = {}): EventForm {
  return {
    roomId: 1,
    roomName: "general",
    eventId: null,
    values: {
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
    },
    venues: [
      { roomId: 8, name: "Lounge", kind: "voice" },
      { roomId: 12, name: "Town hall", kind: "stage" },
    ],
    meetAvailable: false,
    repeatOptions: [{ value: null, label: "Does not repeat" }],
    limits: { titleMaxLength: 255, maxOccurrences: 100, maxRecurrenceYears: 2 },
    series: false,
    head: false,
    ruleEditable: false,
    scopeOptions: [],
    ...patch,
  };
}

describe("the event form's draft", () => {
  it("starts a new event at the next whole hour", () => {
    const draft = initialDraft(form(), new Date(2026, 9, 7, 14, 25));

    expect(draft.startsAt).toBe("2026-10-07T15:00");
    expect(draft.venueRoomId).toBe("");
    expect(showsRepeat(form())).toBe(true);
  });

  it("opens an edit on the event's own values, the until date as a date", () => {
    const edit = form({
      eventId: 5,
      values: {
        ...form().values,
        title: "Sync",
        startsAt: "2026-10-08T11:00",
        venueRoomId: 8,
        recurrenceRule: "weekly",
        recurrenceUntil: "2026-12-31T00:00:00Z",
      },
    });

    const draft = initialDraft(edit, new Date());

    expect(draft).toMatchObject({
      title: "Sync",
      startsAt: "2026-10-08T11:00",
      venueRoomId: "8",
      recurrenceRule: "weekly",
      recurrenceUntil: "2026-12-31",
    });
    expect(showsRepeat(edit)).toBe(false);
    expect(isDirty(draft, draft)).toBe(false);
    expect(isDirty(draft, { ...draft, title: "Sync 2" })).toBe(true);
  });
});

describe("the bodies it sends", () => {
  it("schedules in the zone given, blanks as null, and no until without a rule", () => {
    const draft = {
      ...initialDraft(form(), new Date(2026, 9, 7, 9, 0)),
      title: "  Demo ",
      venueRoomId: "12",
      recurrenceUntil: "2026-11-01",
    };

    expect(createBody(draft, "America/New_York")).toEqual({
      title: "Demo",
      description: null,
      startsAt: "2026-10-07T10:00",
      endsAt: null,
      timeZone: "America/New_York",
      venueRoomId: 12,
      recurrenceRule: null,
      recurrenceUntil: null,
      meetLinkRequested: false,
    });
  });

  it("sends only the controls the classic edit form shows", () => {
    const plain = form({ eventId: 5, values: { ...form().values, title: "Sync" } });
    const draft = initialDraft(plain, new Date());

    expect(updateBody(plain, draft)).toEqual({
      title: "Sync",
      description: null,
      startsAt: null,
      endsAt: null,
      venueRoomId: null,
    });

    const head = form({
      eventId: 5,
      series: true,
      head: true,
      ruleEditable: true,
      meetAvailable: true,
    });

    const headDraft = {
      ...initialDraft(head, new Date()),
      recurrenceRule: "daily",
      scope: "this_and_following" as const,
    };

    expect(updateBody(head, headDraft)).toMatchObject({
      meetLinkRequested: false,
      recurrenceRule: "daily",
      recurrenceUntil: null,
      updateScope: "this_and_following",
    });
  });
});

describe("venues and errors", () => {
  it("groups voice channels before stages", () => {
    expect(venueGroups(form().venues).map((group) => group.label)).toEqual(["Voice", "Stage"]);
    expect(venueGroups([])).toEqual([]);
  });

  it("names fields the way the form does and keeps the rest for the summary", () => {
    expect(splitErrors({ title: ["can't be blank"], base: ["is busy"], endsAt: [] })).toEqual({
      byField: { title: "Title can't be blank." },
      other: ["base is busy."],
    });
  });
});

describe("relative days", () => {
  const now = new Date(2026, 9, 7, 12, 0).getTime();
  const at = (day: number, hour: number) => new Date(2026, 9, day, hour, 0).toISOString();

  it("says now, today, tomorrow or in a few days, and nothing further out or over", () => {
    expect(relativeDay(at(7, 11), at(7, 13), now)).toEqual({ label: "Happening now", live: true });
    expect(relativeDay(at(7, 15), null, now)?.label).toBe("Today");
    expect(relativeDay(at(8, 9), null, now)?.label).toBe("Tomorrow");
    expect(relativeDay(at(10, 9), null, now)?.label).toBe("In 3 days");
    expect(relativeDay(at(20, 9), null, now)).toBeNull();
    expect(relativeDay(at(6, 9), at(6, 10), now)).toBeNull();
  });
});

describe("prefilled links", () => {
  it("reads the /event command's nested query as text, asking for the form's own zone", () => {
    const search =
      "?event%5Bstarts_at%5D=2030-03-08T22%3A00%3A00Z&event%5Btime_zone%5D=Eastern+Time+%28US+%26+Canada%29&event%5Btitle%5D=2026";

    expect(newEventPrefill(search, "Europe/Berlin")).toEqual({
      title: "2026",
      startsAt: "2030-03-08T22:00:00Z",
      timeZone: "Europe/Berlin",
    });
    expect(newEventPrefill("?event%5Btitle%5D=Launch+party", "UTC")).toEqual({
      title: "Launch party",
      startsAt: null,
      timeZone: "UTC",
    });
  });

  it("asks for a bare form when the link fills nothing in", () => {
    expect(newEventPrefill("", "UTC")).toBeNull();
    expect(newEventPrefill("?event%5Btime_zone%5D=UTC&other=1", "UTC")).toBeNull();
  });
});
