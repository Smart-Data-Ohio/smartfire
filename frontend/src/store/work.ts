/**
 * Work tracking in the store (S4). A thread's work facts live on the thread itself
 * (`state.threads[id].work`, kept by `thread.updated`); this slice holds what only the thread
 * pane and the work page need: each open thread's `WorkDetail`, the facts that detail was
 * loaded with (so a pane can tell when live facts moved on and refetch), the writes in flight,
 * and the work list per filter (a snapshot: no live updates, it reloads when shown again).
 */
import type { ThreadDetail } from "../gen/ThreadDetail.ts";
import type { WorkDetail } from "../gen/WorkDetail.ts";
import type { WorkFacts } from "../gen/WorkFacts.ts";
import type { WorkFilter } from "../gen/WorkFilter.ts";
import type { WorkLink } from "../gen/WorkLink.ts";
import type { WorkList } from "../gen/WorkList.ts";
import type { WorkListRow } from "../gen/WorkListRow.ts";
import type { WorkStatus } from "../gen/WorkStatus.ts";
import { mergeFreshSteps } from "./agents.ts";
import { acknowledge, land, markLocal, modifiedAt, receive } from "./freshness.ts";
import type { LoadStatus, Thread } from "./model.ts";
import { mergeUserList } from "./ordering.ts";
import type { State } from "./state.ts";
import { loadThreadDetail, upsertThread } from "./threads.ts";

/** One filter's work list. */
export interface WorkListState {
  readonly status: LoadStatus;
  /** Most recently updated first, as the server sent them. */
  readonly rows: readonly WorkListRow[];
  readonly error: string | null;
  /** Bumped by every load; a reply from an older load changes nothing. */
  readonly generation: number;
}

export interface WorkSlice {
  /** By thread id: the open pane's work section; absent while untracked or not loaded. */
  readonly details: Readonly<Record<number, WorkDetail>>;
  /** By thread id: the facts the held detail goes with (`null`: loaded untracked). */
  readonly heldFacts: Readonly<Record<number, WorkFacts | null>>;
  /** By thread id: work writes on their way (no refetch while any is). */
  readonly writes: Readonly<Record<number, number>>;
  readonly lists: Readonly<Partial<Record<WorkFilter, WorkListState>>>;
}

export const emptyWork: WorkSlice = {
  details: {},
  heldFacts: {},
  writes: {},
  lists: {},
};

export const workKey = (threadId: number) => `work:${threadId}`;

const detailKey = (threadId: number) => `work-detail:${threadId}`;

/** Only changed work facts mark the key; equal write echoes leave its ticket intact. */
export function receiveWorkThread(state: State, thread: Thread, ticket?: number): State {
  const copies = {
    held: state.threads[thread.id]?.work,
    incoming: thread.work,
    same: sameWorkFacts,
  };

  const result =
    ticket === undefined
      ? receive(state.freshness, workKey(thread.id), copies)
      : land(state.freshness, workKey(thread.id), ticket, copies);

  return upsertThread(
    { ...state, freshness: result.freshness },
    {
      ...thread,
      work: result.value ?? null,
    },
  );
}

/** Merge a detail's work key while allowing its unrelated thread data to land. */
export function loadWorkThreadDetail(state: State, detail: ThreadDetail, ticket?: number): State {
  const threadId = detail.thread.id;

  const copies = {
    held: state.threads[threadId]?.work,
    incoming: detail.thread.work,
    same: sameWorkFacts,
  };

  const result =
    ticket === undefined
      ? receive(state.freshness, workKey(threadId), copies)
      : land(state.freshness, workKey(threadId), ticket, copies);

  const next = loadThreadDetail(
    { ...state, freshness: result.freshness },
    {
      ...detail,
      thread: { ...detail.thread, work: result.value ?? null },
    },
  );

  if (result.accepted) {
    return landWorkDetail(next, detail, ticket);
  }

  const held = next.work.details[threadId];

  if (held === undefined || detail.work === null) {
    return next;
  }

  const merged = mergeFreshSteps(next, held.steps, detail.work.steps, ticket);

  return withWork(
    { ...next, freshness: merged.freshness },
    {
      details: { ...next.work.details, [threadId]: { ...held, steps: [...merged.steps] } },
    },
  );
}

/** The filters, in the work page's tab order. */
export const WORK_FILTERS: readonly WorkFilter[] = ["open", "done", "all", "agents", "boards"];

export const emptyWorkList: WorkListState = {
  status: "idle",
  rows: [],
  error: null,
  generation: 0,
};

/** One filter's list, or an empty one never loaded. */
export function workListOf(state: State, filter: WorkFilter): WorkListState {
  return state.work.lists[filter] ?? emptyWorkList;
}

function sameLink(a: WorkLink, b: WorkLink): boolean {
  return (
    a.id === b.id &&
    a.kind === b.kind &&
    a.label === b.label &&
    a.url === b.url &&
    a.pullRequestState === b.pullRequestState &&
    a.title === b.title &&
    a.eventStartsAt === b.eventStartsAt &&
    a.eventTimeZone === b.eventTimeZone &&
    a.eventCancelled === b.eventCancelled
  );
}

/** Whether two sets of work facts say the same thing (any detail change changes them). */
export function sameWorkFacts(a: WorkFacts | null, b: WorkFacts | null): boolean {
  if (a === null || b === null) {
    return a === b;
  }

  return (
    a.status === b.status &&
    (a.owner?.id ?? null) === (b.owner?.id ?? null) &&
    a.ownerActive === b.ownerActive &&
    a.runUrl === b.runUrl &&
    a.resultUpdatedAt === b.resultUpdatedAt &&
    a.links.length === b.links.length &&
    a.links.every((link, index) => {
      const other = b.links[index];

      return other !== undefined && sameLink(link, other);
    })
  );
}

/**
 * Whether the pane's work detail is behind the live facts: it was loaded, no write of ours is on
 * its way, and `thread.updated` brought facts that differ from the ones it came with.
 */
export function workDetailStale(state: State, threadId: number): boolean {
  const held = state.work.heldFacts[threadId];
  const live = state.threads[threadId]?.work;

  if (held === undefined || live === undefined || (state.work.writes[threadId] ?? 0) > 0) {
    return false;
  }

  return (
    modifiedAt(state.freshness, detailKey(threadId)) <
      modifiedAt(state.freshness, workKey(threadId)) || !sameWorkFacts(held, live)
  );
}

function withWork(state: State, change: Partial<WorkSlice>): State {
  return { ...state, work: { ...state.work, ...change } };
}

/** A `ThreadDetail` landed (the pane loaded, or a write answered): keep its work section. */
export function landWorkDetail(state: State, detail: ThreadDetail, ticket?: number): State {
  const threadId = detail.thread.id;
  const { [threadId]: _old, ...others } = state.work.details;

  const { steps, freshness } = mergeFreshSteps(
    state,
    _old?.steps ?? [],
    detail.work?.steps ?? [],
    ticket,
  );

  return withWork(
    {
      ...state,
      freshness: acknowledge(
        freshness,
        detailKey(threadId),
        modifiedAt(state.freshness, workKey(threadId)),
      ),
    },
    {
      details:
        detail.work === null
          ? others
          : { ...others, [threadId]: { ...detail.work, steps: [...steps] } },
      heldFacts: { ...state.work.heldFacts, [threadId]: detail.thread.work },
    },
  );
}

/** A work write started (`+1`) or settled (`-1`). */
export function countWorkWrite(state: State, threadId: number, delta: 1 | -1): State {
  const count = Math.max(0, (state.work.writes[threadId] ?? 0) + delta);
  const { [threadId]: _count, ...others } = state.work.writes;

  return withWork(state, { writes: count === 0 ? others : { ...others, [threadId]: count } });
}

/**
 * The facts a status change shows at once: a new status (tracking starts unassigned, with no
 * links), or `null` to stop tracking. `null` when the thread isn't held.
 */
export function optimisticFacts(
  current: WorkFacts | null,
  status: WorkStatus | null,
): WorkFacts | null {
  if (status === null) {
    return null;
  }

  if (current === null) {
    return {
      status,
      owner: null,
      ownerActive: false,
      runUrl: null,
      resultUpdatedAt: null,
      links: [],
    };
  }

  return { ...current, status };
}

/**
 * Shows work facts on a thread at once (an optimistic change, or its rollback). The held facts
 * follow, so the pane doesn't take its own change for someone else's.
 */
export function putWorkFacts(state: State, threadId: number, facts: WorkFacts | null): State {
  const thread = state.threads[threadId];

  if (thread === undefined) {
    return state;
  }

  const freshness = markLocal(state.freshness, workKey(threadId));

  const next = upsertThread(
    {
      ...state,
      freshness: acknowledge(freshness, detailKey(threadId), freshness.clock),
    },
    { ...thread, work: facts },
  );

  return withWork(next, {
    heldFacts:
      state.work.heldFacts[threadId] === undefined
        ? next.work.heldFacts
        : { ...next.work.heldFacts, [threadId]: facts },
  });
}

function updateList(
  state: State,
  filter: WorkFilter,
  change: (list: WorkListState) => WorkListState,
): State {
  return withWork(state, {
    lists: { ...state.work.lists, [filter]: change(workListOf(state, filter)) },
  });
}

/** A load started: a list never shown says loading; a shown one keeps its rows. */
export function setWorkListLoading(state: State, filter: WorkFilter): State {
  return updateList(state, filter, (list) => ({
    ...list,
    status: list.status === "ready" ? "ready" : "loading",
    error: null,
    generation: list.generation + 1,
  }));
}

/** A load failed: a list never shown says so; a shown one keeps its rows and the message. */
export function setWorkListFailed(
  state: State,
  filter: WorkFilter,
  error: string,
  generation: number,
): State {
  return updateList(state, filter, (list) =>
    list.generation === generation
      ? { ...list, status: list.status === "ready" ? "ready" : "error", error }
      : list,
  );
}

/** A list landed: its rows replace the old, and their creators and owners join the users. */
export function landWorkList(
  state: State,
  filter: WorkFilter,
  list: WorkList,
  generation: number,
  ticket?: number,
): State {
  if (workListOf(state, filter).generation !== generation) {
    return state;
  }

  let next = { ...state, users: mergeUserList(state.users, list.users) };

  const rows = list.threads.map((row) => {
    if (ticket === undefined) {
      return row;
    }

    const result = land(next.freshness, workKey(row.thread.id), ticket, {
      held: next.threads[row.thread.id]?.work,
      incoming: row.thread.work,
      same: sameWorkFacts,
    });

    const thread = { ...row.thread, work: result.value ?? null };

    next = upsertThread({ ...next, freshness: result.freshness }, thread);

    return { ...row, thread };
  });

  return updateList(next, filter, (held) => ({
    ...held,
    status: "ready",
    rows,
    error: null,
  }));
}
