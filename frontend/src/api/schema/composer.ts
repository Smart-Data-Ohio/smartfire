import { Schema } from "effect";
import type { CreateScheduledMessage as GeneratedCreateScheduledMessage } from "../../gen/CreateScheduledMessage.ts";
import type { IconList as GeneratedIconList } from "../../gen/IconList.ts";
import type { MessagePreview as GeneratedMessagePreview } from "../../gen/MessagePreview.ts";
import type { PreviewMessage as GeneratedPreviewMessage } from "../../gen/PreviewMessage.ts";
import type { RunSlashCommand as GeneratedRunSlashCommand } from "../../gen/RunSlashCommand.ts";
import type { ScheduledMessage as GeneratedScheduledMessage } from "../../gen/ScheduledMessage.ts";
import type { ScheduledMessageFilter as GeneratedScheduledMessageFilter } from "../../gen/ScheduledMessageFilter.ts";
import type { ScheduledMessageList as GeneratedScheduledMessageList } from "../../gen/ScheduledMessageList.ts";
import type { ScheduledMessageRemoved as GeneratedScheduledMessageRemoved } from "../../gen/ScheduledMessageRemoved.ts";
import type { ScheduledMessageState as GeneratedScheduledMessageState } from "../../gen/ScheduledMessageState.ts";
import type { SlashCommand as GeneratedSlashCommand } from "../../gen/SlashCommand.ts";
import type { SlashCommandList as GeneratedSlashCommandList } from "../../gen/SlashCommandList.ts";
import type { SlashCommandResult as GeneratedSlashCommandResult } from "../../gen/SlashCommandResult.ts";
import type { UpdateScheduledMessage as GeneratedUpdateScheduledMessage } from "../../gen/UpdateScheduledMessage.ts";
import type { UserSuggestion as GeneratedUserSuggestion } from "../../gen/UserSuggestion.ts";
import type { UserSuggestionList as GeneratedUserSuggestionList } from "../../gen/UserSuggestionList.ts";
import { ConversationName } from "./conversation.ts";
import { Icon } from "./icon.ts";
import { MessageId, RoomId, ScheduledMessageId, ThreadId } from "./ids.ts";
import type { Assert, Pinned } from "./pin.ts";
import { Timestamp } from "./time.ts";
import { User } from "./user.ts";

/** Insert the stable ID token; older servers can return `null` for unavailable mentions. */
export const UserSuggestion = Schema.Struct({
  user: User,
  mentionToken: Schema.NullOr(Schema.String),
});

export type UserSuggestion = typeof UserSuggestion.Type;

export type UserSuggestionPin = Assert<Pinned<typeof UserSuggestion, GeneratedUserSuggestion>>;

/** `GET /api/v1/autocomplete/users?roomId=&query=`: at most 20. */
export const UserSuggestionList = Schema.Struct({ suggestions: Schema.Array(UserSuggestion) });

export type UserSuggestionList = typeof UserSuggestionList.Type;

export type UserSuggestionListPin = Assert<
  Pinned<typeof UserSuggestionList, GeneratedUserSuggestionList>
>;

export { Icon, IconKind } from "./icon.ts";

/** `GET /api/v1/autocomplete/icons?query=` (at most 8) and `GET /api/v1/icons` (all non-emoji). */
export const IconList = Schema.Struct({ icons: Schema.Array(Icon) });

export type IconList = typeof IconList.Type;

export type IconListPin = Assert<Pinned<typeof IconList, GeneratedIconList>>;

export const SlashCommand = Schema.Struct({
  name: Schema.String,
  description: Schema.String,
  argHint: Schema.String,
  takesArguments: Schema.Boolean,
  agentName: Schema.NullOr(Schema.String),
});

export type SlashCommand = typeof SlashCommand.Type;

export type SlashCommandPin = Assert<Pinned<typeof SlashCommand, GeneratedSlashCommand>>;

/** `GET /api/v1/rooms/:id/slash_commands?threadId=`: built-ins, then the room's agent commands. */
export const SlashCommandList = Schema.Struct({ commands: Schema.Array(SlashCommand) });

export type SlashCommandList = typeof SlashCommandList.Type;

export type SlashCommandListPin = Assert<
  Pinned<typeof SlashCommandList, GeneratedSlashCommandList>
>;

/** The body of `POST /api/v1/rooms/:id/slash_commands`. */
export const RunSlashCommand = Schema.Struct({
  text: Schema.String,
  threadId: Schema.NullOr(ThreadId),
});

export type RunSlashCommand = typeof RunSlashCommand.Type;

export type RunSlashCommandPin = Assert<Pinned<typeof RunSlashCommand, GeneratedRunSlashCommand>>;

/** What a slash command did, tagged by `status`. */
export const SlashCommandResult = Schema.Union([
  Schema.Struct({
    status: Schema.Literal("posted"),
    messageId: MessageId,
    notice: Schema.NullOr(Schema.String),
  }),
  Schema.Struct({ status: Schema.Literal("ephemeral"), message: Schema.String }),
  Schema.Struct({ status: Schema.Literal("error"), message: Schema.String }),
  Schema.Struct({ status: Schema.Literal("open_url"), url: Schema.String }),
  Schema.Struct({ status: Schema.Literal("open_poll") }),
  Schema.Struct({
    status: Schema.Literal("start_huddle"),
    roomId: RoomId,
    roomName: Schema.String,
  }),
]);

export type SlashCommandResult = typeof SlashCommandResult.Type;

export type SlashCommandResultPin = Assert<
  Pinned<typeof SlashCommandResult, GeneratedSlashCommandResult>
>;

/** The body of `POST /api/v1/rooms/:id/messages/preview`. */
export const PreviewMessage = Schema.Struct({ markdownSource: Schema.String });

export type PreviewMessagePin = Assert<Pinned<typeof PreviewMessage, GeneratedPreviewMessage>>;

export const MessagePreview = Schema.Struct({ bodyHtml: Schema.String });

export type MessagePreview = typeof MessagePreview.Type;

export type MessagePreviewPin = Assert<Pinned<typeof MessagePreview, GeneratedMessagePreview>>;

export const ScheduledMessageState = Schema.Literals(["pending", "sending", "sent", "dropped"]);

export type ScheduledMessageState = typeof ScheduledMessageState.Type;

export type ScheduledMessageStatePin = Assert<
  Pinned<typeof ScheduledMessageState, GeneratedScheduledMessageState>
>;

/**
 * A message to send later; text only. A pending one that isn't `sendable` is stranded: it will
 * be dropped when due unless access comes back.
 */
export const ScheduledMessage = Schema.Struct({
  id: ScheduledMessageId,
  roomId: RoomId,
  threadId: Schema.NullOr(ThreadId),
  replyToMessageId: Schema.NullOr(MessageId),
  markdownSource: Schema.String,
  /** The source as preview text, spoilers redacted by the server. Shown instead of the source. */
  excerpt: Schema.String,
  sendAt: Timestamp,
  state: ScheduledMessageState,
  sendable: Schema.Boolean,
  sentAt: Schema.NullOr(Timestamp),
  sentMessageId: Schema.NullOr(MessageId),
  droppedAt: Schema.NullOr(Timestamp),
  dropReason: Schema.NullOr(Schema.String),
  createdAt: Timestamp,
});

export type ScheduledMessage = typeof ScheduledMessage.Type;

export type ScheduledMessagePin = Assert<
  Pinned<typeof ScheduledMessage, GeneratedScheduledMessage>
>;

/** The body of `POST /api/v1/rooms/:id/scheduled_messages`; `sendAt` must be in the future. */
export const CreateScheduledMessage = Schema.Struct({
  markdownSource: Schema.String,
  sendAt: Timestamp,
  threadId: Schema.NullOr(ThreadId),
  replyToMessageId: Schema.NullOr(MessageId),
});

export type CreateScheduledMessage = typeof CreateScheduledMessage.Type;

export type CreateScheduledMessagePin = Assert<
  Pinned<typeof CreateScheduledMessage, GeneratedCreateScheduledMessage>
>;

/** The body of `PATCH /api/v1/scheduled_messages/:id`; a field left out keeps its value. */
export const UpdateScheduledMessage = Schema.Struct({
  markdownSource: Schema.optionalKey(Schema.String),
  sendAt: Schema.optionalKey(Timestamp),
});

export type UpdateScheduledMessage = typeof UpdateScheduledMessage.Type;

export type UpdateScheduledMessagePin = Assert<
  Pinned<typeof UpdateScheduledMessage, GeneratedUpdateScheduledMessage>
>;

export const ScheduledMessageFilter = Schema.Literals(["pending", "past"]);

export type ScheduledMessageFilter = typeof ScheduledMessageFilter.Type;

export type ScheduledMessageFilterPin = Assert<
  Pinned<typeof ScheduledMessageFilter, GeneratedScheduledMessageFilter>
>;

/**
 * `GET /api/v1/scheduled_messages?status=&roomId=&before=`: pending soonest first, or past most
 * recent first; 50 a page. `nextCursor` is opaque (it encodes `sendAt` and `id`); pass it back
 * as `before`.
 */
export const ScheduledMessageList = Schema.Struct({
  scheduledMessages: Schema.Array(ScheduledMessage),
  conversations: Schema.Array(ConversationName),
  nextCursor: Schema.NullOr(Schema.String),
});

export type ScheduledMessageList = typeof ScheduledMessageList.Type;

export type ScheduledMessageListPin = Assert<
  Pinned<typeof ScheduledMessageList, GeneratedScheduledMessageList>
>;

/** The `scheduled.removed` event: cancelled in another tab. */
export const ScheduledMessageRemoved = Schema.Struct({ id: ScheduledMessageId, roomId: RoomId });

export type ScheduledMessageRemoved = typeof ScheduledMessageRemoved.Type;

export type ScheduledMessageRemovedPin = Assert<
  Pinned<typeof ScheduledMessageRemoved, GeneratedScheduledMessageRemoved>
>;
