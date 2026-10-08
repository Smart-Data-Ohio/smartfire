/**
 * Agents' event ledgers in the store (S4): every entry seen, by id, and keyset-paged lists, one per
 * agent and outcome filter (`<agentId>:<filter>`, newest first). The ledger has no live updates,
 * so a list just reloads its first page each time it's shown. Only administrators and the agent's
 * owner may read it: a 403 marks the agent's ledger `forbidden`, which shows no Retry.
 */
import type { AgentDeliveryOutcome } from "../gen/AgentDeliveryOutcome.ts";
import type { AgentLedgerEvent } from "../gen/AgentLedgerEvent.ts";
import type { AgentLedgerPage } from "../gen/AgentLedgerPage.ts";
import { mergeUserList } from "./ordering.ts";
import {
  emptyPagedList,
  type PagedList,
  pagedCurrent,
  pagedFailed,
  pagedLanded,
  pagedLoading,
  pagedLoadingMore,
} from "./paged-list.ts";
import type { State } from "./state.ts";

/** An outcome to list, or every entry. */
export type LedgerFilter = AgentDeliveryOutcome | "all";

/** One agent's ledger in one filter. */
export type LedgerListKey = `${number}:${LedgerFilter}`;

export interface LedgerSlice {
  /** By ledger entry id. */
  readonly items: Readonly<Record<number, AgentLedgerEvent>>;
  /** By `LedgerListKey`. */
  readonly lists: Readonly<Record<string, PagedList>>;
  /** The agents whose ledger the viewer may not read (a 403). */
  readonly forbidden: readonly number[];
}

export const emptyLedger: LedgerSlice = { items: {}, lists: {}, forbidden: [] };

/** The key of `agentId`'s ledger in `filter`. */
export function ledgerListKey(agentId: number, filter: LedgerFilter): LedgerListKey {
  return `${agentId}:${filter}`;
}

/** One list, or an empty one never loaded. */
export function ledgerListOf(state: State, key: LedgerListKey): PagedList {
  return state.ledger.lists[key] ?? emptyPagedList;
}

/** Whether the server refused `agentId`'s ledger to the viewer. */
export function isLedgerForbidden(state: State, agentId: number): boolean {
  return state.ledger.forbidden.includes(agentId);
}

function updateList(
  state: State,
  key: LedgerListKey,
  change: (list: PagedList) => PagedList,
): State {
  return {
    ...state,
    ledger: {
      ...state.ledger,
      lists: { ...state.ledger.lists, [key]: change(ledgerListOf(state, key)) },
    },
  };
}

export function setLedgerListLoading(state: State, key: LedgerListKey, more: boolean): State {
  return updateList(state, key, more ? pagedLoadingMore : pagedLoading);
}

/** A load failed; `forbidden` (a 403) also marks the agent's ledger closed to the viewer. */
export function setLedgerListFailed(
  state: State,
  key: LedgerListKey,
  agentId: number,
  error: string,
  forbidden: boolean,
  generation?: number,
): State {
  const failed = updateList(state, key, (list) => pagedFailed(list, error, generation));

  if (!forbidden || isLedgerForbidden(failed, agentId)) {
    return failed;
  }

  return {
    ...failed,
    ledger: { ...failed.ledger, forbidden: [...failed.ledger.forbidden, agentId] },
  };
}

/**
 * A page of one list landed: its entries and users join the store, and the agent's ledger is open
 * to the viewer again. A page from a load the list has since restarted changes nothing.
 */
export function landLedgerPage(
  state: State,
  key: LedgerListKey,
  agentId: number,
  page: AgentLedgerPage,
  mode: "replace" | "more",
  generation?: number,
): State {
  if (!pagedCurrent(ledgerListOf(state, key), generation)) {
    return state;
  }

  const items = { ...state.ledger.items };

  // Ledger rows are append-only and have no mutable record revision to compare.
  for (const event of page.events) {
    items[event.id] = event;
  }

  const lists = {
    ...state.ledger.lists,
    [key]: pagedLanded(
      ledgerListOf(state, key),
      page.events.map((event) => event.id),
      page.nextCursor,
      mode,
    ),
  };

  return {
    ...state,
    users: mergeUserList(state.users, page.users),
    ledger: { items, lists, forbidden: state.ledger.forbidden.filter((id) => id !== agentId) },
  };
}
