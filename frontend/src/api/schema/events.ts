import { Schema } from "effect";
import type { CancelEvent as GeneratedCancelEvent } from "../../gen/CancelEvent.ts";
import type { ChannelEvent as GeneratedChannelEvent } from "../../gen/ChannelEvent.ts";
import type { CreateEvent as GeneratedCreateEvent } from "../../gen/CreateEvent.ts";
import type { EventAttendee as GeneratedEventAttendee } from "../../gen/EventAttendee.ts";
import type { EventCounts as GeneratedEventCounts } from "../../gen/EventCounts.ts";
import type { EventDetail as GeneratedEventDetail } from "../../gen/EventDetail.ts";
import type { EventForm as GeneratedEventForm } from "../../gen/EventForm.ts";
import type { EventLimits as GeneratedEventLimits } from "../../gen/EventLimits.ts";
import type { EventList as GeneratedEventList } from "../../gen/EventList.ts";
import type { EventRecurrenceRule as GeneratedEventRecurrenceRule } from "../../gen/EventRecurrenceRule.ts";
import type { EventRepeatOption as GeneratedEventRepeatOption } from "../../gen/EventRepeatOption.ts";
import type { EventScope as GeneratedEventScope } from "../../gen/EventScope.ts";
import type { EventValues as GeneratedEventValues } from "../../gen/EventValues.ts";
import type { EventVenue as GeneratedEventVenue } from "../../gen/EventVenue.ts";
import type { EventVenueOption as GeneratedEventVenueOption } from "../../gen/EventVenueOption.ts";
import type { UpdateEvent as GeneratedUpdateEvent } from "../../gen/UpdateEvent.ts";
import { AttendanceResponse } from "./cards.ts";
import { EventId, RoomId } from "./ids.ts";
import type { Assert, Pinned } from "./pin.ts";
import { RoomKind } from "./room.ts";
import { Timestamp } from "./time.ts";

export const EventRecurrenceRule = Schema.Literals(["daily", "weekly", "biweekly", "monthly"]);

export type EventRecurrenceRulePin = Assert<
  Pinned<typeof EventRecurrenceRule, GeneratedEventRecurrenceRule>
>;

export const EventScope = Schema.Literals(["this_event", "this_and_following"]);

export type EventScopePin = Assert<Pinned<typeof EventScope, GeneratedEventScope>>;

export const EventVenue = Schema.Struct({
  roomId: RoomId,
  name: Schema.String,
  kind: RoomKind,
  member: Schema.Boolean,
  liveUser: Schema.NullOr(Schema.String),
});

export type EventVenuePin = Assert<Pinned<typeof EventVenue, GeneratedEventVenue>>;

export const EventCounts = Schema.Struct({
  going: Schema.Int,
  maybe: Schema.Int,
  declined: Schema.Int,
});

export type EventCountsPin = Assert<Pinned<typeof EventCounts, GeneratedEventCounts>>;

export const ChannelEvent = Schema.Struct({
  id: EventId,
  roomId: RoomId,
  title: Schema.String,
  startsAt: Timestamp,
  endsAt: Schema.NullOr(Timestamp),
  timeZone: Schema.String,
  zoneLabel: Schema.String,
  organizerName: Schema.String,
  venue: Schema.NullOr(EventVenue),
  counts: EventCounts,
  recurrenceLabel: Schema.NullOr(Schema.String),
  remainingOccurrences: Schema.NullOr(Schema.Int),
  cancelled: Schema.Boolean,
  series: Schema.Boolean,
});

export type ChannelEventPin = Assert<Pinned<typeof ChannelEvent, GeneratedChannelEvent>>;

export const EventList = Schema.Struct({
  roomId: RoomId,
  roomName: Schema.String,
  roomKind: RoomKind,
  mayCreate: Schema.Boolean,
  upcoming: Schema.Array(ChannelEvent),
  past: Schema.Array(ChannelEvent),
  cancelled: Schema.Array(ChannelEvent),
});

export type EventListPin = Assert<Pinned<typeof EventList, GeneratedEventList>>;

export const EventAttendee = Schema.Struct({
  name: Schema.String,
  response: AttendanceResponse,
});

export type EventAttendeePin = Assert<Pinned<typeof EventAttendee, GeneratedEventAttendee>>;

export const EventDetail = Schema.Struct({
  roomId: RoomId,
  roomName: Schema.String,
  event: ChannelEvent,
  descriptionHtml: Schema.NullOr(Schema.String),
  manageable: Schema.Boolean,
  respondable: Schema.Boolean,
  currentResponse: Schema.NullOr(AttendanceResponse),
  head: Schema.Boolean,
  canApplyToFuture: Schema.Boolean,
  previousOccurrenceId: Schema.NullOr(EventId),
  nextOccurrenceId: Schema.NullOr(EventId),
  recurrencePhrase: Schema.NullOr(Schema.String),
  recurrenceUntil: Schema.NullOr(Schema.String),
  meetLink: Schema.NullOr(Schema.String),
  calendarCopy: Schema.Boolean,
  attendees: Schema.Array(EventAttendee),
});

export type EventDetailPin = Assert<Pinned<typeof EventDetail, GeneratedEventDetail>>;

export const EventVenueOption = Schema.Struct({
  roomId: RoomId,
  name: Schema.String,
  kind: RoomKind,
});

export type EventVenueOptionPin = Assert<
  Pinned<typeof EventVenueOption, GeneratedEventVenueOption>
>;

export const EventRepeatOption = Schema.Struct({
  value: Schema.NullOr(EventRecurrenceRule),
  label: Schema.String,
});

export type EventRepeatOptionPin = Assert<
  Pinned<typeof EventRepeatOption, GeneratedEventRepeatOption>
>;

export const EventLimits = Schema.Struct({
  titleMaxLength: Schema.Int,
  maxOccurrences: Schema.Int,
  maxRecurrenceYears: Schema.Int,
});

export type EventLimitsPin = Assert<Pinned<typeof EventLimits, GeneratedEventLimits>>;

export const EventValues = Schema.Struct({
  title: Schema.String,
  description: Schema.NullOr(Schema.String),
  startsAt: Schema.NullOr(Schema.String),
  endsAt: Schema.NullOr(Schema.String),
  timeZone: Schema.String,
  venueRoomId: Schema.NullOr(RoomId),
  recurrenceRule: Schema.NullOr(EventRecurrenceRule),
  recurrenceUntil: Schema.NullOr(Schema.String),
  meetLinkRequested: Schema.Boolean,
  meetLink: Schema.NullOr(Schema.String),
});

export type EventValuesPin = Assert<Pinned<typeof EventValues, GeneratedEventValues>>;

export const EventForm = Schema.Struct({
  roomId: RoomId,
  roomName: Schema.String,
  eventId: Schema.NullOr(EventId),
  values: EventValues,
  venues: Schema.Array(EventVenueOption),
  meetAvailable: Schema.Boolean,
  repeatOptions: Schema.Array(EventRepeatOption),
  limits: EventLimits,
  series: Schema.Boolean,
  head: Schema.Boolean,
  ruleEditable: Schema.Boolean,
  scopeOptions: Schema.Array(EventScope),
});

export type EventFormPin = Assert<Pinned<typeof EventForm, GeneratedEventForm>>;

const writable = {
  title: Schema.String,
  description: Schema.NullOr(Schema.String),
  startsAt: Schema.NullOr(Schema.String),
  endsAt: Schema.NullOr(Schema.String),
  timeZone: Schema.NullOr(Schema.String),
  venueRoomId: Schema.NullOr(RoomId),
  recurrenceRule: Schema.NullOr(Schema.String),
  recurrenceUntil: Schema.NullOr(Schema.String),
  meetLinkRequested: Schema.Boolean,
};

export const CreateEvent = Schema.Struct(writable);

export type CreateEventPin = Assert<Pinned<typeof CreateEvent, GeneratedCreateEvent>>;

/** Submit displayed controls. Omit hidden recurrence/Meet controls to preserve their values. */
export const UpdateEvent = Schema.Struct({
  title: Schema.optionalKey(Schema.String),
  description: Schema.optionalKey(Schema.NullOr(Schema.String)),
  startsAt: Schema.optionalKey(Schema.NullOr(Schema.String)),
  endsAt: Schema.optionalKey(Schema.NullOr(Schema.String)),
  timeZone: Schema.optionalKey(Schema.String),
  venueRoomId: Schema.optionalKey(Schema.NullOr(RoomId)),
  recurrenceRule: Schema.optionalKey(Schema.NullOr(Schema.String)),
  recurrenceUntil: Schema.optionalKey(Schema.NullOr(Schema.String)),
  meetLinkRequested: Schema.optionalKey(Schema.Boolean),
  updateScope: Schema.optionalKey(Schema.String),
});

export type UpdateEventPin = Assert<Pinned<typeof UpdateEvent, GeneratedUpdateEvent>>;

export const CancelEvent = Schema.Struct({ cancelScope: Schema.NullOr(Schema.String) });

export type CancelEventPin = Assert<Pinned<typeof CancelEvent, GeneratedCancelEvent>>;
