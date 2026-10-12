/**
 * A private update on `user` restores access after a recorded removal or a rejected pane load.
 * A reload while removed has no tombstone, but the denied pane still records its failed load.
 * The final batch state decides whether a grant survives a later removal in the same replay.
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
  before: Pick<State, "removedThreads" | "threadPanes">,
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
      const unreadable = wasRemoved || before.threadPanes[id]?.status === "error";

      if (unreadable && after.removedThreads[id] === undefined) {
        regained.add(id);
      }
    }
  }

  return { lost: [...lost], regained: [...regained] };
}
