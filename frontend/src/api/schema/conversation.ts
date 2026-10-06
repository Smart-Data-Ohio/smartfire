import { Schema } from "effect";
import type { ConversationName as GeneratedConversationName } from "../../gen/ConversationName.ts";
import { RoomId, ThreadId } from "./ids.ts";
import type { Assert, Pinned } from "./pin.ts";
import { RoomKind } from "./room.ts";

/**
 * What a cross-room list's rows call their room (viewer-relative) and thread, one per
 * `(roomId, threadId)` pair the list names.
 */
export const ConversationName = Schema.Struct({
  roomId: RoomId,
  threadId: Schema.NullOr(ThreadId),
  roomKind: RoomKind,
  roomName: Schema.String,
  roomIconName: Schema.NullOr(Schema.String),
  threadName: Schema.NullOr(Schema.String),
});

export type ConversationName = typeof ConversationName.Type;

export type ConversationNamePin = Assert<
  Pinned<typeof ConversationName, GeneratedConversationName>
>;
