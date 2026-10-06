import { Schema } from "effect";
import type { HuddleCredentials as GeneratedHuddleCredentials } from "../../gen/HuddleCredentials.ts";
import type { HuddleDetail as GeneratedHuddleDetail } from "../../gen/HuddleDetail.ts";
import type { HuddleModeration as GeneratedHuddleModeration } from "../../gen/HuddleModeration.ts";
import type { HuddleNotice as GeneratedHuddleNotice } from "../../gen/HuddleNotice.ts";
import type { HuddleParticipant as GeneratedHuddleParticipant } from "../../gen/HuddleParticipant.ts";
import type { HuddlePresence as GeneratedHuddlePresence } from "../../gen/HuddlePresence.ts";
import type { HuddlePresenceList as GeneratedHuddlePresenceList } from "../../gen/HuddlePresenceList.ts";
import type { HuddleRing as GeneratedHuddleRing } from "../../gen/HuddleRing.ts";
import type { HuddleRingEvent as GeneratedHuddleRingEvent } from "../../gen/HuddleRingEvent.ts";
import type { HuddleRingState as GeneratedHuddleRingState } from "../../gen/HuddleRingState.ts";
import type { HuddleRoleChanged as GeneratedHuddleRoleChanged } from "../../gen/HuddleRoleChanged.ts";
import type { ModerateHuddle as GeneratedModerateHuddle } from "../../gen/ModerateHuddle.ts";
import { MembershipId, RoomId, UserId } from "./ids.ts";
import type { Assert, Pinned } from "./pin.ts";
import { StageRole } from "./room.ts";
import { User } from "./user.ts";

/** A huddle grant: one session's ticket into one room's call. */
export const GrantId = Schema.Int.pipe(Schema.brand("GrantId"));

export type GrantId = typeof GrantId.Type;

/** Someone in a room's call, with the LiveKit identities of each tab or device they joined from. */
export const HuddleParticipant = Schema.Struct({
  userId: UserId,
  membershipId: MembershipId,
  identities: Schema.Array(Schema.String),
  serverMuted: Schema.Boolean,
});

export type HuddleParticipant = typeof HuddleParticipant.Type;

export type HuddleParticipantPin = Assert<
  Pinned<typeof HuddleParticipant, GeneratedHuddleParticipant>
>;

/** Who's in one room's call (ordered by name); also the `huddle.presence` event's data. */
export const HuddlePresence = Schema.Struct({
  roomId: RoomId,
  participants: Schema.Array(HuddleParticipant),
  live: Schema.Boolean,
});

export type HuddlePresence = typeof HuddlePresence.Type;

export type HuddlePresencePin = Assert<Pinned<typeof HuddlePresence, GeneratedHuddlePresence>>;

/** `GET /api/v1/huddles`: the calls in the viewer's rooms. */
export const HuddlePresenceList = Schema.Struct({
  rooms: Schema.Array(HuddlePresence),
  users: Schema.Array(User),
});

export type HuddlePresenceList = typeof HuddlePresenceList.Type;

export type HuddlePresenceListPin = Assert<
  Pinned<typeof HuddlePresenceList, GeneratedHuddlePresenceList>
>;

/** `GET /api/v1/rooms/:id/huddle`: one room's call; also the in-call access check. */
export const HuddleDetail = Schema.Struct({
  roomName: Schema.String,
  presence: HuddlePresence,
  users: Schema.Array(User),
});

export type HuddleDetail = typeof HuddleDetail.Type;

export type HuddleDetailPin = Assert<Pinned<typeof HuddleDetail, GeneratedHuddleDetail>>;

/** `POST /api/v1/rooms/:id/huddle`: the LiveKit URL and token for a fresh grant. */
export const HuddleCredentials = Schema.Struct({
  url: Schema.String,
  token: Schema.String,
  identity: Schema.String,
  grantId: GrantId,
  roomId: RoomId,
  roomName: Schema.String,
  canPublish: Schema.Boolean,
});

export type HuddleCredentials = typeof HuddleCredentials.Type;

export type HuddleCredentialsPin = Assert<
  Pinned<typeof HuddleCredentials, GeneratedHuddleCredentials>
>;

export const HuddleModeration = Schema.Literals(["mute", "unmute", "disconnect"]);

export type HuddleModeration = typeof HuddleModeration.Type;

export type HuddleModerationPin = Assert<
  Pinned<typeof HuddleModeration, GeneratedHuddleModeration>
>;

/** `POST /api/v1/rooms/:id/huddle/moderation`'s body. */
export const ModerateHuddle = Schema.Struct({
  membershipId: MembershipId,
  action: HuddleModeration,
});

export type ModerateHuddle = typeof ModerateHuddle.Type;

export type ModerateHuddlePin = Assert<Pinned<typeof ModerateHuddle, GeneratedModerateHuddle>>;

/** The `huddle.role` event: rejoin with a fresh token; say so if a host muted the viewer. */
export const HuddleRoleChanged = Schema.Struct({
  roomId: RoomId,
  stageRole: Schema.NullOr(StageRole),
  serverMuted: Schema.Boolean,
});

export type HuddleRoleChanged = typeof HuddleRoleChanged.Type;

export type HuddleRoleChangedPin = Assert<
  Pinned<typeof HuddleRoleChanged, GeneratedHuddleRoleChanged>
>;

/** The `huddle.notice` event: someone joined or left a call in one of the viewer's rooms. */
export const HuddleNotice = Schema.Union([
  Schema.Struct({
    kind: Schema.Literal("joined"),
    roomId: RoomId,
    roomName: Schema.String,
    userId: UserId,
    userName: Schema.String,
    inCall: Schema.Boolean,
    rejoin: Schema.Boolean,
  }),
  Schema.Struct({
    kind: Schema.Literal("left"),
    roomId: RoomId,
    roomName: Schema.String,
    userId: UserId,
    userName: Schema.String,
  }),
  Schema.Struct({ kind: Schema.Literal("ended"), roomId: RoomId }),
]);

export type HuddleNotice = typeof HuddleNotice.Type;

export type HuddleNoticePin = Assert<Pinned<typeof HuddleNotice, GeneratedHuddleNotice>>;

export const HuddleRingEvent = Schema.Literals(["started", "missed", "ended"]);

export type HuddleRingEventPin = Assert<Pinned<typeof HuddleRingEvent, GeneratedHuddleRingEvent>>;

export const HuddleRingState = Schema.Literals(["unread", "read", "handled"]);

export type HuddleRingStatePin = Assert<Pinned<typeof HuddleRingState, GeneratedHuddleRingState>>;

/** The `huddle.ring` event: an incoming call rings, or stops ringing. */
export const HuddleRing = Schema.Struct({
  /** The inbox item (`PATCH /api/v1/activity/:id` answers it); `null` when there's none yet. */
  activityItemId: Schema.NullOr(Schema.Int),
  event: HuddleRingEvent,
  state: HuddleRingState,
  roomId: RoomId,
  roomName: Schema.String,
  callerName: Schema.String,
  silent: Schema.Boolean,
});

export type HuddleRing = typeof HuddleRing.Type;

export type HuddleRingPin = Assert<Pinned<typeof HuddleRing, GeneratedHuddleRing>>;
