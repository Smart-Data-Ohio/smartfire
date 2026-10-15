/**
 * The composer's request/response calls (autocomplete, slash commands, preview) as plain async
 * functions, so React code never touches Effect. None of these land in the store: their answers
 * belong to the composer that asked. Failures reject with an `ActionError` whose message is fit
 * to show. Scheduled messages live in the store: `actions.scheduled` in runtime.ts.
 */
import { Schema } from "effect";
import {
  previewMessage,
  runSlashCommand,
  slashCommands,
  suggestIcons,
  suggestUsers,
} from "../api/composer-endpoints.ts";
import { stickerCatalog } from "../api/sticker-endpoints.ts";
import type { CommandValues } from "../gen/CommandValues.ts";
import type { Icon } from "../gen/Icon.ts";
import type { SlashCommand } from "../gen/SlashCommand.ts";
import type { SlashCommandResult } from "../gen/SlashCommandResult.ts";
import type { StickerCatalog } from "../gen/StickerCatalog.ts";
import type { UserSuggestion } from "../gen/UserSuggestion.ts";
import { runAction } from "./runtime.ts";

const DraftMention = Schema.Struct({
  start: Schema.Int,
  text: Schema.String,
  token: Schema.String,
});

export type DraftMention = typeof DraftMention.Type;

/** Parse draft metadata at the storage boundary; callers check ranges against the draft text. */
export const decodeDraftMentions = Schema.decodeUnknownSync(
  Schema.fromJsonString(Schema.Array(DraftMention)),
);

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
    options: CommandValues | null = null,
  ): Promise<SlashCommandResult> => runAction(runSlashCommand(roomId, text, threadId, options)),

  /** The workspace's stickers, by name, and how many it may hold. */
  stickerCatalog: (): Promise<StickerCatalog> => runAction(stickerCatalog()),

  /** The server's sanitized rendering of a draft. */
  preview: async (roomId: number, markdown: string): Promise<string> =>
    (await runAction(previewMessage(roomId, markdown))).bodyHtml,
};
