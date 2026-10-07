/** The Slack import screens' words and the small rules behind them, as the classic pages have them. */
import type { SlackConnectionState } from "../../gen/SlackConnectionState.ts";
import type { SlackCounts } from "../../gen/SlackCounts.ts";
import type { SlackPeople } from "../../gen/SlackPeople.ts";
import type { SlackPlan } from "../../gen/SlackPlan.ts";
import type { SlackPreset } from "../../gen/SlackPreset.ts";
import type { SlackRunMode } from "../../gen/SlackRunMode.ts";
import type { SlackRunSummary } from "../../gen/SlackRunSummary.ts";
import type { StartSlackImport } from "../../gen/StartSlackImport.ts";

/** How often an active run's status is read again (the classic page's frame poll). */
export const POLL_MS = 5000;

/** A run's mode as the classic pages print it: "dry run" or "import". */
export function modeLabel(mode: SlackRunMode): string {
  return mode.replace("_", " ");
}

/** The setup page's line about the run in progress. */
export function activeRunSentence(run: SlackRunSummary): string {
  return `A ${run.kind} ${modeLabel(run.mode)} is ${run.status}.`;
}

/** The administrator's connection, as step 3 of the setup page says it. */
export function adminConnectionSentence(
  connection: SlackConnectionState,
  teamName: string | null,
): string {
  switch (connection.state) {
    case "connected":
      return teamName === null
        ? "Connected. Reconnect to refresh the grant."
        : `Connected to the ${teamName} Slack workspace. Reconnect to refresh the grant.`;
    case "rejected":
      return rejected(connection.reason);
    case "none":
      return "Connect with your Slack account. The workspace import runs as you: it reads every public channel plus the private channels you are in.";
  }
}

/** A person's own connection, as the personal page says it. */
export function personalConnectionSentence(connection: SlackConnectionState): string {
  switch (connection.state) {
    case "connected":
      return "Connected. Reconnect to refresh the grant.";
    case "rejected":
      return rejected(connection.reason);
    case "none":
      return "Connect with your Slack account to preview and import your history.";
  }
}

function rejected(reason: string | null): string {
  return `Slack rejected the connection${reason === null ? "" : ` (${reason})`}. Reconnect below.`;
}

/** The status section's people line. */
export function peopleLine(people: SlackPeople): string {
  return `${people.total} found (${people.matched} matched, ${people.placeholders} placeholders, ${people.deactivated} deactivated, ${people.bots} bots)`;
}

/** The status section's three count lines: rooms, messages and extras. */
export function countLines(counts: SlackCounts) {
  return {
    rooms: `${counts.roomsCreated} created, ${counts.roomsMerged} merged`,
    messages: `${counts.messages} messages, ${counts.replies} replies in ${counts.threads} threads`,
    extras: `${counts.reactions} reactions, ${counts.pins} pins, ${counts.filesLinked} files linked, ${counts.skipped} skipped`,
  };
}

/**
 * React keys for rows the wire gives no id (issues, samples): each row's content, numbered when
 * the same content repeats. They hold steady while rows are only added at the end, as an issues
 * page grows.
 */
export function keyed<T>(
  rows: readonly T[],
  content: (row: T) => string,
): { readonly key: string; readonly row: T }[] {
  const seen = new Map<string, number>();

  return rows.map((row) => {
    const base = content(row);
    const repeat = seen.get(base) ?? 0;

    seen.set(base, repeat + 1);

    return { key: repeat === 0 ? base : `${base}\u0000${repeat}`, row };
  });
}

/** A run page's URL query as the URL has it (`?page=N` from a classic link). */
export interface RawRunSearch {
  readonly page?: unknown;
}

/** A run page's query: how many pages of issues to show at first, left out at one. */
export interface RunSearch {
  readonly page?: number | undefined;
}

/** Reads `?page=`; anything but a page number past the first is the default. */
export function parseRunSearch(search: RawRunSearch): RunSearch {
  const page = Number(String(search.page ?? ""));

  return { page: Number.isSafeInteger(page) && page > 1 ? page : undefined };
}

/** A form value: blank is none (no date bound, no new secret). */
export function optional(value: string): string | null {
  const trimmed = value.trim();

  return trimmed === "" ? null : trimmed;
}

/** The plan form as the classic form posts it: the checked conversations in plan order, every row's target. */
export function importBody(
  plan: SlackPlan,
  checked: ReadonlySet<string>,
  targets: Readonly<Record<string, string>>,
  preset: SlackPreset,
  days: { readonly oldest: string; readonly latest: string },
): StartSlackImport {
  return {
    conversationIds: plan.conversations.flatMap(({ conversation }) =>
      checked.has(conversation.id) ? [conversation.id] : [],
    ),
    roomTargets: Object.fromEntries(
      plan.conversations.map(({ conversation, target }) => [
        conversation.id,
        targets[conversation.id] ?? target,
      ]),
    ),
    preset,
    oldest: optional(days.oldest),
    latest: optional(days.latest),
  };
}

/** The classic pages' confirmations. */
export const CONFIRM = {
  cancelWorkspace:
    "Cancel this run? It stops at the next step; already-imported records stay until undone.",
  cancelPersonal: "Cancel this run? It stops at the next step.",
  undo: "Undo this import? Every room, message, reaction, and pin it created is removed. Records it only matched to existing accounts or rooms stay.",
  catchUp:
    "Run a catch-up import with the same conversations and targets? Already-imported objects are skipped, so it is safe to repeat.",
  testImport:
    "Start a test import of the checked conversations (recent messages only)? It can be undone.",
  fullImport:
    "Start the full import of the checked conversations (all messages)? It can be undone.",
  importChecked: "Import the checked conversations? It can be undone.",
  disconnectAdmin: "Disconnect Slack? Imports need a connected account.",
  disconnectPersonal: "Disconnect Slack? Your imported history stays.",
  removeCredentials:
    "Remove the Slack app credentials and disconnect every member? Past run history stays.",
} as const;
