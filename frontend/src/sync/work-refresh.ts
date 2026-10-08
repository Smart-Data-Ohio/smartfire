import { Effect } from "effect";
import { thread } from "../api/thread-endpoints.ts";
import type { SyncEvent } from "../gen/SyncEvent.ts";
import type { WorkFacts } from "../gen/WorkFacts.ts";
import type { State } from "../store/state.ts";
import { mutations, store } from "../store/store.ts";

/**
 * The facts that version the work detail, as JSON: all but `messageCount`, which every reply
 * changes and the detail doesn't hold.
 */
export function workVersion(facts: WorkFacts | null | undefined): string {
  if (facts == null) return JSON.stringify(facts ?? null);
  const { messageCount: _, ...version } = facts;

  return JSON.stringify(version);
}

/** Work facts are the detail's version; the pane remembers them at its last GET. */
export function changedWorkPanes(
  state: State,
  events: readonly SyncEvent[],
  topics: readonly string[],
): number[] {
  const open = new Set(topics);

  return [
    ...new Set(
      events.flatMap((event) => {
        if (event.type !== "thread.updated" || !open.has(`thread:${event.data.id}`)) return [];
        const pane = state.threadPanes[event.data.id];

        return pane !== undefined &&
          workVersion(pane.workFacts) !== workVersion(state.threads[event.data.id]?.work)
          ? [event.data.id]
          : [];
      }),
    ),
  ];
}

/**
 * A second update during the GET wins; refetch until detail and live facts agree. The live
 * `messageCount` is kept: it isn't part of the work's version, so an older GET may carry less.
 */
export const refreshWorkPane = Effect.fn("work.refreshPane")(function* (threadId: number) {
  while (store.getState().threads[threadId] !== undefined) {
    const before = store.getState().threads[threadId];
    const since = store.getState().removalCount;
    const detail = yield* thread(threadId);
    const current = store.getState().threads[threadId];

    if (current === undefined) return;

    if (current === before) {
      mutations.loadThreadDetail(detail, since);

      return;
    }

    if (workVersion(current.work) === workVersion(before?.work)) {
      const work =
        detail.thread.work === null || current.work === null
          ? detail.thread.work
          : { ...detail.thread.work, messageCount: current.work.messageCount };

      mutations.loadThreadDetail({ ...detail, thread: { ...current, work } }, since);

      return;
    }

    if (workVersion(current.work) === workVersion(detail.thread.work)) {
      mutations.loadThreadDetail({ ...detail, thread: current }, since);

      return;
    }
  }
});
