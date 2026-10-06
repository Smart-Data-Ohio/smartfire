import { Schema } from "effect";
import type { MessageDTO as GeneratedMessageDTO } from "../../gen/MessageDTO.ts";
import type { MessageRemoved as GeneratedMessageRemoved } from "../../gen/MessageRemoved.ts";
import { MessageId, RoomId, ThreadId, UserId } from "./ids.ts";
import type { Assert, Pinned } from "./pin.ts";
import { Timestamp } from "./time.ts";

/** A message on a room's timeline or in a thread. `bodyHtml` is already sanitized. */
export const MessageDTO = Schema.Struct({
  id: MessageId,
  roomId: RoomId,
  threadId: Schema.NullOr(ThreadId),
  creatorId: UserId,
  clientMessageId: Schema.String,
  bodyHtml: Schema.String,
  markdownSource: Schema.NullOr(Schema.String),
  systemNote: Schema.Boolean,
  action: Schema.Boolean,
  streaming: Schema.Boolean,
  embedsSuppressed: Schema.Boolean,
  replyToMessageId: Schema.NullOr(MessageId),
  forwardedFromMessageId: Schema.NullOr(MessageId),
  editedAt: Schema.NullOr(Timestamp),
  createdAt: Timestamp,
  updatedAt: Timestamp,
});

export type MessageDTO = typeof MessageDTO.Type;

export type MessageDTOPin = Assert<Pinned<typeof MessageDTO, GeneratedMessageDTO>>;

/** The `message.removed` event's data. */
export const MessageRemoved = Schema.Struct({
  id: MessageId,
  roomId: RoomId,
  threadId: Schema.NullOr(ThreadId),
});

export type MessageRemoved = typeof MessageRemoved.Type;

export type MessageRemovedPin = Assert<Pinned<typeof MessageRemoved, GeneratedMessageRemoved>>;
