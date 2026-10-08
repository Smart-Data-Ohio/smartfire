/**
 * Replies that name threads, judged against the removals the store remembers. The store keeps the
 * newest `MAX_REMOVED_THREADS` removals; a reply to a request sent before older ones were forgotten
 * can't tell a thread removed meanwhile from one that wasn't (`uncertainSince`), so it's asked
 * for again with a fresh `since`, which is past every forgotten removal.
 */
import { Effect, Predicate } from "effect";
import { thread } from "../api/thread-endpoints.ts";
import type { ThreadDetail } from "../gen/ThreadDetail.ts";
import { store } from "../store/store.ts";
import { uncertainSince } from "../store/threads.ts";

/** How many replies are asked for before giving up on telling (each one past 500 removals). */
export const MAX_SETTLE_ATTEMPTS = 3;

/**
 * The last reply and its request's `since` (the `removalCount` when it was sent), or `null` when
 * the reply can't be installed: every reply was uncertain, or `again` found the thread gone.
 */
export interface Settled<A> {
  readonly answer: A;
  readonly since: number | null;
}

/**
 * Runs `first`, then `again(previous)` while a thread the reply names (`threadIds`) is uncertain,
 * up to `MAX_SETTLE_ATTEMPTS` replies. `again` answers `null` when the thread is gone.
 */
export const settled = <A, E, R, E2, R2>(
  first: Effect.Effect<A, E, R>,
  again: (previous: A) => Effect.Effect<A | null, E2, R2>,
  threadIds: (answer: A) => readonly number[],
): Effect.Effect<Settled<A>, E | E2, R | R2> =>
  Effect.gen(function* () {
    let since = store.getState().removalCount;
    let answer: A = yield* first;

    for (let attempt = 1; ; attempt += 1) {
      const state = store.getState();

      if (!threadIds(answer).some((id) => uncertainSince(state, id, since))) {
        return { answer, since };
      }

      if (attempt >= MAX_SETTLE_ATTEMPTS) {
        return { answer, since: null };
      }

      since = store.getState().removalCount;
      const next = yield* again(answer);

      if (next === null) {
        return { answer, since: null };
      }

      answer = next;
    }
  });

/**
 * As `settled` for a reply that is the thread's detail (a write's, usually): it's asked for again
 * with `GET /threads/:id`, and a 404 there means the thread was removed.
 */
export const settledDetail = <E, R>(first: Effect.Effect<ThreadDetail, E, R>) =>
  settled(
    first,
    (previous) =>
      thread(previous.thread.id).pipe(
        Effect.catchIf(
          (error) => Predicate.isTagged(error, "NotFound"),
          () => Effect.succeed(null),
        ),
      ),
    (detail) => [detail.thread.id],
  );
