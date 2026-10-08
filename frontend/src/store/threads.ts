/** Reducers for threads: records, the viewer's memberships, panes, lists and indicators. */
import type { ThreadDetail } from "../gen/ThreadDetail.ts";
import type { ThreadIndicatorChanged } from "../gen/ThreadIndicatorChanged.ts";
import type { ThreadList } from "../gen/ThreadList.ts";
import { boardThreadChanged, mergeSteps, removeBoardPost } from "./boards.ts";
import { reconcileMessage } from "./cards.ts";
import type { Thread, ThreadFilter, ThreadMembership } from "./model.ts";
import { mergeUserList } from "./ordering.ts";
import type { State } from "./state.ts";

function matches(filter: ThreadFilter, thread: Thread): boolean {
  return filter === "all" || thread.status === filter;
}

/** Most recently active first, then newest. */
function byActivity(threads: State["threads"]) {
  return (left: number, right: number): number => {
    const a = threads[left];
    const b = threads[right];

    if (a === undefined || b === undefined) {
      return 0;
    }

    if (a.lastActivityAt !== b.lastActivityAt) {
      return a.lastActivityAt < b.lastActivityAt ? 1 : -1;
    }

    return b.id - a.id;
  };
}

/** A thread record (from `thread.created`, `thread.updated` or a reply): into the map and lists. */
export function upsertThread(state: State, thread: Thread): State {
  if (state.removedThreads[thread.id] !== undefined) {
    return state;
  }

  state = boardThreadChanged(state, thread);
  const threads = { ...state.threads, [thread.id]: thread };
  const list = state.roomThreads[thread.roomId];

  if (list === undefined) {
    return { ...state, threads };
  }

  const others = list.ids.filter((id) => id !== thread.id);

  const ids = matches(list.filter, thread)
    ? [...others, thread.id].sort(byActivity(threads))
    : others;

  return {
    ...state,
    threads,
    roomThreads: { ...state.roomThreads, [thread.roomId]: { ...list, ids } },
  };
}

/**
 * The thread may have been removed after a request sent at `since` (the `removalCount` then), so
 * its reply mustn't bring it back: its removal is newer, or a removal that may be its own has been
 * forgotten and the thread isn't held now.
 */
export function removedSince(state: State, threadId: number, since: number): boolean {
  const removed = state.removedThreads[threadId];

  if (removed !== undefined) {
    return removed > since;
  }

  return uncertainSince(state, threadId, since);
}

/**
 * A reply to a request sent at `since` can't tell whether `threadId` was removed meanwhile: no
 * removal of it is remembered, but removals after `since` have been forgotten and it isn't held.
 * The reducers drop such a thread; the actions ask again first (see `settled`), since a fresh
 * request's `since` is past every forgotten removal.
 */
export function uncertainSince(state: State, threadId: number, since: number): boolean {
  return (
    state.removedThreads[threadId] === undefined &&
    since < state.forgottenRemoval &&
    state.threads[threadId] === undefined
  );
}

/** Forgets that `threadId` was removed: a reply to a request sent after the removal showed it. */
export function revive(state: State, threadId: number, since: number): State {
  const removed = state.removedThreads[threadId];

  if (removed === undefined || removed > since) {
    return state;
  }

  const { [threadId]: _lifted, ...removedThreads } = state.removedThreads;

  return { ...state, removedThreads };
}

/** The most removals remembered; older ones only count through `forgottenRemoval`. */
export const MAX_REMOVED_THREADS = 500;

function tombstone(state: State, threadId: number): Partial<State> {
  const count = state.removalCount + 1;
  const entries = Object.entries({ ...state.removedThreads, [threadId]: count });

  if (entries.length <= MAX_REMOVED_THREADS) {
    return { removedThreads: Object.fromEntries(entries), removalCount: count };
  }

  const newest = entries.sort(([, a], [, b]) => b - a);
  const dropped = newest.slice(MAX_REMOVED_THREADS);

  return {
    removedThreads: Object.fromEntries(newest.slice(0, MAX_REMOVED_THREADS)),
    removalCount: count,
    forgottenRemoval: Math.max(state.forgottenRemoval, ...dropped.map(([, removal]) => removal)),
  };
}

/** What an open pane says once its thread is deleted. */
export const THREAD_DELETED = "This thread was deleted.";

/** A moderator deleted the thread: it leaves the lists, its pane says so, the indicator goes. */
export function removeThread(state: State, threadId: number, roomId: number): State {
  const thread = state.threads[threadId];
  const { [threadId]: _gone, ...threads } = state.threads;
  const list = state.roomThreads[roomId];
  const parentId = thread?.parentMessageId ?? null;
  const parent = parentId === null ? undefined : state.messages[parentId];

  return {
    ...removeBoardPost(state, roomId, threadId),
    ...tombstone(state, threadId),
    threads,
    // Only a pane someone opened says so; others aren't kept for every removal.
    threadPanes:
      state.threadPanes[threadId] === undefined
        ? state.threadPanes
        : {
            ...state.threadPanes,
            [threadId]: {
              status: "error",
              error: THREAD_DELETED,
              permissions: null,
              work: null,
              workFacts: null,
            },
          },
    roomThreads:
      list === undefined
        ? state.roomThreads
        : {
            ...state.roomThreads,
            [roomId]: { ...list, ids: list.ids.filter((id) => id !== threadId) },
          },
    messages:
      parent === undefined
        ? state.messages
        : { ...state.messages, [parent.id]: { ...parent, thread: null } },
  };
}

/** The parent message's reply summary changed. */
export function setThreadIndicator(state: State, change: ThreadIndicatorChanged): State {
  const parent = state.messages[change.parentMessageId];

  if (parent === undefined) {
    return state;
  }

  return {
    ...state,
    messages: { ...state.messages, [parent.id]: { ...parent, thread: change.thread } },
  };
}

/** The viewer's membership went unread (`unreadAt`) or read (`null`); unknown memberships wait. */
export function setThreadUnread(state: State, threadId: number, unreadAt: string | null): State {
  const membership = state.threadMemberships[threadId];

  if (membership == null || membership.unreadAt === unreadAt) {
    return state;
  }

  return {
    ...state,
    threadMemberships: { ...state.threadMemberships, [threadId]: { ...membership, unreadAt } },
  };
}

/** The viewer joined, changed involvement or left (`null`). */
export function setThreadMembership(
  state: State,
  threadId: number,
  membership: ThreadMembership | null,
): State {
  return {
    ...state,
    threadMemberships: { ...state.threadMemberships, [threadId]: membership },
  };
}

export function setThreadPaneLoading(state: State, threadId: number): State {
  const pane = state.threadPanes[threadId];

  return {
    ...state,
    threadPanes: {
      ...state.threadPanes,
      [threadId]: {
        status: "loading",
        error: null,
        permissions: pane?.permissions ?? null,
        work: pane?.work ?? null,
        workFacts: pane?.workFacts ?? null,
      },
    },
  };
}

/** A failed load's error; a remembered `thread.removed` wins over whatever the load says. */
export function setThreadPaneError(state: State, threadId: number, error: string): State {
  const pane = state.threadPanes[threadId];

  return {
    ...state,
    threadPanes: {
      ...state.threadPanes,
      [threadId]: {
        status: "error",
        error: state.removedThreads[threadId] === undefined ? error : THREAD_DELETED,
        permissions: pane?.permissions ?? null,
        work: pane?.work ?? null,
        workFacts: pane?.workFacts ?? null,
      },
    },
  };
}

/**
 * `GET /threads/:id` (or a write that answers the thread) landed: the record, the viewer's
 * membership, the parent and permissions. `since` is the `removalCount` when the request was
 * sent: a reply to a request sent after the thread's removal lifts it; an older one is dropped.
 * Agent steps merge with the ones held, by `updatedAt`, since `agent.steps` may be newer.
 */
export function loadThreadDetail(state: State, detail: ThreadDetail, since: number): State {
  const threadId = detail.thread.id;

  if (removedSince(state, threadId, since)) {
    return state;
  }

  const revived = revive(state, threadId, since);
  const parent = detail.parentMessage;
  const held = parent === null ? undefined : state.messages[parent.id];
  const next = upsertThread(revived, detail.thread);
  const heldWork = state.threadPanes[threadId]?.work;

  const work =
    detail.work === null || heldWork == null
      ? detail.work
      : { ...detail.work, steps: mergeSteps(heldWork.steps, detail.work.steps) };

  const kept = parent === null ? undefined : reconcileMessage(held, parent, true);

  return {
    ...next,
    users: mergeUserList(next.users, detail.users),
    messages:
      kept === undefined || kept === held ? next.messages : { ...next.messages, [kept.id]: kept },
    threadMemberships: { ...next.threadMemberships, [threadId]: detail.membership },
    threadPanes: {
      ...next.threadPanes,
      [threadId]: {
        status: "ready",
        error: null,
        permissions: detail.permissions,
        work,
        workFacts: detail.thread.work,
      },
    },
  };
}

export function setThreadListLoading(state: State, roomId: number, filter: ThreadFilter): State {
  const list = state.roomThreads[roomId];

  return {
    ...state,
    roomThreads: {
      ...state.roomThreads,
      [roomId]: {
        filter,
        ids: list?.filter === filter ? list.ids : [],
        status: "loading",
      },
    },
  };
}

export function setThreadListFailed(state: State, roomId: number, filter: ThreadFilter): State {
  const list = state.roomThreads[roomId];

  // A failure for a filter the pane has since left says nothing about the current one.
  if (list === undefined || list.filter !== filter) {
    return state;
  }

  return {
    ...state,
    roomThreads: { ...state.roomThreads, [roomId]: { ...list, status: "error" } },
  };
}

/**
 * `GET /rooms/:id/threads` landed for `filter`. `since` is the `removalCount` when it was sent: a
 * thread removed after that stays out, and one it shows that was removed before is back.
 */
export function loadThreadList(
  state: State,
  roomId: number,
  filter: ThreadFilter,
  page: ThreadList,
  since: number,
): State {
  const list = {
    ...page,
    threads: page.threads.filter(({ thread }) => !removedSince(state, thread.id, since)),
  };

  state = list.threads.reduce((revived, { thread }) => revive(revived, thread.id, since), state);
  const current = state.roomThreads[roomId];
  // A response for a filter the pane has since left (an earlier tab answering late) still
  // teaches us its threads, but mustn't replace the current tab's list.
  const stale = current !== undefined && current.filter !== filter;

  return {
    ...state,
    users: mergeUserList(state.users, list.users),
    threads: {
      ...state.threads,
      ...Object.fromEntries(list.threads.map((summary) => [summary.thread.id, summary.thread])),
    },
    threadMemberships: {
      ...state.threadMemberships,
      ...Object.fromEntries(list.threads.map((summary) => [summary.thread.id, summary.membership])),
    },
    roomThreads: stale
      ? state.roomThreads
      : {
          ...state.roomThreads,
          [roomId]: {
            filter,
            ids: list.threads.map((summary) => summary.thread.id),
            status: "ready",
          },
        },
  };
}
