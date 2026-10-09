import { describe, expect, it } from "@effect/vitest";
import { DateTime, Schema } from "effect";
import { CreateWorkLink, WorkLinkEventCandidate, WorkLinkForm } from "./work-links.ts";

const candidate = {
  id: 9,
  title: "API review",
  startsAt: "2026-10-08T14:00:00.000Z",
  timeZone: "America/New_York",
};

const decode = Schema.decodeUnknownSync;

describe("work link schemas", () => {
  it("decodes the picker and round trips its timestamp and time zone", () => {
    const form = decode(WorkLinkForm)({ events: [candidate] });
    expect(DateTime.formatIso(form.events[0]?.startsAt ?? DateTime.makeUnsafe(0))).toBe(
      candidate.startsAt,
    );
    expect(Schema.encodeSync(WorkLinkForm)(form)).toEqual({ events: [candidate] });
    expect(decode(WorkLinkForm)({ events: [] })).toEqual({ events: [] });
  });
  it("accepts every kind, omitted inactive inputs, and explicit nulls", () => {
    for (const input of [
      { kind: "pull_request", pullRequestUrl: "https://github.com/owner/repo/pull/123" },
      { kind: "event", eventId: 9 },
      { kind: "drive_file", driveUrl: "https://docs.google.com/document/d/1234567890/edit" },
      { kind: "event", eventId: null, pullRequestUrl: null, driveUrl: null },
    ])
      expect(decode(CreateWorkLink)(input)).toEqual(input);
  });
  it("rejects unknown kinds, wrong inputs, bad ids and invalid timestamps", () => {
    for (const input of [
      { kind: "other" },
      {},
      { kind: "event", eventId: "9" },
      { kind: "event", eventId: 1.5 },
      { kind: "drive_file", driveUrl: 4 },
    ]) {
      expect(() => decode(CreateWorkLink)(input)).toThrow();
    }

    for (const changes of [
      { id: "9" },
      { id: 1.5 },
      { startsAt: "tomorrow" },
      { timeZone: null },
    ]) {
      expect(() => decode(WorkLinkEventCandidate)({ ...candidate, ...changes })).toThrow();
    }
  });
});
