import { Schema } from "effect";
import type { CreateWorkLink as GeneratedCreateWorkLink } from "../../gen/CreateWorkLink.ts";
import type { WorkLinkEventCandidate as GeneratedWorkLinkEventCandidate } from "../../gen/WorkLinkEventCandidate.ts";
import type { WorkLinkForm as GeneratedWorkLinkForm } from "../../gen/WorkLinkForm.ts";
import { EventId } from "./ids.ts";
import type { Assert, Pinned } from "./pin.ts";
import { Timestamp } from "./time.ts";

/** Classic event choices, in start/id order, shown in the event's time zone. */
export const WorkLinkEventCandidate = Schema.Struct({
  id: EventId,
  title: Schema.String,
  startsAt: Timestamp,
  timeZone: Schema.String,
});

export type WorkLinkEventCandidate = typeof WorkLinkEventCandidate.Type;

export type WorkLinkEventCandidatePin = Assert<
  Pinned<typeof WorkLinkEventCandidate, GeneratedWorkLinkEventCandidate>
>;

/** `GET /threads/:id/work/links/new`: upcoming, uncancelled, unlinked room events. */
export const WorkLinkForm = Schema.Struct({ events: Schema.Array(WorkLinkEventCandidate) });

export type WorkLinkForm = typeof WorkLinkForm.Type;

export type WorkLinkFormPin = Assert<Pinned<typeof WorkLinkForm, GeneratedWorkLinkForm>>;

/** `POST /threads/:id/work/links`: only the selected kind's input is needed. */
export const CreateWorkLink = Schema.Struct({
  kind: Schema.Literals(["pull_request", "event", "drive_file"]),
  pullRequestUrl: Schema.optionalKey(Schema.NullOr(Schema.String)),
  eventId: Schema.optionalKey(Schema.NullOr(EventId)),
  driveUrl: Schema.optionalKey(Schema.NullOr(Schema.String)),
});

export type CreateWorkLink = typeof CreateWorkLink.Type;

export type CreateWorkLinkPin = Assert<Pinned<typeof CreateWorkLink, GeneratedCreateWorkLink>>;
