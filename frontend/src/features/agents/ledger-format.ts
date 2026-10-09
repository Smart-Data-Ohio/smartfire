/**
 * How an agent's ledger entry reads (S4): a glyph and a sentence per type (a type this build
 * doesn't know reads as its own words), the outcome, the webhook's delivery, a GitHub or Fizzy
 * result and the hop count.
 */
import type { AgentDeliveryOutcome } from "../../gen/AgentDeliveryOutcome.ts";
import type { AgentLedgerEvent } from "../../gen/AgentLedgerEvent.ts";
import type { AgentWebhookStatus } from "../../gen/AgentWebhookStatus.ts";
import type { LedgerFilter } from "../../store/ledger.ts";
import type { IconName } from "../../ui/icons/icon.tsx";

/** The outcome filters, in the order the tabs show them. */
export const LEDGER_FILTERS: readonly { readonly value: LedgerFilter; readonly label: string }[] = [
  { value: "all", label: "All" },
  { value: "delivered", label: "Delivered" },
  { value: "acknowledged", label: "Acknowledged" },
  { value: "pending", label: "Pending" },
  { value: "suppressed", label: "Suppressed" },
];

/** The filter an `?outcome=` names; anything else lists every entry. */
export function ledgerFilterOf(raw: string | undefined): LedgerFilter {
  return LEDGER_FILTERS.find((filter) => filter.value === raw)?.value ?? "all";
}

/** "budget_threshold_crossed" as words: "Budget threshold crossed". */
function humanize(raw: string): string {
  const words = raw.replace(/[._]+/g, " ").trim();

  return words === "" ? "Event" : `${words.charAt(0).toUpperCase()}${words.slice(1)}`;
}

/** An entry's glyph; a type this build doesn't know gets a dotted circle. */
export function ledgerIcon(type: AgentLedgerEvent["eventType"]): IconName {
  switch (type) {
    case "mention":
      return "at";
    case "direct_message":
      return "dms";
    case "reply":
      return "reply";
    case "approval_decided":
      return "shield";
    case "github_action_completed":
      return "git-pull-request";
    case "fizzy_action_completed":
      return "boards";
    case "work_assigned":
      return "briefcase";
    case "work_unassigned":
      return "briefcase";
    case "work_handed_off":
      return "forward";
    case "slash_command":
      return "square-slash";
    case "posted":
      return "send";
    case "delivery_suppressed_rate_limit":
      return "gauge";
    case "delivery_suppressed_hop_limit":
      return "ban";
    case "delivery_suppressed_revoked":
      return "lock";
    default:
      return "circle-dot";
  }
}

/** Who did it and to whom, for the sentence. */
export interface LedgerNames {
  /** The person (or agent) behind it; `null` for none, or an account that's gone. */
  readonly actor: string | null;
  readonly agent: string;
}

/**
 * The entry as a sentence: "Maya mentioned Ember", "Ember finished a GitHub action", "Not
 * delivered: rate limit". A type this build doesn't know reads as its own words.
 */
export function ledgerSentence(type: AgentLedgerEvent["eventType"], names: LedgerNames): string {
  const actor = names.actor ?? "Someone";
  const { agent } = names;

  switch (type) {
    case "mention":
      return `${actor} mentioned ${agent}`;
    case "direct_message":
      return `${actor} sent ${agent} a direct message`;
    case "reply":
      return `${actor} replied to ${agent}`;
    case "approval_decided":
      return names.actor === null
        ? `An approval request from ${agent} was decided`
        : `${actor} decided an approval request`;
    case "github_action_completed":
      return `${agent} finished a GitHub action`;
    case "fizzy_action_completed":
      return `${agent} finished a Fizzy action`;
    case "work_assigned":
      return names.actor === null
        ? `${agent} was assigned work`
        : `${actor} assigned work to ${agent}`;
    case "work_unassigned":
      return names.actor === null
        ? `${agent} was taken off work`
        : `${actor} took ${agent} off work`;
    case "work_handed_off":
      return names.actor === null
        ? `Work was handed off to ${agent}`
        : `${actor} handed work off to ${agent}`;
    case "slash_command":
      return `${actor} ran a slash command for ${agent}`;
    case "posted":
      return `${agent} posted a message`;
    case "delivery_suppressed_rate_limit":
      return "Not delivered: rate limit";
    case "delivery_suppressed_hop_limit":
      return "Not delivered: too many agent hops";
    case "delivery_suppressed_revoked":
      return "Not delivered: access revoked";
    default:
      return humanize(type);
  }
}

/** An outcome chip's words; an unknown outcome reads as its own words. */
export function outcomeLabel(outcome: AgentDeliveryOutcome): string {
  switch (outcome) {
    case "pending":
      return "Pending";
    case "delivered":
      return "Delivered";
    case "acknowledged":
      return "Acknowledged";
    case "suppressed":
      return "Suppressed";
    default:
      return humanize(outcome);
  }
}

/** The webhook status's words. */
function webhookWord(status: AgentWebhookStatus): string {
  switch (status) {
    case "none":
      return "not sent";
    case "pending":
      return "pending";
    case "delivered":
      return "delivered";
    case "failed":
      return "failed";
    default:
      return humanize(status).toLowerCase();
  }
}

/**
 * "Webhook failed · 3 attempts · HTTP 502": `null` when the entry wasn't pushed to the agent's
 * webhook.
 */
export function webhookLine(event: AgentLedgerEvent): string | null {
  if (event.webhookStatus === "none") {
    return null;
  }

  const parts = [`Webhook ${webhookWord(event.webhookStatus)}`];

  if (event.webhookAttempts > 0) {
    parts.push(`${event.webhookAttempts} ${event.webhookAttempts === 1 ? "attempt" : "attempts"}`);
  }

  if (event.webhookLastError !== null) {
    parts.push(event.webhookLastError);
  }

  return parts.join(" · ");
}

/**
 * A GitHub or Fizzy result: "GitHub merge_pull_request: succeeded — Merged #318". `null` when the
 * entry recorded none.
 */
export function externalLine(event: AgentLedgerEvent): string | null {
  const { external } = event;

  if (external === null) {
    return null;
  }

  const service = event.eventType === "fizzy_action_completed" ? "Fizzy" : "GitHub";
  const head = `${service} ${external.action ?? ""}: ${external.status ?? ""}`;

  return external.message === null ? head : `${head} — ${external.message}`;
}

/** "Hop 2": how many agent-to-agent hops led here; `null` at 0. */
export function hopText(hop: number): string | null {
  return hop > 0 ? `Hop ${hop}` : null;
}

/** What an empty ledger says, per filter. */
export function ledgerEmptyText(filter: LedgerFilter): string {
  switch (filter) {
    case "all":
      return "Nothing has been delivered to this agent yet. Mentions, messages and actions show up here.";
    case "pending":
      return "Nothing is waiting to be delivered.";
    case "delivered":
      return "No delivered events.";
    case "acknowledged":
      return "No acknowledged events.";
    case "suppressed":
      return "Nothing was held back.";
    default:
      return "No events.";
  }
}
