/** The Slack import screens' words and the small rules behind them, as the classic pages have them. */
import type { SlackConnectionState } from "../../gen/SlackConnectionState.ts";
import type { SlackConversation } from "../../gen/SlackConversation.ts";
import type { SlackCounts } from "../../gen/SlackCounts.ts";
import type { SlackPeople } from "../../gen/SlackPeople.ts";
import type { SlackPlan } from "../../gen/SlackPlan.ts";
import type { SlackPreset } from "../../gen/SlackPreset.ts";
import type { SlackRunMode } from "../../gen/SlackRunMode.ts";
import type { SlackRunSummary } from "../../gen/SlackRunSummary.ts";
import type { StartSlackImport } from "../../gen/StartSlackImport.ts";
import { MAX_ISSUE_PAGES } from "./slack-pages.ts";

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

/**
 * Reads `?page=`; anything but a page number past the first is the default, and a page past
 * `MAX_ISSUE_PAGES` reads as that many.
 */
export function parseRunSearch(search: RawRunSearch): RunSearch {
  const page = Number(String(search.page ?? ""));

  return {
    page: Number.isSafeInteger(page) && page > 1 ? Math.min(page, MAX_ISSUE_PAGES) : undefined,
  };
}

/** A form value: blank is none (no date bound, no new secret). */
export function optional(value: string): string | null {
  const trimmed = value.trim();

  return trimmed === "" ? null : trimmed;
}

const YEAR_MS = 365 * 86_400_000;

/**
 * What a dry run's counts say about the past year. `"full"`: it read every message.
 * `"bounded"`: it had date bounds, so a zero or old count says nothing about recent use (an
 * oldest bound also hides new replies to older threads). `"undated"`: some channel was scanned
 * before last-message dates and catch-up starts were recorded (even one that counted nothing
 * may have read only a catch-up window), so only the member count is trusted.
 */
export type Coverage = "full" | "bounded" | "undated";

export function planCoverage(plan: SlackPlan): Coverage {
  if (plan.scannedFrom !== null || plan.scannedUntil !== null) return "bounded";

  // A row that starts skipped (its mapped room is gone) was never scanned, so has no dates.
  const undated = plan.conversations.some(
    ({ conversation, target }) => target !== "skip" && !conversation.dated,
  );

  return undated ? "undated" : "full";
}

/**
 * The time a plan measures "the past year" back from: when its dry run started, before it read
 * any channel, so a channel that posted after the run read it isn't called quiet.
 */
export function judgedAt(plan: SlackPlan, now: number): number {
  return plan.startedAt === null ? now : Math.min(now, Date.parse(plan.startedAt));
}

/** A conversation the dry run read only part of: an earlier import's catch-up start. */
export function partlyRead(conversation: SlackConversation): boolean {
  return conversation.scannedFrom !== null;
}

/**
 * A conversation nobody uses: no members, or (when the dry run read its whole history) no
 * messages in the past year. The plan leaves it unchecked. A conversation with messages but no
 * last-message date counts as active.
 */
export function inactive(
  conversation: SlackConversation,
  now: number,
  coverage: Coverage,
): boolean {
  if (conversation.members === 0) return true;

  if (coverage !== "full" || partlyRead(conversation)) return false;

  if (conversation.messages === 0) return true;

  if (conversation.lastMessageAt === null) return false;

  return now - Date.parse(conversation.lastMessageAt) > YEAR_MS;
}

/** Why the plan left a conversation unchecked, or null when it didn't. */
export function inactiveReason(
  conversation: SlackConversation,
  now: number,
  coverage: Coverage,
): string | null {
  if (!inactive(conversation, now, coverage)) return null;

  if (conversation.members === 0) return "No members";

  if (conversation.messages === 0) return "No messages";

  return "No messages in the past year";
}

/**
 * The plan's lines about the channels it left unchecked, or null when there is nothing to say.
 * `partly` is whether an earlier import left some channels read only from its catch-up start.
 */
export function inactiveHint(count: number, coverage: Coverage, partly: boolean): string | null {
  const channels = count === 1 ? "1 channel" : `${count} channels`;
  const start = count === 1 ? "starts" : "start";
  const anyway = "Check one to import it anyway.";

  const imported = partly
    ? "Channels an earlier import brought in aren't unchecked for going quiet: this dry run read only their newer messages."
    : null;

  const lines = (...parts: readonly (string | null)[]) => {
    const text = parts.filter((part) => part !== null).join(" ");

    return text === "" ? null : text;
  };

  switch (coverage) {
    case "full":
      return lines(
        count === 0
          ? null
          : `${channels} ${start} unchecked: no members, or no messages in the past year. ${anyway}`,
        imported,
      );

    case "bounded":
      return lines(
        count === 0 ? null : `${channels} with no members ${start} unchecked.`,
        "This dry run was limited to a date range, so it can't tell which channels went quiet.",
        count === 0 ? null : anyway,
      );

    case "undated":
      return lines(
        count === 0 ? null : `${channels} with no members ${start} unchecked. ${anyway}`,
        "This dry run is from before last-message dates were recorded. Start a new dry run to leave quiet channels unchecked.",
      );
  }
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
  undo: "Undo this import? Imported messages, reactions, pins, and rooms are removed. Your notes room and your own messages stay, as do records matched to existing accounts or rooms.",
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
