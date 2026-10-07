/**
 * The hooks the agent pages read with. Each page loads when it's shown (a profile or directory
 * already held stays on screen while it reloads) and keeps up with `agent.status` after that; the
 * approvals and ledger tabs reload their first page each time they're shown too.
 */
import { useCallback, useEffect, useMemo, useRef } from "react";
import type { AgentApproval } from "../../gen/AgentApproval.ts";
import type { AgentDirectoryRow } from "../../gen/AgentDirectoryRow.ts";
import type { AgentLedgerEvent } from "../../gen/AgentLedgerEvent.ts";
import { type AgentProfileEntry, profileOf } from "../../store/agents.ts";
import { type ApprovalFilter, approvalListKey, approvalListOf } from "../../store/approvals.ts";
import {
  isLedgerForbidden,
  type LedgerFilter,
  ledgerListKey,
  ledgerListOf,
} from "../../store/ledger.ts";
import type { LoadStatus } from "../../store/model.ts";
import type { PagedList } from "../../store/paged-list.ts";
import { useStore } from "../../store/store.ts";
import { actions } from "../../sync/runtime.ts";
import type { PagedState } from "../destinations/paged-list.tsx";

/** The directory as a page reads it. */
export interface AgentDirectoryView {
  /** `idle`/`loading` (skeleton), `ready`, or `error` (nothing shown). */
  readonly status: LoadStatus;
  /** A reload's failure while rows stay shown, or the first load's. */
  readonly error: string | null;
  /** In the server's order: active agents first, then by name. */
  readonly rows: readonly AgentDirectoryRow[];
  readonly reload: () => void;
}

/** The agent directory, loaded each time it's shown and kept live by `agent.status`. */
export function useAgentDirectory(): AgentDirectoryView {
  const directory = useStore((state) => state.agents.directory);
  const byId = useStore((state) => state.agents.rows);
  const reload = useCallback(() => void actions.agents.loadDirectory(), []);

  useEffect(reload, [reload]);

  return useMemo(
    () => ({
      status: directory.status,
      error: directory.error,
      rows: directory.ids.flatMap((id) => byId[id] ?? []),
      reload,
    }),
    [directory, byId, reload],
  );
}

/** One agent's profile, loaded each time it's shown and kept live by `agent.status`. */
export function useAgentProfile(agentId: number): AgentProfileEntry & { reload: () => void } {
  const entry = useStore((state) => profileOf(state, agentId));
  const reload = useCallback(() => void actions.agents.loadProfile(agentId), [agentId]);

  useEffect(reload, [reload]);

  return useMemo(() => ({ ...entry, reload }), [entry, reload]);
}

/** A paged list as `PagedList` reads it, with its rows. */
export interface AgentPagedView<Row> extends PagedState {
  readonly rows: readonly Row[];
}

function pagedState(list: PagedList, loadMore: () => void, reload: () => void): PagedState {
  return {
    status: list.status,
    loadingMore: list.loadingMore,
    hasMore: list.nextCursor !== null,
    error: list.error,
    loadMore,
    reload,
  };
}

/**
 * An agent's approval requests in one status filter. The first page loads each time the list is
 * shown (`approval.updated` doesn't reach every decider, and a new request publishes none), and
 * again while shown when the lists go stale: a new request's activity item, or a resync.
 */
export function useAgentApprovals(
  agentId: number,
  filter: ApprovalFilter,
): AgentPagedView<AgentApproval> {
  const key = approvalListKey(agentId, filter);
  const list = useStore((state) => approvalListOf(state, key));
  const items = useStore((state) => state.approvals.items);

  const callbacks = useMemo(
    () => ({
      load: () => void actions.approvals.load(agentId, filter),
      more: () => void actions.approvals.loadMore(agentId, filter),
    }),
    [agentId, filter],
  );

  const stale = list.stale && list.status === "ready";
  const shownKey = useRef<string | null>(null);

  // Once each time this list is shown, then again whenever it goes stale while shown.
  useEffect(() => {
    if (shownKey.current !== key || stale) {
      shownKey.current = key;
      callbacks.load();
    }
  }, [key, stale, callbacks]);

  return useMemo(
    () => ({
      ...pagedState(list, callbacks.more, callbacks.load),
      rows: list.ids.flatMap((id) => items[id] ?? []),
    }),
    [list, items, callbacks],
  );
}

/** An agent's ledger in one outcome filter, as its tab reads it. */
export interface AgentLedgerView extends AgentPagedView<AgentLedgerEvent> {
  /** The server refused the ledger to the viewer (a 403): no Retry. */
  readonly forbidden: boolean;
}

/**
 * An agent's ledger in one outcome filter. It has no live updates, so its first page loads each
 * time it's shown.
 */
export function useAgentLedger(agentId: number, filter: LedgerFilter): AgentLedgerView {
  const key = ledgerListKey(agentId, filter);
  const list = useStore((state) => ledgerListOf(state, key));
  const items = useStore((state) => state.ledger.items);
  const forbidden = useStore((state) => isLedgerForbidden(state, agentId));

  const callbacks = useMemo(
    () => ({
      load: () => void actions.ledger.load(agentId, filter),
      more: () => void actions.ledger.loadMore(agentId, filter),
    }),
    [agentId, filter],
  );

  const { load } = callbacks;

  // Each time it's shown: there are no live updates to keep a held page current.
  useEffect(load, [load]);

  return useMemo(
    () => ({
      ...pagedState(list, callbacks.more, callbacks.load),
      rows: list.ids.flatMap((id) => items[id] ?? []),
      forbidden,
    }),
    [list, items, forbidden, callbacks],
  );
}
