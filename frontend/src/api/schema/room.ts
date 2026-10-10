import { Schema } from "effect";
import type { Involvement as GeneratedInvolvement } from "../../gen/Involvement.ts";
import type { Membership as GeneratedMembership } from "../../gen/Membership.ts";
import type { Room as GeneratedRoom } from "../../gen/Room.ts";
import type { RoomDetail as GeneratedRoomDetail } from "../../gen/RoomDetail.ts";
import type { RoomKind as GeneratedRoomKind } from "../../gen/RoomKind.ts";
import type { StageRole as GeneratedStageRole } from "../../gen/StageRole.ts";
import type { UnreadDivider as GeneratedUnreadDivider } from "../../gen/UnreadDivider.ts";
import { MembershipId, MessageId, RoomCategoryId, RoomId, UserId } from "./ids.ts";
import type { Assert, Pinned } from "./pin.ts";
import { Timestamp } from "./time.ts";
import { User } from "./user.ts";

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
  topic: Schema.NullOr(Schema.String),
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

/** Where the unread divider goes: above `firstUnreadMessageId`, with `count` root messages from it. */
export const UnreadDivider = Schema.Struct({
  firstUnreadMessageId: MessageId,
  count: Schema.Int,
});

export type UnreadDivider = typeof UnreadDivider.Type;

export type UnreadDividerPin = Assert<Pinned<typeof UnreadDivider, GeneratedUnreadDivider>>;

/** `GET /api/v1/rooms/:id`: the room, the viewer's membership and what the header needs. */
export const RoomDetail = Schema.Struct({
  room: Room,
  membership: Membership,
  displayName: Schema.String,
  memberCount: Schema.Int,
  pinsCount: Schema.Int,
  directMemberIds: Schema.Array(UserId),
  memberPreviewIds: Schema.Array(UserId),
  users: Schema.Array(User),
  unread: Schema.NullOr(UnreadDivider),
});

export type RoomDetail = typeof RoomDetail.Type;

export type RoomDetailPin = Assert<Pinned<typeof RoomDetail, GeneratedRoomDetail>>;
