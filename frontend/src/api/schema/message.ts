import { Schema } from "effect";
import type { CreateMessage as GeneratedCreateMessage } from "../../gen/CreateMessage.ts";
import type { MessageDTO as GeneratedMessageDTO } from "../../gen/MessageDTO.ts";
import type { MessagePage as GeneratedMessagePage } from "../../gen/MessagePage.ts";
import type { MessageRemoved as GeneratedMessageRemoved } from "../../gen/MessageRemoved.ts";
import { MessageId, RoomId, ThreadId, UserId } from "./ids.ts";
import type { Assert, Pinned } from "./pin.ts";
import { Timestamp } from "./time.ts";
import { User } from "./user.ts";

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

/**
 * `GET /api/v1/rooms/:id/messages`: up to 40 root messages, oldest first, with their authors.
 * `before`/`after` are the ids to page from next, `null` at the start of the room / the present.
 */
export const MessagePage = Schema.Struct({
  messages: Schema.Array(MessageDTO),
  users: Schema.Array(User),
  before: Schema.NullOr(MessageId),
  after: Schema.NullOr(MessageId),
});

export type MessagePage = typeof MessagePage.Type;

export type MessagePagePin = Assert<Pinned<typeof MessagePage, GeneratedMessagePage>>;

/** The body of `POST /api/v1/rooms/:id/messages`. Idempotent on `clientMessageId`. */
export const CreateMessage = Schema.Struct({
  clientMessageId: Schema.String,
  markdownSource: Schema.String,
  replyToMessageId: Schema.NullOr(MessageId),
  replyNotifyAuthor: Schema.NullOr(Schema.Boolean),
});

export type CreateMessage = typeof CreateMessage.Type;

export type CreateMessagePin = Assert<Pinned<typeof CreateMessage, GeneratedCreateMessage>>;
