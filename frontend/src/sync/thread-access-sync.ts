import type { SyncEvent } from "../gen/SyncEvent.ts";
import type { State } from "../store/state.ts";
import { isLatestThreadLoad } from "./thread-loads.ts";

export interface ThreadAccessChanges {
  /** Threads the viewer can no longer read. */
  readonly lost: readonly number[];
  readonly regained: readonly number[];
}

export function threadAccessChanges(
  before: Pick<State, "removedThreads" | "threadPanes">,
  after: Pick<State, "removedThreads">,
  events: readonly SyncEvent[],
  recovering: ReadonlyMap<number, number> = new Map(),
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
      const status = before.threadPanes[id]?.status;
      const unreadable = wasRemoved || status === "loading" || status === "error";
      const ticket = recovering.get(id);
      const pendingRecovery = ticket !== undefined && isLatestThreadLoad(id, ticket);

      if (
        unreadable &&
        after.removedThreads[id] === undefined &&
        (wasRemoved || !pendingRecovery)
      ) {
        regained.add(id);
      }
    }
  }

  return { lost: [...lost], regained: [...regained] };
}
