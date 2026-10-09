/**
 * The agent pages' words, as the classic directory row and profile have them. Literals the wire
 * passes through unchecked (a kind, capability or cap this client doesn't know) fall back to
 * their raw names rather than breaking.
 */
import type { AgentActivitySummary } from "../../gen/AgentActivitySummary.ts";
import type { AgentBudgetUsage } from "../../gen/AgentBudgetUsage.ts";
import type { AgentGrant } from "../../gen/AgentGrant.ts";
import type { AgentGrants } from "../../gen/AgentGrants.ts";
import { humanize } from "../people/agent-identity.ts";
import { timeAgo } from "../threads/thread-format.ts";

/** "read_messages" → "Read messages"; an unknown capability is its name with spaces. */
export function capabilityLabel(capability: string): string {
  switch (capability) {
    case "read_messages":
      return "Read messages";
    case "post_messages":
      return "Post messages";
    case "react":
      return "React";
    case "manage_threads":
      return "Manage threads";
    case "external_action":
      return "External actions";
    case "fizzy":
      return "Fizzy";
    case "dm_anyone":
      return "Message anyone directly";
    default:
      return humanize(capability);
  }
}

/**
 * The kind line: "Personal agent of Theo", "Workspace agent, managed by Riel", or with no owner
 * "Personal agent, no owner recorded". An unknown kind reads "Agent".
 */
export function kindDescription(kind: string, ownerName: string | null): string {
  if (kind === "personal") {
    return ownerName === null
      ? "Personal agent, no owner recorded"
      : `Personal agent of ${ownerName}`;
  }

  if (kind === "workspace") {
    return ownerName === null
      ? "Workspace agent, no owner recorded"
      : `Workspace agent, managed by ${ownerName}`;
  }

  return ownerName === null ? "Agent, no owner recorded" : `Agent, managed by ${ownerName}`;
}

/** "since 3 minutes ago", or `null` before the status first changed. */
export function sinceText(statusChangedAt: string | null, now: number): string | null {
  return statusChangedAt === null ? null : `since ${timeAgo(statusChangedAt, now)}`;
}

/** "last seen 2 hours ago", or "never seen". */
export function lastSeenText(lastSeenAt: string | null, now: number): string {
  return lastSeenAt === null ? "Never seen" : `Last seen ${timeAgo(lastSeenAt, now)}`;
}

/** One grant: "Post messages workspace-wide" or "React in 3 rooms". */
export function grantText(grant: AgentGrant): string {
  const name = capabilityLabel(grant.capability);

  if (grant.workspaceWide) {
    return `${name} workspace-wide`;
  }

  return `${name} in ${grant.roomCount} ${grant.roomCount === 1 ? "room" : "rooms"}`;
}

/** The grants summary's lines, or its one-line state when there's nothing to list. */
export function grantsLines(grants: AgentGrants): readonly string[] {
  if (grants.legacy) {
    return ["Legacy access (no grants recorded): reads, posts and reacts wherever it's a member"];
  }

  return grants.grants.length === 0 ? ["No active grants"] : grants.grants.map(grantText);
}

/** "12 delivered, 9 acknowledged, 4 posted, 1 suppressed". */
export function activitySentence(summary: AgentActivitySummary): string {
  return `${summary.delivered} delivered, ${summary.acknowledged} acknowledged, ${summary.posted} posted, ${summary.suppressed} suppressed`;
}

/** A cap's noun: "messages", "board posts", "external actions" (an unknown cap, with spaces). */
export function capNoun(cap: string): string {
  switch (cap) {
    case "messages":
      return "messages";
    case "board_posts":
      return "board posts";
    case "external_actions":
      return "external actions";
    default:
      return cap.replaceAll("_", " ");
  }
}

/** "4/50 messages", or "4 messages" with no cap set. */
export function budgetText(usage: AgentBudgetUsage): string {
  const noun = capNoun(usage.cap);

  return usage.limit === null ? `${usage.used} ${noun}` : `${usage.used}/${usage.limit} ${noun}`;
}

/** How full a cap is, 0 to 1, or `null` with no cap (or a zero cap). */
export function budgetFraction(usage: AgentBudgetUsage): number | null {
  return usage.limit === null || usage.limit <= 0 ? null : Math.min(1, usage.used / usage.limit);
}

/** "Anthropic · claude-agent-sdk", whichever of the two is set, or `null`. */
export function providerLine(provider: string | null, runtime: string | null): string | null {
  const parts = [provider, runtime].filter((part) => part !== null && part.trim() !== "");

  return parts.length === 0 ? null : parts.join(" · ");
}

/** Folds case and accents, so "zoe" finds "Zoë". */
export function foldText(text: string): string {
  return text
    .normalize("NFD")
    .replace(/\p{Diacritic}/gu, "")
    .toLowerCase()
    .trim();
}
