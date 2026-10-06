/**
 * The viewer's memberships in a room's active threads, fetched when the room opens. Without them
 * the store can't tell a followed thread from a stranger's, so `thread.unread` events for it
 * would be dropped and reply indicators couldn't show the unread dot. The Threads pane's own list
 * (`roomThreads`) is left alone.
 */
import { Effect } from "effect";
import * as api from "../api/thread-endpoints.ts";
import type { ThreadList } from "../gen/ThreadList.ts";
import { mergeUserList } from "../store/ordering.ts";
import type { State } from "../store/state.ts";
import { store } from "../store/store.ts";

/**
 * Adds what the store doesn't know yet: thread records it lacks (a held record may be newer) and
 * memberships it has never seen (a held one came from the pane or an event, so it's fresher).
 */
export function mergeThreadSummaries(state: State, list: ThreadList): State {
  const fresh = list.threads.filter((summary) => state.threads[summary.thread.id] === undefined);

  const unknown = list.threads.filter((summary) => !(summary.thread.id in state.threadMemberships));

  if (fresh.length === 0 && unknown.length === 0) {
    return state;
  }

  return {
    ...state,
    users: mergeUserList(state.users, list.users),
    threads: {
      ...state.threads,
      ...Object.fromEntries(fresh.map((summary) => [summary.thread.id, summary.thread])),
    },
    threadMemberships: {
      ...state.threadMemberships,
      ...Object.fromEntries(unknown.map((summary) => [summary.thread.id, summary.membership])),
    },
  };
}

/** Loads the room's active threads and merges the viewer's memberships; failures are quiet. */
export const prefetchMemberships = Effect.fn("threads.prefetchMemberships")(function* (
  roomId: number,
) {
  yield* api.threads(roomId, "active").pipe(
    Effect.tap((list) =>
      Effect.sync(() => store.setState((state) => mergeThreadSummaries(state, list), true)),
    ),
    Effect.catch((error) => Effect.logDebug("thread prefetch failed", error.message)),
  );
});
