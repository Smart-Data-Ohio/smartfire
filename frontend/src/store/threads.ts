/** Reducers for threads: records, the viewer's memberships, panes, lists and indicators. */
import type { ThreadDetail } from "../gen/ThreadDetail.ts";
import type { ThreadIndicatorChanged } from "../gen/ThreadIndicatorChanged.ts";
import type { ThreadList } from "../gen/ThreadList.ts";
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

/** A moderator deleted the thread: it leaves the lists, its pane says so, the indicator goes. */
export function removeThread(state: State, threadId: number, roomId: number): State {
  const thread = state.threads[threadId];
  const { [threadId]: _gone, ...threads } = state.threads;
  const list = state.roomThreads[roomId];
  const parentId = thread?.parentMessageId ?? null;
  const parent = parentId === null ? undefined : state.messages[parentId];

  return {
    ...state,
    threads,
    threadPanes: {
      ...state.threadPanes,
      [threadId]: { status: "error", error: "This thread was deleted.", permissions: null },
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
      [threadId]: { status: "loading", error: null, permissions: pane?.permissions ?? null },
    },
  };
}

export function setThreadPaneError(state: State, threadId: number, error: string): State {
  const pane = state.threadPanes[threadId];

  return {
    ...state,
    threadPanes: {
      ...state.threadPanes,
      [threadId]: { status: "error", error, permissions: pane?.permissions ?? null },
    },
  };
}

/** `GET /threads/:id` landed: the record, the viewer's membership, the parent and permissions. */
export function loadThreadDetail(state: State, detail: ThreadDetail): State {
  const threadId = detail.thread.id;
  const parent = detail.parentMessage;
  const held = parent === null ? undefined : state.messages[parent.id];
  const next = upsertThread(state, detail.thread);
  const keepHeld = parent === null || (held !== undefined && held.updatedAt > parent.updatedAt);

  return {
    ...next,
    users: mergeUserList(next.users, detail.users),
    messages: keepHeld ? next.messages : { ...next.messages, [parent.id]: parent },
    threadMemberships: { ...next.threadMemberships, [threadId]: detail.membership },
    threadPanes: {
      ...next.threadPanes,
      [threadId]: { status: "ready", error: null, permissions: detail.permissions },
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

export function setThreadListFailed(state: State, roomId: number): State {
  const list = state.roomThreads[roomId];

  if (list === undefined) {
    return state;
  }

  return {
    ...state,
    roomThreads: { ...state.roomThreads, [roomId]: { ...list, status: "error" } },
  };
}

/** `GET /rooms/:id/threads` landed for `filter`. */
export function loadThreadList(
  state: State,
  roomId: number,
  filter: ThreadFilter,
  list: ThreadList,
): State {
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
    roomThreads: {
      ...state.roomThreads,
      [roomId]: {
        filter,
        ids: list.threads.map((summary) => summary.thread.id),
        status: "ready",
      },
    },
  };
}
