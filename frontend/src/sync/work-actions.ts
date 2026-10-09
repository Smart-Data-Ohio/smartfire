/**
 * Work tracking's actions as Effect programs (S4). The work list loads per filter; a reply from
 * an older load cannot replace its list membership. A status or owner change shows on the
 * thread at once and rolls back if the server refuses, unless something changed the facts
 * meanwhile. Changes to one thread go one at a time. Every write lands the `ThreadDetail` the
 * server answers, and while one is on its way the pane doesn't refetch on its `thread.updated`.
 */
import { Effect } from "effect";
import { thread as fetchThread } from "../api/thread-endpoints.ts";
import * as api from "../api/work-endpoints.ts";
import type { CreateWorkHandoff } from "../gen/CreateWorkHandoff.ts";
import type { ThreadDetail } from "../gen/ThreadDetail.ts";
import type { UpdateWork } from "../gen/UpdateWork.ts";
import type { WorkFacts } from "../gen/WorkFacts.ts";
import type { WorkFilter } from "../gen/WorkFilter.ts";
import type { WorkStatus } from "../gen/WorkStatus.ts";
import { mutations, store } from "../store/store.ts";
import { captureWorkRead, optimisticFacts, workDetailStale, workListOf } from "../store/work.ts";
import { keyedSerial } from "./serial.ts";
import { settledDetail } from "./settle.ts";

/** Loads (or reloads) a filter's list; a failure lands as the list's error. */
export const loadList = Effect.fn("work.loadList")(function* (filter: WorkFilter) {
  mutations.setWorkListLoading(filter);

  const state = store.getState();
  const { generation } = workListOf(state, filter);
  const read = captureWorkRead(state);

  yield* api.workList(filter).pipe(
    Effect.tap((list) => Effect.sync(() => mutations.landWorkList(filter, list, generation, read))),
    Effect.catch((error) =>
      Effect.sync(() => mutations.setWorkListFailed(filter, error.message, generation)),
    ),
  );
});

/** Threads with a refetch on its way, so a burst of events asks once. */
const refreshing = new Set<number>();

/**
 * Refetches the thread's detail because live facts moved past the held ones. Another attempt
 * needs a newer confirmed revision or a tracking change during the read. No progress or a
 * failure keeps what the pane shows and sets its error; the next change can try again.
 */
export const refresh = Effect.fn("work.refresh")(function* (threadId: number) {
  if (refreshing.has(threadId)) {
    return;
  }

  refreshing.add(threadId);

  yield* Effect.gen(function* () {
    while (true) {
      const before = store.getState();
      const overlay = before.work.overlays[threadId];
      const held = overlay === undefined ? before.threads[threadId]?.work : overlay.confirmed;
      const read = captureWorkRead(before);

      const detail = yield* fetchThread(threadId);

      mutations.loadThreadDetail(detail, read.since, read);

      const after = store.getState();

      if (!workDetailStale(after, threadId)) {
        return;
      }

      const revision = after.threads[threadId]?.work?.updatedAt;
      const previous = held?.updatedAt;

      const progressed =
        revision !== previous &&
        (revision === undefined || previous === undefined || revision > previous);

      if (!progressed) {
        mutations.setThreadPaneError(
          threadId,
          "The work detail could not catch up to its latest revision",
        );

        return;
      }
    }
  }).pipe(
    Effect.catch((error) =>
      Effect.sync(() => mutations.setThreadPaneError(threadId, error.message)),
    ),
    Effect.ensuring(Effect.sync(() => refreshing.delete(threadId))),
  );
});

/** Writes to one thread's work go one at a time, so each rolls back to a copy none holds. */
const serial = keyedSerial<number>();

/**
 * Runs one write: shows `optimistic` facts at once (when given), sends `request`, lands the
 * detail it answers, and on a refusal or interruption puts the old facts back unless they
 * changed meanwhile.
 */
const write = <E, R>(
  threadId: number,
  shown: ((before: WorkFacts | null) => WorkFacts | null) | null,
  request: (before: WorkFacts | null) => Effect.Effect<ThreadDetail, E, R>,
) =>
  serial.run(
    threadId,
    Effect.gen(function* () {
      const before = store.getState().threads[threadId]?.work ?? null;
      const optimistic = shown === null ? undefined : shown(before);

      mutations.countWorkWrite(threadId, 1);

      if (optimistic !== undefined) {
        mutations.putWorkFacts(threadId, optimistic);
      }

      const rollBack = Effect.sync(() => {
        if (optimistic !== undefined) {
          mutations.rollbackWork(threadId, optimistic);
        }
      });

      const reply = yield* settledDetail(request(before), (detail, _since, read) =>
        mutations.landWorkReply(detail, optimistic, read),
      ).pipe(
        Effect.tapError(() => rollBack),
        Effect.onInterrupt(() => rollBack),
      );

      const detail = reply.answer;

      if (store.getState().work.overlays[threadId] !== undefined) {
        // An outdated reply left no confirmed copy of this successful local change.
        do {
          const read = captureWorkRead(store.getState());
          const copy = yield* fetchThread(threadId);

          mutations.landWorkReply(copy, optimistic, read);
        } while (store.getState().work.overlays[threadId] !== undefined);
      }

      const current = store.getState().threads[threadId]?.work;

      if (
        current != null &&
        detail.thread.work !== null &&
        current.updatedAt > detail.thread.work.updatedAt
      ) {
        yield* refresh(threadId);
      }

      return detail;
    }).pipe(Effect.ensuring(Effect.sync(() => mutations.countWorkWrite(threadId, -1)))),
  );

/**
 * Moves the work to `status`, starts tracking an untracked thread, or stops tracking it
 * (`null`, which clears the owner too, as the server requires).
 */
export const setStatus = Effect.fn("work.setStatus")(function* (
  threadId: number,
  status: WorkStatus | null,
) {
  return yield* write(
    threadId,
    (before) => optimisticFacts(before, status),
    (before) => {
      const body: UpdateWork =
        status === null && before?.owner != null ? { status, ownerId: null } : { status };

      return api.updateWork(threadId, body);
    },
  );
});

/** Assigns the work to a person or agent from `ownerCandidates`, or unassigns it (`null`). */
export const assign = Effect.fn("work.assign")(function* (
  threadId: number,
  ownerId: number | null,
) {
  return yield* write(
    threadId,
    (before) => {
      const owner = ownerId === null ? null : (store.getState().users[ownerId] ?? undefined);

      // An owner the store doesn't know yet shows once the server answers.
      return before === null || owner === undefined
        ? before
        : { ...before, owner, ownerActive: owner !== null };
    },
    () => api.updateWork(threadId, { ownerId }),
  );
});

/** Records the result (Markdown), or clears it (`null` or blank). Not optimistic. */
export const saveResult = Effect.fn("work.saveResult")(function* (
  threadId: number,
  markdown: string | null,
) {
  return yield* write(threadId, null, () => api.updateWork(threadId, { resultMarkdown: markdown }));
});

/** Hands the work to an agent with a context package. Not optimistic. */
export const handOff = Effect.fn("work.handOff")(function* (
  threadId: number,
  body: CreateWorkHandoff,
) {
  return yield* write(threadId, null, () => api.handOffWork(threadId, body));
});

/** Updates board work fields through the same serialized write path as the work controls. */
export const update = Effect.fn("work.update")(function* (threadId: number, body: UpdateWork) {
  return yield* write(threadId, null, () => api.updateWork(threadId, body));
});
