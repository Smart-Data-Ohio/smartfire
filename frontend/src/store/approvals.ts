/**
 * Agents' approval requests in the store (S4): every request seen, by id, and keyset-paged lists,
 * one per agent and status filter (`<agentId>:<filter>`, newest first). A request placed by a
 * decision (the viewer's, optimistic or confirmed, or `approval.updated`) moves into or out of
 * every loaded list of its agent at once, as the S3 lists do.
 *
 * A new request publishes no `approval.updated`: it reaches deciders as an
 * `agent_approval_request` activity item, and the lists go stale so a shown one refetches its first
 * page. `approval.updated` doesn't reach every decider either (see the contract), so the lists
 * also go stale after a resync and reload whenever they're shown again.
 */
import type { ActivityItem } from "../gen/ActivityItem.ts";
import type { AgentApproval } from "../gen/AgentApproval.ts";
import type { AgentApprovalPage } from "../gen/AgentApprovalPage.ts";
import type { AgentApprovalStatus } from "../gen/AgentApprovalStatus.ts";
import type { ApprovalDecision } from "../gen/ApprovalDecision.ts";
import type { ApprovalUpdated } from "../gen/ApprovalUpdated.ts";
import { mergeUserList } from "./ordering.ts";
import {
  eachPaged,
  emptyPagedList,
  type IdOrder,
  type PagedList,
  pagedCurrent,
  pagedFailed,
  pagedLanded,
  pagedLoading,
  pagedLoadingMore,
  pagedPlaced,
  pagedStale,
} from "./paged-list.ts";
import type { State } from "./state.ts";

/** A status to list, or every one. */
export type ApprovalFilter = AgentApprovalStatus | "all";

/** One agent's list in one filter. */
export type ApprovalListKey = `${number}:${ApprovalFilter}`;

export interface ApprovalsSlice {
  /** By approval id. */
  readonly items: Readonly<Record<number, AgentApproval>>;
  /** By `ApprovalListKey`. */
  readonly lists: Readonly<Record<string, PagedList>>;
}

export const emptyApprovals: ApprovalsSlice = { items: {}, lists: {} };

/** The key of `agentId`'s list in `filter`. */
export function approvalListKey(agentId: number, filter: ApprovalFilter): ApprovalListKey {
  return `${agentId}:${filter}`;
}

/** Whether an approval belongs in the list under `key`. */
function belongsTo(key: string, approval: AgentApproval): boolean {
  return (
    key === approvalListKey(approval.agentId, "all") ||
    key === approvalListKey(approval.agentId, approval.status)
  );
}

/** Newest first: the server pages by id, descending. */
const newestFirst: IdOrder = (left, right) => right - left;

/** One list, or an empty one never loaded. */
export function approvalListOf(state: State, key: ApprovalListKey): PagedList {
  return state.approvals.lists[key] ?? emptyPagedList;
}

function updateList(
  state: State,
  key: ApprovalListKey,
  change: (list: PagedList) => PagedList,
): State {
  return {
    ...state,
    approvals: {
      ...state.approvals,
      lists: { ...state.approvals.lists, [key]: change(approvalListOf(state, key)) },
    },
  };
}

export function setApprovalListLoading(state: State, key: ApprovalListKey, more: boolean): State {
  return updateList(state, key, more ? pagedLoadingMore : pagedLoading);
}

export function setApprovalListFailed(
  state: State,
  key: ApprovalListKey,
  error: string,
  generation?: number,
): State {
  return updateList(state, key, (list) => pagedFailed(list, error, generation));
}

/**
 * A page of one list landed: its requests and users join the store. A page from a load the list
 * has since restarted (`generation`) changes nothing.
 */
export function landApprovalPage(
  state: State,
  key: ApprovalListKey,
  page: AgentApprovalPage,
  mode: "replace" | "more",
  generation?: number,
): State {
  if (!pagedCurrent(approvalListOf(state, key), generation)) {
    return state;
  }

  const items = { ...state.approvals.items };

  for (const approval of page.approvals) {
    items[approval.id] = approval;
  }

  const lists = {
    ...state.approvals.lists,
    [key]: pagedLanded(
      approvalListOf(state, key),
      page.approvals.map((approval) => approval.id),
      page.nextCursor,
      mode,
    ),
  };

  return { ...state, users: mergeUserList(state.users, page.users), approvals: { items, lists } };
}

/**
 * A request as it is now (a decision's reply, `approval.updated`, an optimistic decision or its
 * rollback): stored, and moved into or out of every loaded list of its agent.
 */
export function applyApproval(state: State, approval: AgentApproval): State {
  const items = { ...state.approvals.items, [approval.id]: approval };

  const lists = eachPaged(state.approvals.lists, (list, key) =>
    pagedPlaced(list, approval.id, belongsTo(key, approval), newestFirst),
  );

  return { ...state, approvals: { items, lists } };
}

/** `approval.updated`: the request with its agent's and decider's users. */
export function applyApprovalUpdated(state: State, update: ApprovalUpdated): State {
  return applyApproval(
    { ...state, users: mergeUserList(state.users, update.users) },
    update.approval,
  );
}

/**
 * What the viewer's decision shows before the server answers: decided by them, now, with their
 * note, and no buttons.
 */
export function decidedLocally(
  approval: AgentApproval,
  decision: ApprovalDecision,
  deciderId: number,
  note: string | null,
  now: number,
): AgentApproval {
  return {
    ...approval,
    status: decision,
    decidedById: deciderId,
    decidedAt: new Date(now).toISOString(),
    decisionNote: note,
    approvable: false,
    deniable: false,
  };
}

/** Every loaded list reloads when next shown (a new request, or missed events). */
export function markApprovalsStale(state: State): State {
  const lists = eachPaged(state.approvals.lists, pagedStale);

  return lists === state.approvals.lists
    ? state
    : { ...state, approvals: { ...state.approvals, lists } };
}

/**
 * An activity item arrived: a new (pending) approval request publishes no `approval.updated`, so
 * the approvals lists go stale and a shown one refetches its first page. A decided request's item
 * changes nothing here; its `approval.updated` places it.
 */
export function approvalRequested(state: State, item: ActivityItem): State {
  return item.eventType === "agent_approval_request" && item.source.approvalStatus === "pending"
    ? markApprovalsStale(state)
    : state;
}
