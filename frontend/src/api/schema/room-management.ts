import { Schema } from "effect";
import type { CreateRoom as GeneratedCreateRoom } from "../../gen/CreateRoom.ts";
import type { RoomForm as GeneratedRoomForm } from "../../gen/RoomForm.ts";
import type { RoomFormStageRole as GeneratedRoomFormStageRole } from "../../gen/RoomFormStageRole.ts";
import type { RoomLeft as GeneratedRoomLeft } from "../../gen/RoomLeft.ts";
import type { RoomMutation as GeneratedRoomMutation } from "../../gen/RoomMutation.ts";
import type { RoomRemoved as GeneratedRoomRemoved } from "../../gen/RoomRemoved.ts";
import type { UpdateRoom as GeneratedUpdateRoom } from "../../gen/UpdateRoom.ts";
import { RoomId, UserId } from "./ids.ts";
import type { Assert, Pinned } from "./pin.ts";
import { Involvement, Room, RoomDetail, RoomKind, StageRole } from "./room.ts";
import { SidebarRow } from "./sidebar.ts";
import { User } from "./user.ts";

export const RoomFormStageRole = Schema.Struct({ userId: UserId, role: StageRole });

export type RoomFormStageRolePin = Assert<
  Pinned<typeof RoomFormStageRole, GeneratedRoomFormStageRole>
>;

/** Classic new/edit form facts; capabilities come from the server's persisted room policy. */
export const RoomForm = Schema.Struct({
  type: RoomKind,
  roomId: Schema.NullOr(RoomId),
  name: Schema.NullOr(Schema.String),
  iconName: Schema.NullOr(Schema.String),
  displayName: Schema.String,
  userIds: Schema.Array(UserId),
  memberIds: Schema.Array(UserId),
  candidateIds: Schema.Array(UserId),
  displayMemberIds: Schema.Array(UserId),
  users: Schema.Array(User),
  allowedTypes: Schema.Array(RoomKind),
  conversionTypes: Schema.Array(RoomKind),
  canSubmit: Schema.Boolean,
  canDelete: Schema.Boolean,
  canLeave: Schema.Boolean,
  groupCapable: Schema.Boolean,
  defaultInvolvement: Involvement,
  stageRoles: Schema.Array(RoomFormStageRole),
});

export type RoomFormPin = Assert<Pinned<typeof RoomForm, GeneratedRoomForm>>;

const createFields = {
  name: Schema.NullOr(Schema.String),
  iconName: Schema.NullOr(Schema.String),
};

const memberFields = { userIds: Schema.Array(UserId) };

/** Non-direct creation; explicit member lists are required on every selected-members type. */
export const CreateRoom = Schema.Union([
  Schema.Struct({ type: Schema.Literal("open"), ...createFields }),
  Schema.Struct({ type: Schema.Literal("closed"), ...createFields, ...memberFields }),
  Schema.Struct({ type: Schema.Literal("voice"), ...createFields, ...memberFields }),
  Schema.Struct({ type: Schema.Literal("stage"), ...createFields, ...memberFields }),
  Schema.Struct({ type: Schema.Literal("board"), ...createFields, ...memberFields }),
]);

export type CreateRoomPin = Assert<Pinned<typeof CreateRoom, GeneratedCreateRoom>>;

const updateFields = {
  name: Schema.optionalKey(Schema.NullOr(Schema.String)),
  iconName: Schema.optionalKey(Schema.NullOr(Schema.String)),
};

/** Omitted name/icon retain their values; null clears them. Members replace the full list. */
export const UpdateRoom = Schema.Union([
  Schema.Struct({ type: Schema.Literal("open"), ...updateFields }),
  Schema.Struct({ type: Schema.Literal("closed"), ...updateFields, ...memberFields }),
  Schema.Struct({ type: Schema.Literal("voice"), ...updateFields, ...memberFields }),
  Schema.Struct({ type: Schema.Literal("stage"), ...updateFields, ...memberFields }),
  Schema.Struct({ type: Schema.Literal("board"), ...updateFields, ...memberFields }),
]);

export type UpdateRoomPin = Assert<Pinned<typeof UpdateRoom, GeneratedUpdateRoom>>;

/** A successful save may leave the acting person without membership or a visible row. */
export const RoomMutation = Schema.Struct({
  room: Room,
  detail: Schema.NullOr(RoomDetail),
  row: Schema.NullOr(SidebarRow),
});

export type RoomMutationPin = Assert<Pinned<typeof RoomMutation, GeneratedRoomMutation>>;

export const RoomRemoved = Schema.Struct({ roomId: RoomId, deleted: Schema.Boolean });

export type RoomRemovedPin = Assert<Pinned<typeof RoomRemoved, GeneratedRoomRemoved>>;

export const RoomLeft = Schema.Struct({ roomId: RoomId, deleted: Schema.Boolean });

export type RoomLeftPin = Assert<Pinned<typeof RoomLeft, GeneratedRoomLeft>>;
