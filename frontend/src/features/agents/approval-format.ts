/**
 * How an agent's approval request reads (S4): its status, who decided it and when, why the viewer
 * can't approve it, and who it acts as on GitHub or Fizzy.
 */
import type { AgentApproval } from "../../gen/AgentApproval.ts";
import type { AgentApprovalStatus } from "../../gen/AgentApprovalStatus.ts";
import { toMillis } from "../../lib/time.ts";
import type { ApprovalFilter } from "../../store/approvals.ts";
import { timeAgo } from "../threads/thread-format.ts";

const MINUTE_MS = 60_000;

const HOUR_MS = 60 * MINUTE_MS;

const DAY_MS = 24 * HOUR_MS;

const relative = new Intl.RelativeTimeFormat("en", { numeric: "always" });

/** The status filters, in the order the tabs show them. */
export const APPROVAL_FILTERS: readonly {
  readonly value: ApprovalFilter;
  readonly label: string;
}[] = [
  { value: "all", label: "All" },
  { value: "pending", label: "Pending" },
  { value: "approved", label: "Approved" },
  { value: "denied", label: "Denied" },
  { value: "expired", label: "Expired" },
  { value: "cancelled", label: "Cancelled" },
];

/** The filter a `?status=` names; anything else lists every request. */
export function approvalFilterOf(raw: string | undefined): ApprovalFilter {
  return APPROVAL_FILTERS.find((filter) => filter.value === raw)?.value ?? "all";
}

/**
 * The status to show: a pending request past its expiry reads expired, as the server settles it on
 * the next read (`effective_status`).
 */
export function shownStatus(approval: AgentApproval, now: number): AgentApprovalStatus {
  return approval.status === "pending" && toMillis(approval.expiresAt) <= now
    ? "expired"
    : approval.status;
}

/** A status chip's words. */
export function approvalStatusLabel(status: AgentApprovalStatus): string {
  switch (status) {
    case "pending":
      return "Pending";
    case "approved":
      return "Approved";
    case "denied":
      return "Denied";
    case "cancelled":
      return "Cancelled";
    case "expired":
      return "Expired";
  }
}

/** "in 2 days", "in 5 hours", "in 12 minutes", "in a moment" (never "tomorrow": it's a deadline). */
function timeUntil(timestamp: string, now: number): string {
  const left = toMillis(timestamp) - now;

  // From a day out it counts whole days, rounded: 47 hours left reads "in 2 days".
  if (left >= DAY_MS) {
    return relative.format(Math.round(left / DAY_MS), "day");
  }

  if (left >= HOUR_MS) {
    return relative.format(Math.round(left / HOUR_MS), "hour");
  }

  return left >= MINUTE_MS
    ? relative.format(Math.floor(left / MINUTE_MS), "minute")
    : "in a moment";
}

/**
 * The line under a request: when a pending one expires, who decided it and when ("by someone" when
 * the decider's account is gone), when it expired, or that the agent withdrew it.
 */
export function decisionLine(
  approval: AgentApproval,
  deciderName: string | null,
  now: number,
): string {
  const status = shownStatus(approval, now);
  const by = deciderName ?? "someone";
  const when = approval.decidedAt === null ? null : timeAgo(approval.decidedAt, now);

  switch (status) {
    case "pending":
      return `Asked ${timeAgo(approval.createdAt, now)} · expires ${timeUntil(approval.expiresAt, now)}`;
    case "approved":
      return when === null ? `Approved by ${by}` : `Approved by ${by} · ${when}`;
    case "denied":
      return when === null ? `Denied by ${by}` : `Denied by ${by} · ${when}`;
    case "cancelled":
      return "Cancelled by the agent";
    case "expired":
      return `Expired ${timeAgo(approval.expiresAt, now)} without a decision`;
  }
}

/** The service an admin-only action writes to: "GitHub" or "Fizzy". */
function serviceOf(action: string): string {
  return action.startsWith("fizzy.") ? "Fizzy" : "GitHub";
}

/**
 * Why there's no Approve button, when the viewer may only deny: "Only an administrator can approve
 * GitHub write actions." `null` when the viewer may approve, or may not decide at all.
 */
export function adminOnlyText(approval: AgentApproval): string | null {
  if (approval.approvable || !approval.deniable || !approval.adminOnly) {
    return null;
  }

  return `Only an administrator can approve ${serviceOf(approval.action)} write actions.`;
}

/** Who the action runs as: "Acts on GitHub as @ember-bot", "Acts on Fizzy as Ember"; else `null`. */
export function actsAsText(approval: AgentApproval): string | null {
  if (approval.githubLogin !== null) {
    return `Acts on GitHub as @${approval.githubLogin}`;
  }

  return approval.fizzyUserName === null ? null : `Acts on Fizzy as ${approval.fizzyUserName}`;
}

/** What a decision announces: "Approved: Merge PR #318 into main". */
export function decisionAnnouncement(decision: "approved" | "denied", summary: string): string {
  return `${decision === "approved" ? "Approved" : "Denied"}: ${summary}`;
}

/**
 * The one pending request that gets the beam: the newest still open. Only one surface owns the
 * beam at a time, so the other pending cards get a still violet edge.
 */
export function beamedApprovalId(approvals: readonly AgentApproval[], now: number): number | null {
  return approvals.find((approval) => shownStatus(approval, now) === "pending")?.id ?? null;
}

/** What an empty list says, per filter. */
export function approvalsEmptyText(filter: ApprovalFilter, agentName: string): string {
  switch (filter) {
    case "all":
      return `${agentName} hasn't asked for approval yet. Requests to deploy, merge or post on its behalf show up here.`;
    case "pending":
      return "Nothing is waiting for a decision.";
    case "approved":
      return "No approved requests.";
    case "denied":
      return "No denied requests.";
    case "expired":
      return "No requests expired without a decision.";
    case "cancelled":
      return "No requests were withdrawn.";
  }
}
