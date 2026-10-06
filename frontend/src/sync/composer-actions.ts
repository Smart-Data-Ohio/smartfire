/**
 * The composer's request/response calls (autocomplete, slash commands, preview, scheduled
 * messages) as plain async functions, so React code never touches Effect. None of these land in
 * the store: their answers belong to the composer that asked. Failures reject with an
 * `ActionError` whose message is fit to show.
 */
import {
  cancelScheduledMessage,
  previewMessage,
  runSlashCommand,
  scheduledMessages,
  scheduleMessage,
  sendScheduledNow,
  slashCommands,
  suggestIcons,
  suggestUsers,
  updateScheduledMessage,
} from "../api/composer-endpoints.ts";
import type { CreateScheduledMessage } from "../gen/CreateScheduledMessage.ts";
import type { Icon } from "../gen/Icon.ts";
import type { ScheduledMessage } from "../gen/ScheduledMessage.ts";
import type { SlashCommand } from "../gen/SlashCommand.ts";
import type { SlashCommandResult } from "../gen/SlashCommandResult.ts";
import type { UpdateScheduledMessage } from "../gen/UpdateScheduledMessage.ts";
import type { UserSuggestion } from "../gen/UserSuggestion.ts";
import { runAction } from "./runtime.ts";

export const composerActions = {
  /** People to `@`-mention in the room, matching `query` anywhere in the name. */
  suggestUsers: async (roomId: number, query: string): Promise<readonly UserSuggestion[]> =>
    (await runAction(suggestUsers(roomId, query))).suggestions,

  /** Up to 8 emoji and workspace icons for `:query`. */
  suggestIcons: async (query: string): Promise<readonly Icon[]> =>
    (await runAction(suggestIcons(query))).icons,

  /** The commands this composer offers (built-ins, then the room's agent commands). */
  slashCommands: async (
    roomId: number,
    threadId: number | null,
  ): Promise<readonly SlashCommand[]> =>
    (await runAction(slashCommands(roomId, threadId))).commands,

  /** Runs `/name args` in the room (or a thread in it). */
  runSlashCommand: (
    roomId: number,
    text: string,
    threadId: number | null,
  ): Promise<SlashCommandResult> => runAction(runSlashCommand(roomId, text, threadId)),

  /** The server's sanitized rendering of a draft. */
  preview: async (roomId: number, markdown: string): Promise<string> =>
    (await runAction(previewMessage(roomId, markdown))).bodyHtml,

  /** The viewer's pending scheduled messages in the room, soonest first. */
  scheduled: async (roomId: number): Promise<readonly ScheduledMessage[]> =>
    (await runAction(scheduledMessages(roomId))).scheduledMessages,

  schedule: (roomId: number, body: CreateScheduledMessage): Promise<ScheduledMessage> =>
    runAction(scheduleMessage(roomId, body)),

  updateScheduled: (id: number, body: UpdateScheduledMessage): Promise<ScheduledMessage> =>
    runAction(updateScheduledMessage(id, body)),

  cancelScheduled: (id: number): Promise<void> => runAction(cancelScheduledMessage(id)),

  sendScheduledNow: (id: number): Promise<ScheduledMessage> => runAction(sendScheduledNow(id)),
};
