import { Schema } from "effect";
import type { CreateMessage as GeneratedCreateMessage } from "../../gen/CreateMessage.ts";
import type { MessageDTO as GeneratedMessageDTO } from "../../gen/MessageDTO.ts";
import type { MessagePage as GeneratedMessagePage } from "../../gen/MessagePage.ts";
import type { MessageRead as GeneratedMessageRead } from "../../gen/MessageRead.ts";
import type { MessageRemoved as GeneratedMessageRemoved } from "../../gen/MessageRemoved.ts";
import type { MessageSound as GeneratedMessageSound } from "../../gen/MessageSound.ts";
import type { MessageSource as GeneratedMessageSource } from "../../gen/MessageSource.ts";
import type { SavedMark as GeneratedSavedMark } from "../../gen/SavedMark.ts";
import type { UpdateMessage as GeneratedUpdateMessage } from "../../gen/UpdateMessage.ts";
import { AgentStep } from "./agents.ts";
import { Attachment } from "./attachment.ts";
import { MessageCard, Poll } from "./cards.ts";
import { ConversationName } from "./conversation.ts";
import { MessageId, RoomId, SavedItemId, ThreadId, UserId } from "./ids.ts";
import type { Assert, Pinned } from "./pin.ts";
import { Boost, Reaction } from "./reaction.ts";
import { ThreadIndicator } from "./thread-parts.ts";
import { Timestamp } from "./time.ts";
import { User } from "./user.ts";

export const MessageSound = Schema.Struct({
  name: Schema.String,
  url: Schema.String,
  presentation: Schema.Union([
    Schema.Struct({ kind: Schema.Literal("text"), text: Schema.String }),
    Schema.Struct({
      kind: Schema.Literal("image"),
      url: Schema.String,
      width: Schema.Int,
      height: Schema.Int,
    }),
  ]),
});

export type MessageSoundPin = Assert<Pinned<typeof MessageSound, GeneratedMessageSound>>;

/**
 * A message on a room's timeline or in a thread. `bodyHtml` is already sanitized. Viewer
 * independent: "you reacted" is the viewer's id in `reactorIds`, edit and delete rights follow
 * from `creatorId`, `systemNote` and the viewer's role.
 */
export const MessageDTO = Schema.Struct({
  id: MessageId,
  roomId: RoomId,
  threadId: Schema.NullOr(ThreadId),
  creatorId: UserId,
  clientMessageId: Schema.String,
  bodyHtml: Schema.String,
  sound: Schema.NullOr(MessageSound),
  markdownSource: Schema.NullOr(Schema.String),
  systemNote: Schema.Boolean,
  action: Schema.Boolean,
  streaming: Schema.Boolean,
  embedsSuppressed: Schema.Boolean,
  replyToMessageId: Schema.NullOr(MessageId),
  replyTargetDeletedAt: Schema.optionalKey(Schema.NullOr(Timestamp)),
  forwardedFromMessageId: Schema.NullOr(MessageId),
  forwardedAt: Schema.NullOr(Timestamp),
  forwardNote: Schema.NullOr(Schema.String),
  editedAt: Schema.NullOr(Timestamp),
  attachment: Schema.NullOr(Attachment),
  reactions: Schema.Array(Reaction),
  boosts: Schema.Array(Boost),
  pinned: Schema.Boolean,
  thread: Schema.NullOr(ThreadIndicator),
  poll: Schema.NullOr(Poll),
  cards: Schema.Array(MessageCard),
  cardsAsOf: Timestamp,
  /** An agent's steps; merged per step by its own `updatedAt` (see `AgentStep`). */
  steps: Schema.Array(AgentStep),
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

/** One of a page's messages the viewer saved, and the saved item (to unsave it). */
export const SavedMark = Schema.Struct({ messageId: MessageId, savedItemId: SavedItemId });

export type SavedMark = typeof SavedMark.Type;

export type SavedMarkPin = Assert<Pinned<typeof SavedMark, GeneratedSavedMark>>;

/**
 * `GET /api/v1/rooms/:id/messages` (or `/threads/:id/messages`): up to 40 messages, oldest
 * first, with their authors and the viewer's saves among them. `before`/`after` are the ids to
 * page from next, `null` at the start / the present.
 */
export const MessagePage = Schema.Struct({
  messages: Schema.Array(MessageDTO),
  users: Schema.Array(User),
  before: Schema.NullOr(MessageId),
  after: Schema.NullOr(MessageId),
  saved: Schema.Array(SavedMark),
});

export type MessagePage = typeof MessagePage.Type;

export type MessagePagePin = Assert<Pinned<typeof MessagePage, GeneratedMessagePage>>;

/**
 * `GET /api/v1/messages/:id`: one message the viewer can reach, with its author and indicator
 * repliers, its conversation and the viewer's save. A 404 for anything else, so a forward whose
 * origin is out of reach shows as "Forwarded" only.
 */
export const MessageRead = Schema.Struct({
  message: MessageDTO,
  users: Schema.Array(User),
  conversation: ConversationName,
  saved: Schema.NullOr(SavedMark),
});

export type MessageRead = typeof MessageRead.Type;

export type MessageReadPin = Assert<Pinned<typeof MessageRead, GeneratedMessageRead>>;

/** The body of `POST /api/v1/rooms/:id/messages`. Idempotent on `clientMessageId`. */
export const CreateMessage = Schema.Struct({
  clientMessageId: Schema.String,
  markdownSource: Schema.String,
  replyToMessageId: Schema.NullOr(MessageId),
  replyNotifyAuthor: Schema.NullOr(Schema.Boolean),
  attachmentSignedId: Schema.NullOr(Schema.String),
  driveFileIds: Schema.optionalKey(Schema.Array(Schema.String)),
});

export type CreateMessage = typeof CreateMessage.Type;

export type CreateMessagePin = Assert<Pinned<typeof CreateMessage, GeneratedCreateMessage>>;

/** The body of `PATCH /api/v1/messages/:id`. Always carries the Markdown. */
export const UpdateMessage = Schema.Struct({
  markdownSource: Schema.String,
  removeDriveFileIds: Schema.optionalKey(Schema.Array(Schema.String)),
});

export type UpdateMessage = typeof UpdateMessage.Type;

export type UpdateMessagePin = Assert<Pinned<typeof UpdateMessage, GeneratedUpdateMessage>>;

/** `GET /api/v1/messages/:id/source`: the edit box's starting Markdown. */
export const MessageSource = Schema.Struct({ messageId: MessageId, markdownSource: Schema.String });

export type MessageSource = typeof MessageSource.Type;

export type MessageSourcePin = Assert<Pinned<typeof MessageSource, GeneratedMessageSource>>;
