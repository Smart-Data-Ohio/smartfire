import { Schema } from "effect";
import type { Involvement as GeneratedInvolvement } from "../../gen/Involvement.ts";
import type { Membership as GeneratedMembership } from "../../gen/Membership.ts";
import type { Room as GeneratedRoom } from "../../gen/Room.ts";
import type { RoomKind as GeneratedRoomKind } from "../../gen/RoomKind.ts";
import type { StageRole as GeneratedStageRole } from "../../gen/StageRole.ts";
import { MembershipId, MessageId, RoomCategoryId, RoomId, UserId } from "./ids.ts";
import type { Assert, Pinned } from "./pin.ts";
import { Timestamp } from "./time.ts";

export const RoomKind = Schema.Literals(["open", "closed", "direct", "voice", "stage", "board"]);

export type RoomKindPin = Assert<Pinned<typeof RoomKind, GeneratedRoomKind>>;

/** A channel, direct message, voice or stage room, or board. */
export const Room = Schema.Struct({
  id: RoomId,
  kind: RoomKind,
  name: Schema.NullOr(Schema.String),
  iconName: Schema.NullOr(Schema.String),
  creatorId: UserId,
  createdAt: Timestamp,
  updatedAt: Timestamp,
});

export type Room = typeof Room.Type;

export type RoomPin = Assert<Pinned<typeof Room, GeneratedRoom>>;

export const Involvement = Schema.Literals([
  "invisible",
  "nothing",
  "muted",
  "mentions",
  "everything",
]);

export type InvolvementPin = Assert<Pinned<typeof Involvement, GeneratedInvolvement>>;

export const StageRole = Schema.Literals(["listener", "speaker", "host"]);

export type StageRolePin = Assert<Pinned<typeof StageRole, GeneratedStageRole>>;

/** One person's place in a room: notification level, read position, sidebar placement. */
export const Membership = Schema.Struct({
  id: MembershipId,
  roomId: RoomId,
  userId: UserId,
  involvement: Involvement,
  unreadAt: Schema.NullOr(Timestamp),
  lastReadMessageId: Schema.NullOr(MessageId),
  roomCategoryId: Schema.NullOr(RoomCategoryId),
  favoritePosition: Schema.NullOr(Schema.Int),
  stageRole: Schema.NullOr(StageRole),
});

export type Membership = typeof Membership.Type;

export type MembershipPin = Assert<Pinned<typeof Membership, GeneratedMembership>>;
