/**
 * Work tracking's actions as Effect programs (S4). The work list loads per filter (a reply from
 * an older load changes nothing). A status or owner change shows on the thread at once and
 * rolls back if the server refuses, unless something changed the facts meanwhile; changes to
 * one thread go one at a time. Every write lands the `ThreadDetail` the server answers, and
 * while one is on its way the pane doesn't refetch on the `thread.updated` it causes.
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
import { optimisticFacts, sameWorkFacts, workListOf, workVersion } from "../store/work.ts";
import { keyedSerial } from "./serial.ts";

/** Loads (or reloads) a filter's list; a failure lands as the list's error. */
export const loadList = Effect.fn("work.loadList")(function* (filter: WorkFilter) {
  mutations.setWorkListLoading(filter);

  const { generation } = workListOf(store.getState(), filter);

  yield* api.workList(filter).pipe(
    Effect.tap((list) => Effect.sync(() => mutations.landWorkList(filter, list, generation))),
    Effect.catch((error) =>
      Effect.sync(() => mutations.setWorkListFailed(filter, error.message, generation)),
    ),
  );
});

/** Threads with a refetch on its way, so a burst of events asks once. */
const refreshing = new Set<number>();

/**
 * Refetches the thread's detail because live facts moved past the held ones. A failure keeps
 * what the pane shows; the next change tries again.
 */
export const refresh = Effect.fn("work.refresh")(function* (threadId: number) {
  if (refreshing.has(threadId)) {
    return;
  }

  refreshing.add(threadId);

  const version = workVersion(store.getState(), threadId);

  yield* fetchThread(threadId).pipe(
    Effect.tap((detail) => Effect.sync(() => mutations.loadThreadDetail(detail, version))),
    Effect.ignore,
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
      const version = workVersion(store.getState(), threadId);
      const optimistic = shown === null ? undefined : shown(before);

      mutations.countWorkWrite(threadId, 1);

      if (optimistic !== undefined) {
        mutations.putWorkFacts(threadId, optimistic);
      }

      const rollBack = Effect.sync(() => {
        const now = store.getState().threads[threadId]?.work ?? null;

        if (optimistic !== undefined && sameWorkFacts(now, optimistic)) {
          mutations.putWorkFacts(threadId, before);
        }
      });

      const detail = yield* request(before).pipe(
        Effect.tapError(() => rollBack),
        Effect.onInterrupt(() => rollBack),
      );

      mutations.loadThreadDetail(detail, version);

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
