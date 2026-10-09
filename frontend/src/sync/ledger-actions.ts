/**
 * Agents' event ledgers as Effect programs (S4). Loads land in the store, failures too (a 403 as
 * the agent's ledger being closed to the viewer), so they never fail.
 */
import { Effect, Predicate } from "effect";
import * as api from "../api/agent-endpoints.ts";
import {
  type LedgerFilter,
  type LedgerListKey,
  ledgerListKey,
  ledgerListOf,
} from "../store/ledger.ts";
import { mutations, store } from "../store/store.ts";

/** The outcome to ask the server for: `null` for every entry. */
function outcomeOf(filter: LedgerFilter) {
  return filter === "all" ? null : filter;
}

function failLoad(key: LedgerListKey, agentId: number, generation: number) {
  return (error: { readonly _tag: string; readonly message: string }) =>
    Effect.sync(() =>
      mutations.setLedgerListFailed(
        key,
        agentId,
        error.message,
        Predicate.isTagged(error, "Forbidden"),
        generation,
      ),
    );
}

/** Loads (or reloads) an agent's first page in `filter`. */
export const load = Effect.fn("ledger.load")(function* (agentId: number, filter: LedgerFilter) {
  const key = ledgerListKey(agentId, filter);

  mutations.setLedgerListLoading(key, false);

  const { generation } = ledgerListOf(store.getState(), key);

  yield* api.agentLedger(agentId, outcomeOf(filter), null).pipe(
    Effect.tap((page) =>
      Effect.sync(() => mutations.landLedgerPage(key, agentId, page, "replace", generation)),
    ),
    Effect.catch(failLoad(key, agentId, generation)),
  );
});

/** Loads the next page, if there is one and none is on its way. */
export const loadMore = Effect.fn("ledger.loadMore")(function* (
  agentId: number,
  filter: LedgerFilter,
) {
  const key = ledgerListKey(agentId, filter);
  const list = ledgerListOf(store.getState(), key);

  if (list.nextCursor === null || list.loadingMore || list.status !== "ready") {
    return;
  }

  mutations.setLedgerListLoading(key, true);

  yield* api.agentLedger(agentId, outcomeOf(filter), list.nextCursor).pipe(
    Effect.tap((page) =>
      Effect.sync(() => mutations.landLedgerPage(key, agentId, page, "more", list.generation)),
    ),
    Effect.catch(failLoad(key, agentId, list.generation)),
  );
});
