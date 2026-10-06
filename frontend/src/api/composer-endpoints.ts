/**
 * The S2 composer's endpoints: autocomplete, slash commands, preview and scheduled messages.
 * `#` room links complete from the sidebar, so they have no endpoint.
 */
import { Effect } from "effect";
import type { CreateScheduledMessage } from "../gen/CreateScheduledMessage.ts";
import type { IconList } from "../gen/IconList.ts";
import type { MessagePreview } from "../gen/MessagePreview.ts";
import type { ScheduledMessage } from "../gen/ScheduledMessage.ts";
import type { ScheduledMessageList } from "../gen/ScheduledMessageList.ts";
import type { SlashCommandList } from "../gen/SlashCommandList.ts";
import type { SlashCommandResult } from "../gen/SlashCommandResult.ts";
import type { UpdateScheduledMessage } from "../gen/UpdateScheduledMessage.ts";
import type { UserSuggestionList } from "../gen/UserSuggestionList.ts";
import { call, get, noContent } from "./call.ts";
import {
  IconList as IconListSchema,
  MessagePreview as MessagePreviewSchema,
  ScheduledMessageList as ScheduledMessageListSchema,
  ScheduledMessage as ScheduledMessageSchema,
  SlashCommandList as SlashCommandListSchema,
  SlashCommandResult as SlashCommandResultSchema,
  UserSuggestionList as UserSuggestionListSchema,
} from "./schema/composer.ts";
import { wire } from "./wire.ts";

/** `GET /autocomplete/users?roomId=&query=`: people to `@`-mention, members of the room first. */
export const suggestUsers = Effect.fn("api.suggestUsers")(function* (
  roomId: number,
  query: string,
) {
  return yield* call(
    get("/autocomplete/users", { roomId: String(roomId), query }),
    wire<UserSuggestionList>(UserSuggestionListSchema),
  );
});

/** `GET /autocomplete/icons?query=`: at most 8 emoji and workspace icons for `:`. */
export const suggestIcons = Effect.fn("api.suggestIcons")(function* (query: string) {
  return yield* call(get("/autocomplete/icons", { query }), wire<IconList>(IconListSchema));
});

/** `GET /icons`: every brand and workspace icon (no emoji), for the picker's custom tab. */
export const icons = Effect.fn("api.icons")(function* () {
  return yield* call(get("/icons"), wire<IconList>(IconListSchema));
});

/** `GET /rooms/:id/slash_commands?threadId=`: the commands this composer offers. */
export const slashCommands = Effect.fn("api.slashCommands")(function* (
  roomId: number,
  threadId: number | null,
) {
  return yield* call(
    get(
      `/rooms/${roomId}/slash_commands`,
      threadId === null ? undefined : { threadId: String(threadId) },
    ),
    wire<SlashCommandList>(SlashCommandListSchema),
  );
});

/** `POST /rooms/:id/slash_commands`: runs `/name args`; the result says what happened. */
export const runSlashCommand = Effect.fn("api.runSlashCommand")(function* (
  roomId: number,
  text: string,
  threadId: number | null,
) {
  return yield* call(
    { method: "POST", path: `/rooms/${roomId}/slash_commands`, body: { text, threadId } },
    wire<SlashCommandResult>(SlashCommandResultSchema),
  );
});

/** `POST /rooms/:id/messages/preview`: the server's rendering of a draft. */
export const previewMessage = Effect.fn("api.previewMessage")(function* (
  roomId: number,
  markdownSource: string,
) {
  return yield* call(
    { method: "POST", path: `/rooms/${roomId}/messages/preview`, body: { markdownSource } },
    wire<MessagePreview>(MessagePreviewSchema),
  );
});

/** `GET /scheduled_messages?roomId=`: the viewer's pending ones, soonest first. */
export const scheduledMessages = Effect.fn("api.scheduledMessages")(function* (
  roomId: number | null,
) {
  return yield* call(
    get("/scheduled_messages", roomId === null ? undefined : { roomId: String(roomId) }),
    wire<ScheduledMessageList>(ScheduledMessageListSchema),
  );
});

/** `POST /rooms/:id/scheduled_messages`. */
export const scheduleMessage = Effect.fn("api.scheduleMessage")(function* (
  roomId: number,
  body: CreateScheduledMessage,
) {
  return yield* call(
    { method: "POST", path: `/rooms/${roomId}/scheduled_messages`, body },
    wire<ScheduledMessage>(ScheduledMessageSchema),
  );
});

/** `PATCH /scheduled_messages/:id`: new text or time. */
export const updateScheduledMessage = Effect.fn("api.updateScheduledMessage")(function* (
  scheduledMessageId: number,
  body: UpdateScheduledMessage,
) {
  return yield* call(
    { method: "PATCH", path: `/scheduled_messages/${scheduledMessageId}`, body },
    wire<ScheduledMessage>(ScheduledMessageSchema),
  );
});

/** `DELETE /scheduled_messages/:id` (204): cancels it. */
export const cancelScheduledMessage = Effect.fn("api.cancelScheduledMessage")(function* (
  scheduledMessageId: number,
) {
  return yield* call(
    { method: "DELETE", path: `/scheduled_messages/${scheduledMessageId}` },
    noContent,
  );
});

/** `POST /scheduled_messages/:id/send_now`: posts it at once. */
export const sendScheduledNow = Effect.fn("api.sendScheduledNow")(function* (
  scheduledMessageId: number,
) {
  return yield* call(
    { method: "POST", path: `/scheduled_messages/${scheduledMessageId}/send_now` },
    wire<ScheduledMessage>(ScheduledMessageSchema),
  );
});
