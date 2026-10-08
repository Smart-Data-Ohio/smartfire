import type { AgentStep } from "../gen/AgentStep.ts";
import type { AgentStepsChanged } from "../gen/AgentStepsChanged.ts";
import type { BoardDigest } from "../gen/BoardDigest.ts";
import type { BoardListing } from "../gen/BoardListing.ts";
import type { BoardOwnerOption } from "../gen/BoardOwnerOption.ts";
import type { BoardStatusFilter } from "../gen/BoardStatusFilter.ts";
import type { BoardTagCount } from "../gen/BoardTagCount.ts";
import type { Thread } from "../gen/Thread.ts";
import { mergeUserList } from "./ordering.ts";
import type { State } from "./state.ts";

export interface BoardQuery {
  readonly status: BoardStatusFilter;
  readonly owner: string;
  readonly tag: string;
}

export const defaultBoardQuery: BoardQuery = { status: "open", owner: "anyone", tag: "" };

export interface BoardState {
  readonly status: "loading" | "ready" | "error";
  readonly error: string | null;
  readonly query: BoardQuery;
  readonly page: number;
  /** The server's window, plus posts received through thread.created. */
  readonly postIds: readonly number[];
  readonly hasMore: boolean;
  readonly loadingMore: boolean;
  readonly anyPosts: boolean;
  readonly ownerOptions: readonly BoardOwnerOption[];
  readonly tagCounts: readonly BoardTagCount[];
  readonly digest: BoardDigest | null;
  readonly canAdminister: boolean;
  /** Every request has its own generation, including repeat queries and pagination. */
  readonly generation: number;
  readonly livePostIds: readonly number[];
  /** Changes received while a listing request is in flight win over its snapshot. */
  readonly changedPostIds: readonly number[];
  readonly removedPostIds: readonly number[];
}

export function normalizeBoardQuery(query: BoardQuery): BoardQuery {
  return {
    status: query.status,
    owner: /^(anyone|me|agents|[1-9]\d*)$/.test(query.owner) ? query.owner : "anyone",
    tag: query.tag.trim().toLowerCase(),
  };
}

export function setBoardLoading(
  state: State,
  roomId: number,
  query: BoardQuery,
  more = false,
): State {
  const held = state.boards[roomId];
  const normalized = normalizeBoardQuery(query);
  const same = held !== undefined && JSON.stringify(held.query) === JSON.stringify(normalized);

  return {
    ...state,
    boards: {
      ...state.boards,
      [roomId]: {
        status: more && held !== undefined ? held.status : "loading",
        error: null,
        query: normalized,
        page: more ? (held?.page ?? 1) : 1,
        postIds: same ? held.postIds : [],
        hasMore: more && held !== undefined ? held.hasMore : false,
        loadingMore: more,
        anyPosts: held?.anyPosts ?? false,
        ownerOptions: held?.ownerOptions ?? [],
        tagCounts: held?.tagCounts ?? [],
        digest: held?.digest ?? null,
        canAdminister: held?.canAdminister ?? false,
        generation: (held?.generation ?? 0) + 1,
        livePostIds: same && more ? held.livePostIds : [],
        changedPostIds: [],
        removedPostIds: [],
      },
    },
  };
}

export function setBoardError(
  state: State,
  roomId: number,
  generation: number,
  error: string,
): State {
  const held = state.boards[roomId];

  if (held === undefined || held.generation !== generation) return state;

  return {
    ...state,
    boards: {
      ...state.boards,
      [roomId]: {
        ...held,
        status: held.loadingMore ? held.status : "error",
        error,
        loadingMore: false,
      },
    },
  };
}

/** A stale response changes neither the selected query nor the live facts other views hold. */
export function loadBoardListing(state: State, listing: BoardListing, generation: number): State {
  const held = state.boards[listing.roomId];

  if (held === undefined || held.generation !== generation) return state;
  const posts = listing.posts.filter(({ thread }) => !held.removedPostIds.includes(thread.id));
  const snapshot = posts.filter(({ thread }) => !held.changedPostIds.includes(thread.id));
  // Asked for after any removal this board saw, so a post it shows really exists.
  const shown = new Set(posts.map(({ thread }) => thread.id));

  const removedThreads = Object.fromEntries(
    Object.entries(state.removedThreads).filter(([id]) => !shown.has(Number(id))),
  );

  return {
    ...state,
    removedThreads,
    users: mergeUserList(state.users, listing.users),
    threads: {
      ...state.threads,
      ...Object.fromEntries(snapshot.map(({ thread }) => [thread.id, thread])),
    },
    threadMemberships: {
      ...state.threadMemberships,
      ...Object.fromEntries(posts.map(({ thread, membership }) => [thread.id, membership])),
    },
    boards: {
      ...state.boards,
      [listing.roomId]: {
        ...held,
        status: "ready",
        error: null,
        query: { status: listing.status, owner: listing.owner, tag: listing.tag },
        page: listing.page,
        postIds: [...new Set([...posts.map(({ thread }) => thread.id), ...held.livePostIds])],
        hasMore: listing.hasMore,
        loadingMore: false,
        anyPosts: listing.anyPosts || held.livePostIds.length > 0,
        ownerOptions: listing.ownerOptions,
        tagCounts: listing.tagCounts,
        digest: listing.digest,
        canAdminister: listing.canAdminister,
        changedPostIds: [],
        removedPostIds: [],
      },
    },
  };
}

/** Preserve live facts when the board snapshot was requested before this update. */
export function boardThreadChanged(state: State, thread: Thread): State {
  const held = state.boards[thread.roomId];

  if (
    held === undefined ||
    (held.status !== "loading" && !held.loadingMore) ||
    held.changedPostIds.includes(thread.id)
  )
    return state;

  return {
    ...state,
    boards: {
      ...state.boards,
      [thread.roomId]: { ...held, changedPostIds: [...held.changedPostIds, thread.id] },
    },
  };
}

/** Only creation admits a thread outside the loaded window; ordinary updates never do. */
export function addBoardPost(state: State, thread: Thread): State {
  const held = state.boards[thread.roomId];

  if (
    held === undefined ||
    held.postIds.includes(thread.id) ||
    state.removedThreads[thread.id] !== undefined
  )
    return state;

  return {
    ...state,
    boards: {
      ...state.boards,
      [thread.roomId]: {
        ...held,
        postIds: [...held.postIds, thread.id],
        livePostIds: [...held.livePostIds, thread.id],
        anyPosts: true,
      },
    },
  };
}

export function removeBoardPost(state: State, roomId: number, threadId: number): State {
  const held = state.boards[roomId];

  if (held === undefined) return state;
  // Only a listing in flight needs to know: it may still show the post.
  const loading = held.status === "loading" || held.loadingMore;

  return {
    ...state,
    boards: {
      ...state.boards,
      [roomId]: {
        ...held,
        postIds: held.postIds.filter((id) => id !== threadId),
        livePostIds: held.livePostIds.filter((id) => id !== threadId),
        removedPostIds:
          loading && !held.removedPostIds.includes(threadId)
            ? [...held.removedPostIds, threadId]
            : held.removedPostIds,
      },
    },
  };
}

function matches(state: State, thread: Thread, query: BoardQuery): boolean {
  const work = thread.work;

  if (work === null) return false;

  if (query.status === "open" && work.status === "done") return false;

  if (query.status === "done" && work.status !== "done") return false;

  if (
    query.owner === "me" &&
    (work.owner === null || work.owner.id !== (state.me?.user.id ?? state.boot?.user.id))
  )
    return false;

  if (query.owner === "agents" && work.owner?.role !== "bot" && work.owner?.agent == null)
    return false;

  if (/^\d+$/.test(query.owner) && work.owner?.id !== Number(query.owner)) return false;

  return query.tag === "" || work.tags.includes(query.tag);
}

/** Live facts determine filtering and order within the server window and live arrivals. */
export function boardPostIds(state: State, roomId: number): number[] {
  const board = state.boards[roomId];

  if (board === undefined) return [];

  return board.postIds
    .filter((id) => {
      const thread = state.threads[id];

      return (
        thread !== undefined && thread.roomId === roomId && matches(state, thread, board.query)
      );
    })
    .sort((a, b) => {
      const left = state.threads[a];
      const right = state.threads[b];

      if (left === undefined || right === undefined) return 0;

      return left.lastActivityAt === right.lastActivityAt
        ? b - a
        : left.lastActivityAt < right.lastActivityAt
          ? 1
          : -1;
    });
}

export interface BoardColumns {
  planned: number[];
  in_progress: number[];
  blocked: number[];
  done: number[];
}

export function boardColumns(state: State, roomId: number): BoardColumns {
  const columns: BoardColumns = { planned: [], in_progress: [], blocked: [], done: [] };

  for (const id of boardPostIds(state, roomId)) {
    const status = state.threads[id]?.work?.status;

    if (status !== undefined && Object.hasOwn(columns, status)) columns[status].push(id);
  }

  return columns;
}

/**
 * Steps merged by id, keeping the copy with the later `updatedAt` (the incoming one on a tie), in
 * `(position, id)` order. Steps are never deleted apart from their parent, so none is dropped.
 */
export function mergeSteps(
  held: readonly AgentStep[],
  incoming: readonly AgentStep[],
): AgentStep[] {
  const byId = new Map<number, AgentStep>(held.map((step) => [step.id, step]));

  for (const step of incoming) {
    const kept = byId.get(step.id);

    if (kept === undefined || Date.parse(step.updatedAt) >= Date.parse(kept.updatedAt)) {
      byId.set(step.id, step);
    }
  }

  return [...byId.values()].sort((a, b) => a.position - b.position || a.id - b.id);
}

/**
 * `agent.steps` for a work thread: merges the steps into the open post's work by id, keeping the
 * copy with the later `updatedAt` (the later arrival on a tie), in `(position, id)` order. Steps
 * on a message, and threads whose work isn't loaded, are left alone (a load brings them).
 */
export function mergeWorkSteps(state: State, event: AgentStepsChanged): State {
  if (event.messageId !== null || event.threadId === null) {
    return state;
  }

  const pane = state.threadPanes[event.threadId];

  if (pane?.work == null) {
    return state;
  }

  const steps = mergeSteps(pane.work.steps, event.steps);

  return {
    ...state,
    threadPanes: {
      ...state.threadPanes,
      [event.threadId]: { ...pane, work: { ...pane.work, steps } },
    },
  };
}
