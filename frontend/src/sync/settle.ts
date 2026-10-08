/**
 * Replies that name threads, judged against the removals the store remembers. The store keeps the
 * newest `MAX_REMOVED_THREADS` removals; a reply to a request sent before older ones were forgotten
 * can't tell a thread removed meanwhile from one that wasn't (`uncertainSince`), so it's asked
 * for again with a fresh `since`, which is past every forgotten removal.
 */
import { Effect, Predicate, Result } from "effect";
import { thread } from "../api/thread-endpoints.ts";
import type { ThreadDetail } from "../gen/ThreadDetail.ts";
import { store } from "../store/store.ts";
import { uncertainSince } from "../store/threads.ts";

/**
 * What a pane says when asking again found no thread: it may have been deleted, or the viewer's
 * access may be gone for now, so the pane keeps its Try again.
 */
export const UNAVAILABLE =
  "This thread isn't available. It may have been deleted, or you may no longer have access.";

/** How many replies are asked for before giving up on telling (each one past 500 removals). */
/** When removals kept outrunning the thread's replies (see `settled`); Try again asks afresh. */
export const UNSETTLED = "This thread couldn't be loaded. Try again.";

export const MAX_SETTLE_ATTEMPTS = 3;

/**
 * Installed; gone (asking again answered 404, which a removal and a lost room membership both
 * do, so callers treat it as "not available", not as a deletion); or every reply was uncertain.
 */
export type SettleOutcome = "installed" | "gone" | "unsettled";

/** The last reply, and what became of it. */
export interface Settled<A> {
  readonly answer: A;
  readonly outcome: SettleOutcome;
}

/**
 * Runs `first`, then `again(previous)` while a thread the reply names (`threadIds`) is uncertain,
 * up to `MAX_SETTLE_ATTEMPTS` replies; `again` answers `null` when the thread is gone. A reply
 * that isn't uncertain is installed (`install`, with its request's `since`) in the same step as
 * the check, so no removal can land between them.
 */
export const settled = <A, E, R, E2, R2>(
  first: Effect.Effect<A, E, R>,
  again: (previous: A) => Effect.Effect<A | null, E2, R2>,
  threadIds: (answer: A) => readonly number[],
  install: (answer: A, since: number) => void,
): Effect.Effect<Settled<A>, E | E2, R | R2> =>
  Effect.gen(function* () {
    let since = store.getState().removalCount;
    let answer: A = yield* first;

    for (let attempt = 1; ; attempt += 1) {
      const state = store.getState();

      if (!threadIds(answer).some((id) => uncertainSince(state, id, since))) {
        install(answer, since);

        return { answer, outcome: "installed" };
      }

      if (attempt >= MAX_SETTLE_ATTEMPTS) {
        return { answer, outcome: "unsettled" };
      }

      since = store.getState().removalCount;
      const next = yield* again(answer);

      if (next === null) {
        return { answer, outcome: "gone" };
      }

      answer = next;
    }
  });

/** `GET /threads/:id`, asked again by `settled`: `null` on a 404 (removed, or out of reach). */
export const refetchThread = (threadId: number) =>
  thread(threadId).pipe(
    Effect.catchIf(
      (error) => Predicate.isTagged(error, "NotFound"),
      () => Effect.succeed(null),
    ),
  );

/** As `settled` for a reply that is the thread's detail (a write's, usually). */
export const settledDetail = <E, R>(
  first: Effect.Effect<ThreadDetail, E, R>,
  install: (detail: ThreadDetail, since: number) => void,
) =>
  settled(
    first,
    (previous) => refetchThread(previous.thread.id),
    (detail) => [detail.thread.id],
    install,
  );

/** What the pane says when its header didn't settle: the failure, a 404 on asking again, or giving up. */
export const paneProblem = <A>(
  detail: Result.Result<Settled<A>, { readonly message: string }>,
): string | null =>
  Result.isFailure(detail)
    ? detail.failure.message
    : detail.success.outcome === "gone"
      ? UNAVAILABLE
      : detail.success.outcome === "unsettled"
        ? UNSETTLED
        : null;
