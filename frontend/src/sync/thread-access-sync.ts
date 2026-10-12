/**
 * What a batch of sync events did to the viewer's access to private threads, for the engine's
 * follow-ups: a `thread.removed` on the viewer's own `user` topic took a thread away, and a private
 * `thread.updated` there for a thread this page couldn't read gave one back (the viewer was
 * added again).
 *
 * A removal and a grant can share one batch (a reconnect replays both), so "removed" means removed
 * before the batch or earlier in it, and "lifted" is read from the state after the whole batch.
 * A thread also counts as unreadable when this page never held it or its pane failed to load:
 * its removal then predates the page (or its replay), so no removal is remembered for it, yet
 * the server refused the pane's subscription all the same.
 */
import type { SyncEvent } from "../gen/SyncEvent.ts";
import type { State } from "../store/state.ts";

export interface ThreadAccessChanges {
  /** Threads the viewer can no longer read. */
  readonly lost: readonly number[];
  /** Threads this page couldn't read before the batch that the viewer can read again. */
  readonly regained: readonly number[];
}

export function threadAccessChanges(
  before: Pick<State, "removedThreads" | "threads" | "threadPanes">,
  after: Pick<State, "removedThreads">,
  events: readonly SyncEvent[],
): ThreadAccessChanges {
  const lost = new Set<number>();
  const regained = new Set<number>();

  for (const event of events) {
    if (event.topic !== "user") continue;

    if (event.type === "thread.removed") {
      lost.add(event.data.threadId);
    }

    if (event.type === "thread.updated" && event.data.visibility === "private") {
      const id = event.data.id;
      const wasRemoved = before.removedThreads[id] !== undefined || lost.has(id);
      const unreadable =
        wasRemoved ||
        before.threads[id] === undefined ||
        before.threadPanes[id]?.status === "error";

      if (unreadable && after.removedThreads[id] === undefined) {
        regained.add(id);
      }
    }
  }

  return { lost: [...lost], regained: [...regained] };
}
