import { Schema } from "effect";

/** Ids are SQLite rowids, integers below 2^53 on the wire; each kind is branded apart. */
export const UserId = Schema.Int.pipe(Schema.brand("UserId"));

export type UserId = typeof UserId.Type;

export const RoomId = Schema.Int.pipe(Schema.brand("RoomId"));

export type RoomId = typeof RoomId.Type;

export const MembershipId = Schema.Int.pipe(Schema.brand("MembershipId"));

export type MembershipId = typeof MembershipId.Type;

export const MessageId = Schema.Int.pipe(Schema.brand("MessageId"));

export type MessageId = typeof MessageId.Type;

export const ThreadId = Schema.Int.pipe(Schema.brand("ThreadId"));

export type ThreadId = typeof ThreadId.Type;

export const RoomCategoryId = Schema.Int.pipe(Schema.brand("RoomCategoryId"));

export type RoomCategoryId = typeof RoomCategoryId.Type;
