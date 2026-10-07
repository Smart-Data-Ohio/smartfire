/**
 * Agents' approval requests as Effect programs (S4). Loads land in the store (failures as the
 * list's error) and never fail. A decision shows at once and lands the server's reply; if the
 * server refuses, it comes back as it was, unless an `approval.updated` brought a newer copy
 * meanwhile. A 422, 404 or 409 reloads All and any held Pending list to pick up missed decisions.
 * An older successful reply leaves its overlay visible and reloads All once in the background.
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
  captureApprovalRead,
  decidedLocally,
} from "../store/approvals.ts";
import { mutations, store } from "../store/store.ts";
import { withRead } from "./freshness.ts";

/** The status to ask the server for: `null` for every one. */
function statusOf(filter: ApprovalFilter) {
  return filter === "all" ? null : filter;
}

function failLoad(key: ApprovalListKey, generation: number) {
  return (error: { readonly message: string }) =>
    Effect.sync(() => mutations.setApprovalListFailed(key, error.message, generation));
}

function needsRefetch(error: ApiFailure): boolean {
  return (
    Predicate.isTagged(error, "Validation") ||
    Predicate.isTagged(error, "NotFound") ||
    Predicate.isTagged(error, "Conflict")
  );
}

/** Loads (or reloads) an agent's first page in `filter`. */
export const load = Effect.fn("approvals.load")(function* (
  agentId: number,
  filter: ApprovalFilter,
) {
  const key = approvalListKey(agentId, filter);

  yield* withRead(
    (ticket) =>
      Effect.gen(function* () {
        mutations.setApprovalListLoading(key, false);

        const state = store.getState();
        const { generation } = approvalListOf(state, key);
        const read = captureApprovalRead(state);

        yield* api.agentApprovals(agentId, statusOf(filter), null).pipe(
          Effect.tap((page) =>
            Effect.sync(() =>
              mutations.landApprovalPage(key, page, "replace", generation, ticket, read),
            ),
          ),
          Effect.catch(failLoad(key, generation)),
        );
      }),
    `approvals:${key}`,
  );
});

/** Loads the next page, if there is one and none is on its way. */
export const loadMore = Effect.fn("approvals.loadMore")(function* (
  agentId: number,
  filter: ApprovalFilter,
) {
  const key = approvalListKey(agentId, filter);
  const state = store.getState();
  const list = approvalListOf(state, key);
  const read = captureApprovalRead(state);

  if (list.nextCursor === null || list.loadingMore || list.status !== "ready") {
    return;
  }

  mutations.setApprovalListLoading(key, true);

  yield* withRead(
    (ticket) =>
      api.agentApprovals(agentId, statusOf(filter), list.nextCursor).pipe(
        Effect.tap((page) =>
          Effect.sync(() =>
            mutations.landApprovalPage(key, page, "more", list.generation, ticket, read),
          ),
        ),
        Effect.catch(failLoad(key, list.generation)),
      ),
    `approvals:${key}`,
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
  const read = captureApprovalRead(state);
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
    if (shown !== undefined && before !== undefined) {
      mutations.rollbackApproval(shown, before);
    }
  };

  const decided = yield* api
    .decideApproval(approvalId, note === null ? { decision } : { decision, note })
    .pipe(
      Effect.tap((decided) => Effect.sync(() => mutations.settleApproval(decided, read))),
      Effect.tapError((error) =>
        Effect.gen(function* () {
          rollBack();

          if (needsRefetch(error)) {
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

  if (store.getState().approvals.overlays[approvalId] !== undefined) {
    yield* Effect.forkDetach(load(decided.agentId, "all"));
  }

  return decided;
});
