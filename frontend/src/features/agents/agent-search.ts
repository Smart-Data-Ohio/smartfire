/**
 * The agent sections' URL queries, parsed in the route tree (the sections load lazily): the
 * approvals tab's `?status=` and the activity tab's `?outcome=`, each left out at "all".
 */
import type { AgentApprovalStatus } from "../../gen/AgentApprovalStatus.ts";
import type { AgentDeliveryOutcome } from "../../gen/AgentDeliveryOutcome.ts";

const STATUSES: readonly AgentApprovalStatus[] = [
  "pending",
  "approved",
  "denied",
  "cancelled",
  "expired",
];

const OUTCOMES: readonly AgentDeliveryOutcome[] = [
  "pending",
  "delivered",
  "acknowledged",
  "suppressed",
];

/** The approvals tab's query as the URL has it. */
export interface RawApprovalsSearch {
  readonly status?: unknown;
}

/** The approvals tab's query: the status filter, left out for every request. */
export interface ApprovalsSearch {
  readonly status?: AgentApprovalStatus | undefined;
}

/** Reads `?status=`; anything unknown lists every request. */
export function parseApprovalsSearch(search: RawApprovalsSearch): ApprovalsSearch {
  return { status: STATUSES.find((status) => status === search.status) };
}

/** The activity tab's query as the URL has it. */
export interface RawLedgerSearch {
  readonly outcome?: unknown;
}

/** The activity tab's query: the outcome filter, left out for every entry. */
export interface LedgerSearch {
  readonly outcome?: AgentDeliveryOutcome | undefined;
}

/** Reads `?outcome=`; anything unknown lists every entry. */
export function parseLedgerSearch(search: RawLedgerSearch): LedgerSearch {
  return { outcome: OUTCOMES.find((outcome) => outcome === search.outcome) };
}
