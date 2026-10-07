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
import { membership, observeMembership, placeId, replay } from "./freshness.ts";
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
  pagedStale,
} from "./paged-list.ts";
import { landsOver, mergeRevision, sameRecord } from "./revision.ts";
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
  readonly overlays: Readonly<
    Record<
      number,
      {
        readonly before: AgentApproval;
        readonly shown: AgentApproval;
        readonly confirmed: AgentApproval;
      }
    >
  >;
}

export const emptyApprovals: ApprovalsSlice = { items: {}, lists: {}, overlays: {} };

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
 * has since restarted (`generation`) merges records without replacing its list membership.
 */
export function landApprovalPage(
  state: State,
  key: ApprovalListKey,
  page: AgentApprovalPage,
  mode: "replace" | "more",
  generation?: number,
  ticket?: number,
): State {
  const current =
    pagedCurrent(approvalListOf(state, key), generation) &&
    (ticket === undefined || state.freshness.reads[ticket]?.list === `approvals:${key}`);

  let next = state;

  for (const approval of page.approvals) {
    next = applyApproval(next, approval, key);
  }

  if (!current) {
    const held = approvalListOf(next, key);

    const ids = held.ids.filter((id) => {
      const item = next.approvals.items[id];

      return item === undefined || belongsTo(key, item);
    });

    const filtered =
      ids.length === held.ids.length ? next : updateList(next, key, (list) => ({ ...list, ids }));

    return {
      ...filtered,
      freshness: membership(filtered.freshness, `approvals:${key}`, held.ids, ids),
      users: mergeUserList(filtered.users, page.users),
    };
  }

  const held = approvalListOf(next, key);

  const ids = page.approvals.flatMap((approval) =>
    belongsTo(key, next.approvals.items[approval.id] ?? approval) ? [approval.id] : [],
  );

  const loaded = pagedLanded(held, ids, page.nextCursor, mode);

  const replayed = replay(
    next.freshness,
    `approvals:${key}`,
    ticket ?? next.freshness.clock,
    loaded.ids,
    newestFirst,
  );

  return {
    ...next,
    freshness: membership(next.freshness, `approvals:${key}`, held.ids, replayed),
    users: mergeUserList(next.users, page.users),
    approvals: {
      ...next.approvals,
      lists: { ...next.approvals.lists, [key]: { ...loaded, ids: replayed } },
    },
  };
}

function placeApproval(state: State, approval: AgentApproval, sourceList?: ApprovalListKey): State {
  let freshness = state.freshness;

  const lists = eachPaged(state.approvals.lists, (list, key) => {
    if (key === sourceList) {
      return list;
    }

    const last = list.ids.at(-1);
    const belongs = belongsTo(key, approval);

    const inWindow =
      list.ids.includes(approval.id) ||
      list.nextCursor === null ||
      last === undefined ||
      newestFirst(approval.id, last) <= 0;

    const ids = placeId(
      list.ids,
      approval.id,
      belongs && inWindow && (list.status === "ready" || list.status === "loading"),
      newestFirst,
    );

    freshness = membership(freshness, `approvals:${key}`, list.ids, ids);

    if (ids === list.ids && (ids.includes(approval.id) || !belongs)) {
      freshness = observeMembership(freshness, `approvals:${key}`, approval.id, belongs);
    }

    return ids === list.ids ? list : { ...list, ids };
  });

  return {
    ...state,
    freshness,
    approvals: {
      ...state.approvals,
      items: { ...state.approvals.items, [approval.id]: approval },
      lists,
    },
  };
}

function dropOverlay(state: State, id: number): State {
  const { [id]: _overlay, ...overlays } = state.approvals.overlays;

  return { ...state, approvals: { ...state.approvals, overlays } };
}

/** All server copies, including cancelled/expired ones, compare updatedAt. */
export function applyApproval(
  state: State,
  approval: AgentApproval,
  sourceList?: ApprovalListKey,
): State {
  const overlay = state.approvals.overlays[approval.id];
  const stored = overlay?.confirmed ?? state.approvals.items[approval.id];

  if (!landsOver(stored, approval)) {
    return state;
  }

  if (overlay !== undefined && approval.updatedAt <= overlay.before.updatedAt) {
    const confirmed = mergeRevision(overlay.confirmed, approval);

    const shown = {
      ...confirmed,
      status: overlay.shown.status,
      decidedById: overlay.shown.decidedById,
      decidedAt: overlay.shown.decidedAt,
      decisionNote: overlay.shown.decisionNote,
      approvable: overlay.shown.approvable,
      deniable: overlay.shown.deniable,
    };

    const unchanged = sameRecord(overlay.shown, shown);

    return placeApproval(
      {
        ...state,
        approvals: {
          ...state.approvals,
          overlays: { ...state.approvals.overlays, [approval.id]: { ...overlay, confirmed } },
        },
      },
      unchanged ? overlay.shown : shown,
      sourceList,
    );
  }

  return placeApproval(dropOverlay(state, approval.id), approval, sourceList);
}

/**
 * Only decision fields overlay the confirmed request while the write is pending. The local
 * decision time is for display; updatedAt stays at the confirmed revision the write started from.
 */
export function showApproval(state: State, shown: AgentApproval): State {
  const before = state.approvals.items[shown.id];

  return before === undefined
    ? state
    : placeApproval(
        {
          ...state,
          approvals: {
            ...state.approvals,
            overlays: {
              ...state.approvals.overlays,
              [shown.id]: { before, shown, confirmed: before },
            },
          },
        },
        shown,
      );
}

/** A refusal restores only the local copy still shown, never a confirmed decision. */
export function rollbackApproval(
  state: State,
  shown: AgentApproval,
  _before: AgentApproval,
): State {
  const overlay = state.approvals.overlays[shown.id];

  return overlay?.shown !== shown
    ? state
    : placeApproval(dropOverlay(state, shown.id), overlay.confirmed);
}

/** The write reply confirms ties too; an outdated reply leaves the unresolved overlay. */
export function settleApproval(state: State, approval: AgentApproval): State {
  const overlay = state.approvals.overlays[approval.id];

  if (overlay !== undefined && landsOver(overlay.before, approval)) {
    state = placeApproval(dropOverlay(state, approval.id), overlay.confirmed);
  }

  return applyApproval(state, approval);
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
