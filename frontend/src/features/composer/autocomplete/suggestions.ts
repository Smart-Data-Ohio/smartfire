/**
 * What each trigger offers: people and icons from the server (debounced by the hook), commands
 * from a per-conversation cache, rooms from the sidebar the client already holds.
 */
import type { Icon } from "../../../gen/Icon.ts";
import type { SidebarRow } from "../../../gen/SidebarRow.ts";
import type { SlashCommand } from "../../../gen/SlashCommand.ts";
import type { User } from "../../../gen/User.ts";
import type { SidebarState } from "../../../store/state.ts";
import { composerActions } from "../../../sync/composer-actions.ts";
import { filterCommands } from "../slash.ts";
import { roomLink, type TriggerKind } from "./trigger.ts";

export type Suggestion =
  | {
      readonly kind: "mention";
      readonly key: string;
      readonly user: User;
      /** `<@123>`; older servers may return a name token or `null` (shown disabled). */
      readonly insert: string | null;
    }
  | { readonly kind: "emoji"; readonly key: string; readonly icon: Icon; readonly insert: string }
  | {
      readonly kind: "command";
      readonly key: string;
      readonly command: SlashCommand;
      readonly insert: string;
    }
  | {
      readonly kind: "room";
      readonly key: string;
      readonly row: SidebarRow;
      readonly insert: string;
    };

/** How many rows a menu shows. */
export const MAX_SUGGESTIONS = 8;

/** The menu's heading per trigger. */
export const TRIGGER_TITLES: Readonly<Record<TriggerKind, string>> = {
  mention: "People",
  emoji: "Emoji",
  command: "Commands",
  room: "Channels",
};

const commandCache = new Map<string, Promise<readonly SlashCommand[]>>();

/** The conversation's commands, fetched once per room or thread (a failed fetch is retried). */
export function loadCommands(
  roomId: number,
  threadId: number | null,
): Promise<readonly SlashCommand[]> {
  const key = `${roomId}:${threadId ?? ""}`;
  const cached = commandCache.get(key);

  if (cached !== undefined) {
    return cached;
  }

  const loading = composerActions.slashCommands(roomId, threadId);

  commandCache.set(key, loading);
  loading.catch(() => commandCache.delete(key));

  return loading;
}

/** Forgets cached command lists (tests, and a mock reset). */
export function clearCommandCache(): void {
  commandCache.clear();
}

export async function mentionSuggestions(
  roomId: number,
  query: string,
): Promise<readonly Suggestion[]> {
  const found = await composerActions.suggestUsers(roomId, query.trim());

  return found.slice(0, MAX_SUGGESTIONS).map((entry) => ({
    kind: "mention",
    key: `user-${entry.user.id}`,
    user: entry.user,
    insert: entry.mentionToken,
  }));
}

export async function emojiSuggestions(query: string): Promise<readonly Suggestion[]> {
  const found = await composerActions.suggestIcons(query.toLowerCase());

  return found.map((icon) => ({
    kind: "emoji",
    key: `icon-${icon.kind}-${icon.name}`,
    icon,
    insert: `:${icon.name}:`,
  }));
}

export function commandSuggestions(
  commands: readonly SlashCommand[],
  query: string,
): readonly Suggestion[] {
  return filterCommands(commands, query).map((command) => ({
    kind: "command",
    key: `command-${command.name}`,
    command,
    insert: `/${command.name}`,
  }));
}

/**
 * Rooms to link with `#`: the sidebar's channels (not DMs), names starting with the query first,
 * then names containing it, each alphabetically as the sidebar orders them.
 */
export function roomSuggestions(sidebar: SidebarState, query: string): readonly Suggestion[] {
  const needle = query.toLowerCase();
  const prefix: Suggestion[] = [];
  const inside: Suggestion[] = [];

  for (const roomId of sidebar.order) {
    const row = sidebar.rows[roomId];

    if (row === undefined || row.room.kind === "direct") {
      continue;
    }

    const name = row.displayName.toLowerCase();
    const bucket = name.startsWith(needle) ? prefix : name.includes(needle) ? inside : null;

    bucket?.push({
      kind: "room",
      key: `room-${roomId}`,
      row,
      insert: roomLink(row.displayName, roomId),
    });
  }

  return [...prefix, ...inside].slice(0, MAX_SUGGESTIONS);
}

/** Whether a row can be chosen. */
export function selectable(suggestion: Suggestion): boolean {
  return suggestion.kind !== "mention" || suggestion.insert !== null;
}
