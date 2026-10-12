/**
 * What a batch of sync events did to the viewer's access to private threads, for the engine's
 * follow-ups: a `thread.removed` on the viewer's own `user` topic took a thread away, and a private
 * `thread.updated` there that lifted its removal gave one back (the viewer was added again).
 *
 * A removal and a grant can share one batch (a reconnect replays both), so "removed" means removed
 * before the batch or earlier in it, and "lifted" is read from the state after the whole batch.
 */
import type { SyncEvent } from "../gen/SyncEvent.ts";
import type { State } from "../store/state.ts";

export interface ThreadAccessChanges {
  /** Threads the viewer can no longer read. */
  readonly lost: readonly number[];
  /** Threads removed before the batch, or earlier in it, that the viewer can read again. */
  readonly regained: readonly number[];
}

export function threadAccessChanges(
  before: Pick<State, "removedThreads">,
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

      if (wasRemoved && after.removedThreads[id] === undefined) {
        regained.add(id);
      }
    }
  }

  return { lost: [...lost], regained: [...regained] };
}
