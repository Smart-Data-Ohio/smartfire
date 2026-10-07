/**
 * Agents' approval requests as Effect programs (S4). Loads land in the store (failures as the
 * list's error) and never fail. A decision shows at once and lands the server's reply; if the
 * server refuses (403 for an admin-only action, 422 once it isn't pending), it comes back as it
 * was, unless an `approval.updated` brought a newer copy meanwhile. A refusal saying it is no
 * longer pending reloads All and any held Pending list to pick up missed decisions.
 */
import { Effect, Predicate } from "effect";
import * as api from "../api/agent-endpoints.ts";
import type { ApiFailure } from "../api/errors.ts";
import type { ApprovalDecision } from "../gen/ApprovalDecision.ts";
import {
  type ApprovalFilter,
  type ApprovalListKey,
  approvalListKey,
  approvalListOf,
  decidedLocally,
} from "../store/approvals.ts";
import { mutations, store } from "../store/store.ts";

/** The status to ask the server for: `null` for every one. */
function statusOf(filter: ApprovalFilter) {
  return filter === "all" ? null : filter;
}

function failLoad(key: ApprovalListKey, generation: number) {
  return (error: { readonly message: string }) =>
    Effect.sync(() => mutations.setApprovalListFailed(key, error.message, generation));
}

function noLongerPending(error: ApiFailure): boolean {
  if (Predicate.isTagged(error, "NotFound") || Predicate.isTagged(error, "Conflict")) {
    return true;
  }

  return (
    Predicate.isTagged(error, "Validation") &&
    [error.message, ...Object.values(error.fields).flat()].some((message) =>
      /already decided|expired/i.test(message),
    )
  );
}

/** Loads (or reloads) an agent's first page in `filter`. */
export const load = Effect.fn("approvals.load")(function* (
  agentId: number,
  filter: ApprovalFilter,
) {
  const key = approvalListKey(agentId, filter);

  mutations.setApprovalListLoading(key, false);

  const { generation } = approvalListOf(store.getState(), key);

  yield* api.agentApprovals(agentId, statusOf(filter), null).pipe(
    Effect.tap((page) =>
      Effect.sync(() => mutations.landApprovalPage(key, page, "replace", generation)),
    ),
    Effect.catch(failLoad(key, generation)),
  );
});

/** Loads the next page, if there is one and none is on its way. */
export const loadMore = Effect.fn("approvals.loadMore")(function* (
  agentId: number,
  filter: ApprovalFilter,
) {
  const key = approvalListKey(agentId, filter);
  const list = approvalListOf(store.getState(), key);

  if (list.nextCursor === null || list.loadingMore || list.status !== "ready") {
    return;
  }

  mutations.setApprovalListLoading(key, true);

  yield* api.agentApprovals(agentId, statusOf(filter), list.nextCursor).pipe(
    Effect.tap((page) =>
      Effect.sync(() => mutations.landApprovalPage(key, page, "more", list.generation)),
    ),
    Effect.catch(failLoad(key, list.generation)),
  );
});

/**
 * Approves or denies a request, at once; answers the server's copy. Fails with the server's error
 * after putting the request back.
 */
export const decide = Effect.fn("approvals.decide")(function* (
  approvalId: number,
  decision: ApprovalDecision,
  note: string | null,
) {
  const state = store.getState();
  const before = state.approvals.items[approvalId];
  const deciderId = state.me?.user.id ?? state.boot?.user.id ?? null;

  const shown =
    before === undefined || deciderId === null
      ? undefined
      : decidedLocally(before, decision, deciderId, note, Date.now());

  if (shown !== undefined) {
    mutations.showApproval(shown);
  }

  /** Back as it was, unless something newer replaced what this decision showed. */
  const rollBack = () => {
    if (shown !== undefined) {
      mutations.rollbackApproval(shown);
    }
  };

  const decided = yield* api
    .decideApproval(approvalId, note === null ? { decision } : { decision, note })
    .pipe(
      Effect.tapError((error) =>
        Effect.gen(function* () {
          rollBack();

          if (noLongerPending(error)) {
            mutations.markApprovalsStale();

            if (before !== undefined) {
              yield* load(before.agentId, "all");

              if (state.approvals.lists[approvalListKey(before.agentId, "pending")] !== undefined) {
                yield* load(before.agentId, "pending");
              }
            }
          }
        }),
      ),
      // Interrupted, so the outcome is unknown: put it back, and reload to be sure.
      Effect.onInterrupt(() =>
        Effect.sync(() => {
          rollBack();
          mutations.markApprovalsStale();
        }),
      ),
    );

  mutations.applyApproval(decided);

  return decided;
});
