import { addBoardPost, boardAutomationsChanged } from "./boards.ts";
/**
 * Pure state transitions. Each takes the current state (and the time, so tests and the sync
 * engine's clock agree) and returns the next. `store.ts` wraps them in `setState`.
 */

import { applyActivityItem, removeActivityItem } from "./activity.ts";
import { applyAgentStatus, applyAgentSteps } from "./agents.ts";
import { applyApprovalUpdated, approvalRequested } from "./approvals.ts";
import { applyMessageCards, applyPoll, applyPollBallot, reconcileMessage } from "./cards.ts";
import { setHuddlePresence, setStage } from "./huddles.ts";
import { mergeSavedMarks, setPinState, setReactions } from "./message-extras.ts";
import type {
  Me,
  MessageDTO,
  MessagePage,
  OpenRoomPreview,
  PendingMessage,
  RoomDetail,
  RoomState,
  Sidebar,
  SidebarRow,
  SyncEvent,
  Timeline,
  User,
  UserPresence,
} from "./model.ts";
import { compareMessages, insertOrdered, mergeUserList } from "./ordering.ts";
import { removeCategory, replyCategories, upsertCategory } from "./organize.ts";
import {
  changedRowIds,
  claimDivider,
  dividerMovedSince,
  isStale,
  markResynced,
  readSince,
  touchedSince,
  touchRows,
} from "./row-touches.ts";
import { applySavedChange, dropSavedForMessage } from "./saved-list.ts";
import { applyScheduled, removeScheduled } from "./scheduled.ts";
import { emptyTimeline, type State, TOMBSTONE_TTL_MS, TYPING_TTL_MS } from "./state.ts";
import { removeThread, setThreadIndicator, setThreadUnread } from "./threads.ts";
import { receiveWorkThread } from "./work.ts";
import { setWorkspaceBranding, setWorkspaceStyles } from "./workspace.ts";

export { compareMessages };

export function setMe(state: State, me: Me): State {
  return { ...state, me, users: mergeUserList(state.users, [me.user]) };
}

export function mergeUsers(state: State, users: readonly User[]): State {
  return { ...state, users: mergeUserList(state.users, users) };
}

export function setPresence(state: State, list: readonly UserPresence[]): State {
  if (list.length === 0) {
    return state;
  }

  const presence = { ...state.presence };

  for (const entry of list) {
    presence[entry.userId] = entry;
  }

  return { ...state, presence };
}

function sortSidebarOrder(rows: Readonly<Record<number, SidebarRow>>): readonly number[] {
  return Object.values(rows)
    .toSorted((left, right) => {
      const a = (left.room.name ?? left.displayName).toLowerCase();
      const b = (right.room.name ?? right.displayName).toLowerCase();

      return a < b ? -1 : a > b ? 1 : left.room.id - right.room.id;
    })
    .map((row) => row.room.id);
}

/**
 * Installs a whole-sidebar HTTP reply. `since` is its request's ticket: once a sync resync has
 * landed after it the reply is stale and changes nothing; otherwise it installs as
 * `installSidebar` does.
 */
export function loadSidebar(state: State, sidebar: Sidebar, since: number): State {
  return isStale(state, since) ? state : installSidebar(state, sidebar, since);
}

/**
 * A whole-sidebar snapshot the sync engine read (a resync), `since` its own ticket: installed as
 * `installSidebar` does, then every HTTP reply to a request already in flight is stale.
 */
export function resyncSidebar(state: State, sidebar: Sidebar, since: number): State {
  return markResynced(installSidebar(state, sidebar, since));
}

/**
 * Installs a whole-sidebar snapshot. `since` is its request's ticket: a room the sync path
 * changed after that keeps the store's row, or stays gone if sync removed it, and a row sync
 * added that the snapshot predates stays too. Categories are guarded the same way.
 */
function installSidebar(state: State, sidebar: Sidebar, since: number): State {
  const rows: Record<number, SidebarRow> = {};
  const installed: SidebarRow[] = [];

  for (const row of sidebar.rows) {
    if (!touchedSince(state, row.room.id, since)) {
      rows[row.room.id] = row;
      installed.push(row);
    }
  }

  for (const row of Object.values(state.sidebar.rows)) {
    if (touchedSince(state, row.room.id, since)) {
      rows[row.room.id] = row;
    }
  }

  const listed = sidebar.rows.map((row) => row.room.id).filter((id) => rows[id] !== undefined);

  let next: State = {
    ...state,
    users: mergeUserList(state.users, sidebar.users),
    sidebar: {
      status: "ready",
      // A row sync added that the snapshot predates takes its sorted place, as an upsert would.
      order: listed.length === Object.keys(rows).length ? listed : sortSidebarOrder(rows),
      rows,
      categories: replyCategories(state, sidebar.categories, since),
      placeholderUserIds: sidebar.directPlaceholderUserIds,
      canCreateRooms: sidebar.canCreateRooms,
      overlay: state.sidebar.overlay,
    },
  };

  for (const row of installed) {
    next = setDetailRow(next, row);
  }

  return next;
}

function updateRow(state: State, roomId: number, change: (row: SidebarRow) => SidebarRow): State {
  const row = state.sidebar.rows[roomId];

  if (row === undefined) {
    return state;
  }

  return {
    ...state,
    sidebar: { ...state.sidebar, rows: { ...state.sidebar.rows, [roomId]: change(row) } },
  };
}

/**
 * The room was read here: its unread messages clear and the membership stops being unread, as
 * the server's row (published on the read) will say. Thread pings stay until their thread is
 * read, and the inbox keeps its own unread mentions.
 */
export function markRoomRead(state: State, roomId: number): State {
  return updateRow(state, roomId, (row) =>
    row.unreadCount === 0 &&
    row.notificationCount === row.threadNotificationCount &&
    row.membership.unreadAt === null
      ? row
      : {
          ...row,
          unreadCount: 0,
          notificationCount: row.threadNotificationCount,
          membership: { ...row.membership, unreadAt: null },
        },
  );
}

function timelineOf(state: State, roomId: number): Timeline {
  return state.timelines[roomId] ?? emptyTimeline;
}

function withTimeline(state: State, roomId: number, timeline: Timeline): State {
  return { ...state, timelines: { ...state.timelines, [roomId]: timeline } };
}

function withThreadTimeline(state: State, threadId: number, timeline: Timeline): State {
  return { ...state, threadTimelines: { ...state.threadTimelines, [threadId]: timeline } };
}

function roomView(
  detail: RoomDetail | null,
  status: RoomState["status"],
  error: string | null,
  preview: OpenRoomPreview | null,
): RoomState {
  return { detail, status, error, preview };
}

export function setRoomLoading(state: State, roomId: number): State {
  const room = state.rooms[roomId];

  return {
    ...state,
    rooms: {
      ...state.rooms,
      [roomId]: roomView(room?.detail ?? null, "loading", null, null),
    },
  };
}

export function setRoomError(state: State, roomId: number, error: string): State {
  const room = state.rooms[roomId];

  return {
    ...state,
    rooms: {
      ...state.rooms,
      [roomId]: roomView(room?.detail ?? null, "error", error, null),
    },
  };
}

/** The viewer can join this open room. The conversation stays unloaded. */
export function setRoomPreview(state: State, roomId: number, preview: OpenRoomPreview): State {
  return {
    ...state,
    rooms: {
      ...state.rooms,
      [roomId]: roomView(null, "ready", null, preview),
    },
  };
}

export function setRoomDetail(state: State, detail: RoomDetail): State {
  const roomId = detail.room.id;
  const timeline = timelineOf(state, roomId);

  return {
    ...state,
    users: mergeUserList(state.users, detail.users),
    rooms: { ...state.rooms, [roomId]: roomView(detail, "ready", null, null) },
    timelines: {
      ...state.timelines,
      [roomId]:
        timeline.status === "ready"
          ? timeline
          : {
              ...timeline,
              unreadFromId: detail.unread?.firstUnreadMessageId ?? null,
              unreadCount: detail.unread?.count ?? 0,
            },
    },
  };
}

/**
 * How a page lands: `replace` makes it the window; `older` and `newer` add it at an end;
 * `refresh` re-reads part of a window in place (a resync of a window away from the present): the
 * window's messages inside the page's span that the page lacks are gone, the rest stay. `resync`
 * lands a resync's newest page without moving the reader (see `landNewest`).
 */
export type PageMode = "replace" | "older" | "newer" | "refresh" | "resync";

type PageDirection = "older" | "newer";

function requestId(timeline: Timeline, direction: PageDirection): number {
  return direction === "older" ? timeline.olderRequest : timeline.newerRequest;
}

/** A directional result whose id was bumped (a replace, or a later page) must not land. */
function staleRequest(timeline: Timeline, mode: PageMode, request: number | undefined): boolean {
  if ((mode !== "older" && mode !== "newer") || request === undefined) {
    return false;
  }

  return requestId(timeline, mode) !== request;
}

function beginDirectional(timeline: Timeline, direction: PageDirection): Timeline {
  return direction === "older"
    ? { ...timeline, loadingOlder: true, olderRequest: timeline.olderRequest + 1 }
    : { ...timeline, loadingNewer: true, newerRequest: timeline.newerRequest + 1 };
}

/** Clears one direction's flag when `request` still owns it. */
function clearOwned(
  timeline: Timeline,
  direction: PageDirection,
  request: number | undefined,
): Timeline | undefined {
  if (requestId(timeline, direction) !== request) {
    return undefined;
  }

  return direction === "older"
    ? { ...timeline, loadingOlder: false }
    : { ...timeline, loadingNewer: false };
}

/** A page merged into the store, and the window it makes. */
interface LandedPage {
  readonly state: State;
  readonly timeline: Timeline;
}

/**
 * `ids` without the ones inside the page's span (first to last by time) that the page lacks:
 * deleted, or moved out of this conversation, since the window loaded.
 */
function withoutMissing(
  ids: readonly number[],
  pageIds: readonly number[],
  messages: Readonly<Record<number, MessageDTO>>,
): readonly number[] {
  const first = messages[pageIds[0] ?? -1];
  const last = messages[pageIds.at(-1) ?? -1];

  if (first === undefined || last === undefined) {
    return ids;
  }

  const onPage = new Set(pageIds);

  return ids.filter((id) => {
    const message = messages[id];

    if (onPage.has(id) || message === undefined) {
      return true;
    }

    return compareMessages(message, first) < 0 || compareMessages(message, last) > 0;
  });
}

/** The newest message the window held when its fresh page was asked for (live arrivals aside). */
function heldNewest(timeline: Timeline): number | undefined {
  const arrived = new Set(timeline.arrived ?? []);

  return timeline.ids.findLast((id) => !arrived.has(id));
}

/**
 * A resync's newest page, landed where the reader is. A window at the present that the page still
 * meets takes it in place (`refresh`), keeping the history above it and the reader's scroll
 * position. One the page no longer meets (more was posted meanwhile than a page holds, or its
 * newest message can't be placed) keeps its messages and now stops short of the present: the
 * timeline pages on towards it from where the reader is. A window away from the present is left
 * for the engine to re-read around its middle, and only a window with nothing loaded (or a room
 * emptied meanwhile) takes the page as a fresh one.
 */
function landNewest(state: State, timeline: Timeline, page: MessagePage): LandedPage {
  const newest = heldNewest(timeline);
  const held = state.messages[newest ?? -1];
  const oldest = page.messages[0];

  if (timeline.status !== "ready" || newest === undefined || oldest === undefined) {
    return landPage(state, timeline, page, "replace");
  }

  if (timeline.after !== null) {
    return { state, timeline };
  }

  if (held !== undefined && (page.before === null || compareMessages(oldest, held) <= 0)) {
    return landPage(state, timeline, page, "refresh");
  }

  // A gap between the window and the page: drop the live arrivals beyond it, which the pages
  // towards the present bring back in order.
  const arrived = new Set(timeline.arrived ?? []);

  return {
    state,
    timeline: {
      ...timeline,
      ids: timeline.ids.filter((id) => !arrived.has(id)),
      after: newest,
      arrived: null,
    },
  };
}

/** The page's messages merged into the store, and the window they make with `timeline`. */
function landPage(state: State, timeline: Timeline, page: MessagePage, mode: PageMode): LandedPage {
  if (mode === "resync") {
    return landNewest(state, timeline, page);
  }

  const messages = { ...state.messages };
  let reconciled = state;

  for (const message of page.messages) {
    reconciled = removePending(reconciled, message.clientMessageId);

    const held = messages[message.id];

    if (state.tombstones[message.id] !== undefined) {
      continue;
    }

    // A stale page must not undo a newer live edit (nor an older poll or cards a newer one).
    const kept = reconcileMessage(held, message, true);

    if (kept !== held) {
      messages[message.id] = kept;
    }
  }

  const pageIds = page.messages
    .filter((message) => state.tombstones[message.id] === undefined)
    .map((message) => message.id);

  const insertAll = (into: readonly number[], add: readonly number[]) => {
    let merged = into;

    for (const id of add) {
      const message = messages[id];

      if (message !== undefined && state.tombstones[id] === undefined) {
        merged = insertOrdered(merged, message, messages);
      }
    }

    return merged;
  };

  let ids: readonly number[];
  let { before, after } = timeline;

  if (mode === "replace") {
    // Messages that arrived live while it loaded join a window that reaches the present.
    ids = page.after === null ? insertAll(pageIds, timeline.arrived ?? []) : pageIds;
    before = page.before;
    after = page.after;
  } else if (mode === "refresh") {
    ids = insertAll(withoutMissing(timeline.ids, pageIds, messages), pageIds);

    const first = ids[0];
    const last = ids.at(-1);

    // The page reaching past either end of the window moves that end's cursor too.
    if (first !== undefined && pageIds[0] === first) {
      before = page.before;
    }

    if (last !== undefined && pageIds.at(-1) === last) {
      after = page.after;
    }
  } else {
    ids = insertAll(timeline.ids, pageIds);
    before = mode === "newer" ? timeline.before : page.before;
    after = mode === "older" ? timeline.after : page.after;
  }

  const fresh = mode === "replace" || mode === "refresh";

  return {
    state: {
      ...reconciled,
      messages,
      users: mergeUserList(state.users, page.users),
      saved: mergeSavedMarks(state.saved, page),
    },
    timeline: {
      ...timeline,
      ids,
      status: "ready",
      before,
      after,
      // A fresh window ends the load that raised a flag and retires both directional requests,
      // so a page still in flight cannot clear the next one's flag or move this cursor.
      // Leaving `loadingNewer` set blocks every later forward page (the pane sets it while a
      // permalink's window loads).
      loadingOlder: mode === "replace" || mode === "older" ? false : timeline.loadingOlder,
      loadingNewer: mode === "replace" || mode === "newer" ? false : timeline.loadingNewer,
      olderRequest: mode === "replace" ? timeline.olderRequest + 1 : timeline.olderRequest,
      newerRequest: mode === "replace" ? timeline.newerRequest + 1 : timeline.newerRequest,
      generation: mode === "replace" ? timeline.generation + 1 : timeline.generation,
      arrived: fresh ? null : timeline.arrived,
    },
  };
}

/** Lands a page of messages: a fresh window, or one more page on either end. */
export function applyPage(
  state: State,
  roomId: number,
  page: MessagePage,
  mode: PageMode,
  request?: number,
): State {
  const timeline = timelineOf(state, roomId);

  if (staleRequest(timeline, mode, request)) {
    return state;
  }

  const landed = landPage(state, timeline, page, mode);

  return withTimeline(landed.state, roomId, landed.timeline);
}

/** Lands a page of a thread's replies, as `applyPage` does for a room. */
export function applyThreadPage(
  state: State,
  threadId: number,
  page: MessagePage,
  mode: PageMode,
  request?: number,
): State {
  const timeline = state.threadTimelines[threadId] ?? emptyTimeline;

  if (staleRequest(timeline, mode, request)) {
    return state;
  }

  const landed = landPage(state, timeline, page, mode);

  return withThreadTimeline(landed.state, threadId, landed.timeline);
}

export function setPageLoading(state: State, roomId: number, direction: PageDirection): State {
  return withTimeline(state, roomId, beginDirectional(timelineOf(state, roomId), direction));
}

/**
 * A fresh window (open, retry, resync, jump to present) is on its way: live messages that arrive
 * before it lands are held for it.
 */
export function setPageReplacing(state: State, roomId: number): State {
  return withTimeline(state, roomId, { ...timelineOf(state, roomId), arrived: [] });
}

/** As `setPageReplacing`, for a thread's replies. */
export function setThreadPageReplacing(state: State, threadId: number): State {
  const timeline = state.threadTimelines[threadId] ?? emptyTimeline;

  return withThreadTimeline(state, threadId, { ...timeline, arrived: [] });
}

export function setPageFailed(
  state: State,
  roomId: number,
  direction?: PageDirection,
  request?: number,
): State {
  const timeline = timelineOf(state, roomId);

  if (direction !== undefined) {
    const cleared = clearOwned(timeline, direction, request);

    return cleared === undefined ? state : withTimeline(state, roomId, cleared);
  }

  return withTimeline(state, roomId, {
    ...timeline,
    arrived: null,
    loadingOlder: false,
    loadingNewer: false,
    status: timeline.status === "loading" ? "error" : timeline.status,
  });
}

export function setThreadPageLoading(
  state: State,
  threadId: number,
  direction: PageDirection,
): State {
  const timeline = state.threadTimelines[threadId] ?? emptyTimeline;

  return withThreadTimeline(state, threadId, {
    ...beginDirectional(timeline, direction),
    status: timeline.status === "idle" ? "loading" : timeline.status,
  });
}

export function setThreadPageFailed(
  state: State,
  threadId: number,
  direction?: PageDirection,
  request?: number,
): State {
  const timeline = state.threadTimelines[threadId] ?? emptyTimeline;

  if (direction !== undefined) {
    const cleared = clearOwned(timeline, direction, request);

    return cleared === undefined ? state : withThreadTimeline(state, threadId, cleared);
  }

  return withThreadTimeline(state, threadId, {
    ...timeline,
    arrived: null,
    loadingOlder: false,
    loadingNewer: false,
    status: timeline.status === "loading" || timeline.status === "idle" ? "error" : timeline.status,
  });
}

/**
 * The in-flight page was dropped. `replace` is a fresh window: it also releases replies held for
 * that window. A directional page clears only its own flag, and only while its request id is
 * still current, so a superseded interrupt cannot release the page that replaced it.
 */
export function clearThreadPageLoading(
  state: State,
  threadId: number,
  direction: PageDirection | "replace",
  request?: number,
): State {
  const timeline = state.threadTimelines[threadId];

  if (timeline === undefined) {
    return state;
  }

  if (direction !== "replace") {
    const cleared = clearOwned(timeline, direction, request);

    return cleared === undefined ? state : withThreadTimeline(state, threadId, cleared);
  }

  return withThreadTimeline(state, threadId, {
    ...timeline,
    arrived: null,
    loadingOlder: false,
    loadingNewer: false,
  });
}

/**
 * "Mark unread from here": the divider moves to `fromId`, counting it and every loaded root
 * message after it. A message outside the loaded window leaves the divider alone.
 */
export function moveUnreadDivider(state: State, roomId: number, fromId: number): State {
  const timeline = state.timelines[roomId];
  const index = timeline?.ids.indexOf(fromId) ?? -1;

  if (timeline === undefined || index < 0) {
    return state;
  }

  return withTimeline(state, roomId, {
    ...timeline,
    unreadFromId: fromId,
    unreadCount: timeline.ids.length - index,
  });
}

/**
 * "Mark unread from here", confirmed: the divider moves to `fromId` and the sidebar row counts
 * what the divider counts (at least 1, when the message is outside the loaded window). `since` is
 * the request's ticket. A reply that started before a resync, a newer mark-unread reply moving
 * the divider, or a sync `room.read` (another tab read the room) changes nothing, divider
 * included. Any other row change sync made after the request began is newer, so the row stays,
 * but the divider still moves: sync never moves it, and the viewer's own `room.unread` echo
 * often lands first.
 */
export function markUnreadFrom(
  state: State,
  roomId: number,
  fromId: number,
  now: number,
  since: number,
): State {
  if (
    isStale(state, since) ||
    dividerMovedSince(state, roomId, since) ||
    readSince(state, roomId, since)
  ) {
    return state;
  }

  const moved = claimDivider(moveUnreadDivider(state, roomId, fromId), roomId);
  const timeline = moved.timelines[roomId];
  const count = timeline?.unreadFromId === fromId ? timeline.unreadCount : 0;

  if (touchedSince(moved, roomId, since)) {
    return moved;
  }

  return updateRow(moved, roomId, (row) => ({
    ...row,
    unreadCount: Math.max(count, 1),
    membership: {
      ...row.membership,
      unreadAt: row.membership.unreadAt ?? new Date(now).toISOString(),
    },
  }));
}

/** Forgets the divider once the room is left, so the next visit computes a fresh one. */
export function clearUnreadDivider(state: State, roomId: number): State {
  const timeline = state.timelines[roomId];

  if (timeline === undefined || timeline.unreadFromId === null) {
    return state;
  }

  return withTimeline(state, roomId, { ...timeline, unreadFromId: null, unreadCount: 0 });
}

function removePending(state: State, clientMessageId: string): State {
  const pending = state.pending[clientMessageId];

  if (pending === undefined) {
    return state;
  }

  const { [clientMessageId]: _gone, ...rest } = state.pending;

  const others = (ids: readonly string[] | undefined) =>
    (ids ?? []).filter((id) => id !== clientMessageId);

  if (pending.threadId !== null) {
    return {
      ...state,
      pending: rest,
      pendingByThread: {
        ...state.pendingByThread,
        [pending.threadId]: others(state.pendingByThread[pending.threadId]),
      },
    };
  }

  return {
    ...state,
    pending: rest,
    pendingByRoom: {
      ...state.pendingByRoom,
      [pending.roomId]: others(state.pendingByRoom[pending.roomId]),
    },
  };
}

/**
 * A confirmed message (the `POST` reply or the `message.created` event, whichever is first):
 * replaces its pending row and joins its timeline (the room's, or its thread's) if that window
 * reaches the present or its pending row was visible while a fresh page loads. The second
 * arrival is a no-op unless it's newer.
 */
export function receiveMessage(state: State, message: MessageDTO): State {
  if (state.tombstones[message.id] !== undefined) {
    return removePending(state, message.clientMessageId);
  }

  const reconciled = removePending(state, message.clientMessageId);
  const held = reconciled.messages[message.id];

  const kept = reconcileMessage(held, message, false);

  if (held !== undefined && kept === held) {
    return reconciled;
  }

  const messages = { ...reconciled.messages, [message.id]: kept };
  const threadId = message.threadId;

  const timeline =
    threadId === null ? reconciled.timelines[message.roomId] : reconciled.threadTimelines[threadId];

  if (timeline === undefined) {
    return held === undefined ? reconciled : { ...reconciled, messages };
  }

  // A fresh window is loading: hold the message for it, whatever the window shows meanwhile.
  const waiting =
    timeline.arrived === null
      ? timeline
      : {
          ...timeline,
          arrived: [...timeline.arrived.filter((id) => id !== message.id), message.id],
        };

  // Keep a sent row visible while its fresh page loads. Other arrivals stay beyond history.
  const visibleSend =
    timeline.arrived !== null && state.pending[message.clientMessageId] !== undefined;

  if (timeline.status !== "ready" || (timeline.after !== null && !visibleSend)) {
    if (waiting === timeline) {
      return held === undefined ? reconciled : { ...reconciled, messages };
    }

    return threadId === null
      ? withTimeline({ ...reconciled, messages }, message.roomId, waiting)
      : withThreadTimeline({ ...reconciled, messages }, threadId, waiting);
  }

  const next = { ...waiting, ids: insertOrdered(timeline.ids, message, messages) };

  return threadId === null
    ? withTimeline({ ...reconciled, messages }, message.roomId, next)
    : withThreadTimeline({ ...reconciled, messages }, threadId, next);
}

/**
 * An edit lands only if it's newer than the copy held (and the message is held at all); its poll
 * and cards each by their `asOf`.
 */
export function updateMessage(state: State, message: MessageDTO): State {
  const held = state.messages[message.id];

  if (held === undefined) {
    return state;
  }

  const kept = reconcileMessage(held, message, false);

  return kept === held ? state : { ...state, messages: { ...state.messages, [message.id]: kept } };
}

export function removeMessage(
  state: State,
  messageId: number,
  roomId: number,
  threadId: number | null,
  now: number,
): State {
  const { [messageId]: _gone, ...messages } = state.messages;
  const tombstones = { ...state.tombstones, [messageId]: now + TOMBSTONE_TTL_MS };
  const { [messageId]: _unsaved, ...saved } = state.saved;
  let next: State = { ...state, messages, tombstones, saved };
  const timeline = state.timelines[roomId];

  if (timeline !== undefined) {
    next = withTimeline(next, roomId, {
      ...timeline,
      ids: timeline.ids.filter((id) => id !== messageId),
    });
  }

  const threadTimeline = threadId === null ? undefined : state.threadTimelines[threadId];

  if (threadId !== null && threadTimeline !== undefined) {
    next = withThreadTimeline(next, threadId, {
      ...threadTimeline,
      ids: threadTimeline.ids.filter((id) => id !== messageId),
    });
  }

  return next;
}

export function addPending(state: State, pending: PendingMessage): State {
  const all = { ...state.pending, [pending.clientMessageId]: pending };

  if (pending.threadId !== null) {
    return {
      ...state,
      pending: all,
      pendingByThread: {
        ...state.pendingByThread,
        [pending.threadId]: [
          ...(state.pendingByThread[pending.threadId] ?? []),
          pending.clientMessageId,
        ],
      },
    };
  }

  return {
    ...state,
    pending: all,
    pendingByRoom: {
      ...state.pendingByRoom,
      [pending.roomId]: [...(state.pendingByRoom[pending.roomId] ?? []), pending.clientMessageId],
    },
  };
}

export function setPendingState(
  state: State,
  clientMessageId: string,
  status: PendingMessage["state"],
  error: string | null,
): State {
  const pending = state.pending[clientMessageId];

  if (pending === undefined) {
    return state;
  }

  return {
    ...state,
    pending: { ...state.pending, [clientMessageId]: { ...pending, state: status, error } },
  };
}

export function discardPending(state: State, clientMessageId: string): State {
  return removePending(state, clientMessageId);
}

function setTyping(state: State, topic: string, userId: number, on: boolean, now: number): State {
  const typists = state.typing[topic] ?? {};

  if (!on && typists[userId] === undefined) {
    return state;
  }

  const { [userId]: _previous, ...others } = typists;

  return {
    ...state,
    typing: {
      ...state.typing,
      [topic]: on ? { ...others, [userId]: now + TYPING_TTL_MS } : others,
    },
  };
}

function upsertRow(state: State, row: SidebarRow): State {
  const rows = { ...state.sidebar.rows, [row.room.id]: row };
  const known = state.sidebar.rows[row.room.id] !== undefined;
  const renamed = known && state.sidebar.rows[row.room.id]?.displayName !== row.displayName;

  return setDetailRow(
    {
      ...state,
      sidebar: {
        ...state.sidebar,
        rows,
        order: known && !renamed ? state.sidebar.order : sortSidebarOrder(rows),
      },
    },
    row,
  );
}

/** A synced row updates cached header facts without replacing its roster or unread divider. */
function setDetailRow(state: State, row: SidebarRow): State {
  const loaded = state.rooms[row.room.id];

  if (loaded?.detail === null || loaded?.detail === undefined) {
    return state;
  }

  return {
    ...state,
    rooms: {
      ...state.rooms,
      [row.room.id]: {
        ...loaded,
        // The patch retired the read that had set loading; that read must not land later.
        status: loaded.status === "loading" ? "ready" : loaded.status,
        detail: {
          ...loaded.detail,
          room: row.room,
          membership: row.membership,
          displayName: row.displayName,
          directMemberIds: row.directMemberIds,
        },
      },
    },
  };
}

/**
 * A successful local delete, leave, or self-removal establishes that access was revoked: the room
 * shows as unavailable, and its sidebar row leaves unless `keepRow` (the row is newer than the
 * reply that said so).
 */
export function setRoomUnavailable(state: State, roomId: number, keepRow = false): State {
  const next = keepRow ? state : removeRow(state, roomId);

  return {
    ...next,
    rooms: {
      ...next.rooms,
      [roomId]: roomView(null, "error", "This room is no longer available", null),
    },
  };
}

function removeRow(state: State, roomId: number): State {
  if (state.sidebar.rows[roomId] === undefined) {
    return state;
  }

  const { [roomId]: _gone, ...rows } = state.sidebar.rows;

  return {
    ...state,
    sidebar: { ...state.sidebar, rows, order: state.sidebar.order.filter((id) => id !== roomId) },
  };
}

/**
 * Whether a new root message would push under the classic policy, as the server's
 * `notificationCount` counts it: every one in an `everything` room, a mention in a `mentions` or
 * `muted` one, none in a `nothing` one. A reply to you arrives with the row the server sends
 * next; a keyword alert alone never pushes, so it never counts.
 */
function notifies(row: SidebarRow, mentioned: boolean): boolean {
  switch (row.membership.involvement) {
    case "everything":
      return true;
    case "mentions":
    case "muted":
      return mentioned;
    default:
      return false;
  }
}

function roomUnread(
  state: State,
  roomId: number,
  messageId: number | null,
  mentioned: boolean,
  now: number,
): State {
  return updateRow(state, roomId, (row) => ({
    ...row,
    unreadCount: messageId === null ? Math.max(row.unreadCount, 1) : row.unreadCount + 1,
    mentionCount: row.mentionCount + (mentioned ? 1 : 0),
    notificationCount:
      row.notificationCount + (messageId !== null && notifies(row, mentioned) ? 1 : 0),
    membership: {
      ...row.membership,
      unreadAt: row.membership.unreadAt ?? new Date(now).toISOString(),
    },
  }));
}

/**
 * Applies one batch of sync events, in order, as a single state change. Rows the batch changes
 * are touched, so an HTTP reply older than them leaves them be. A `reply` batch is an HTTP
 * reply's rows put through the same reducers (already filtered by `untouchedReplyEvents`), so it
 * touches nothing.
 */
export function applyEvents(
  state: State,
  events: readonly SyncEvent[],
  now: number,
  source: "sync" | "reply" = "sync",
): State {
  const meId = state.me?.user.id ?? state.boot?.user.id ?? null;
  let next = state;

  for (const event of events) {
    switch (event.type) {
      case "message.created":
        next = stopTypingFor(receiveMessage(next, event.data), event.topic, event.data.creatorId);
        break;
      case "message.updated":
        next = updateMessage(next, event.data);
        break;
      case "message.removed":
        next = dropSavedForMessage(
          removeMessage(next, event.data.id, event.data.roomId, event.data.threadId, now),
          event.data.id,
        );
        break;
      case "message.reactions":
        next = setReactions(next, event.data);
        break;
      case "message.pinned":
        next = setPinState(next, event.data);
        break;
      case "saved.changed":
        next = applySavedChange(next, event.data.messageId, event.data.item);
        break;
      case "activity.item":
        next = approvalRequested(
          applyActivityItem(next, event.data.item, event.data),
          event.data.item,
        );
        break;
      case "activity.removed":
        next = removeActivityItem(next, event.data.id, event.data);
        break;
      case "scheduled.changed":
        next = applyScheduled(next, event.data);
        break;
      case "scheduled.removed":
        next = removeScheduled(next, event.data.id);
        break;
      case "thread.indicator":
        next = setThreadIndicator(next, event.data);
        break;
      case "thread.created":
        next = addBoardPost(receiveWorkThread(next, event.data), event.data);
        break;
      case "thread.updated":
        next = receiveWorkThread(next, event.data);
        break;
      case "thread.removed":
        next = removeThread(next, event.data.threadId, event.data.roomId);
        break;
      case "board.automations.changed":
        next = boardAutomationsChanged(next, event.data.roomId);
        break;
      case "thread.unread":
        next = event.data.refreshOnly
          ? next
          : setThreadUnread(next, event.data.threadId, new Date(now).toISOString());
        break;
      case "thread.read":
        next = setThreadUnread(next, event.data.threadId, null);
        break;
      case "typing":
        next =
          event.data.userId === meId
            ? next
            : setTyping(next, event.topic, event.data.userId, event.data.on, now);
        break;
      case "room.unread":
        next = roomUnread(next, event.data.roomId, event.data.messageId, event.data.mentioned, now);
        break;
      case "room.read":
        next = markRoomRead(next, event.data.roomId);
        break;
      case "sidebar.row.upserted":
        next = upsertRow(next, event.data);
        break;
      case "sidebar.row.removed":
        next = removeRow(next, event.data.roomId);
        break;
      case "sidebar.category.upserted":
        next = upsertCategory(next, event.data);
        break;
      case "sidebar.category.removed":
        next = removeCategory(next, event.data.id);
        break;
      case "presence":
        next = setPresence(next, [event.data]);
        break;
      case "agent.status":
        next = applyAgentStatus(next, event.data);
        break;
      case "agent.steps":
        next = applyAgentSteps(next, event.data);
        break;
      case "approval.updated":
        next = applyApprovalUpdated(next, event.data);
        break;
      case "huddle.presence":
        next = setHuddlePresence(next, event.data);
        break;
      case "stage.updated":
        next = setStage(next, event.data);
        break;
      case "poll.updated":
        next = applyPoll(next, event.data.poll);
        break;
      case "poll.ballot":
        next = applyPollBallot(next, event.data);
        break;
      case "message.cards":
        next = applyMessageCards(next, event.data);
        break;
      case "workspace.updated":
        next = setWorkspaceBranding(next, event.data);
        break;
      case "workspace.styles.updated":
        next = setWorkspaceStyles(next, event.data.css);
        break;
    }
  }

  return source === "sync"
    ? touchRows(
        next,
        syncTouchedRooms(state, next, events),
        syncTouchedCategories(events),
        syncReadRooms(events),
      )
    : next;
}

/**
 * The rooms a sync batch touched: every room a `sidebar.row.*`, `room.read` or `room.unread`
 * event names, whether or not it changed anything (a row added and removed in one batch, or a
 * removal of a row already gone, still outranks an older reply that lists it), and every row
 * the batch's other events changed (a new message's count).
 */
function syncTouchedRooms(
  before: State,
  after: State,
  events: readonly SyncEvent[],
): readonly number[] {
  const ids = new Set(changedRowIds(before.sidebar.rows, after.sidebar.rows));

  for (const event of events) {
    switch (event.type) {
      case "sidebar.row.upserted":
        ids.add(event.data.room.id);
        break;
      case "sidebar.row.removed":
      case "room.read":
      case "room.unread":
        ids.add(event.data.roomId);
        break;
    }
  }

  return [...ids];
}

/** The categories a sync batch names, so an older category reply leaves them be. */
function syncTouchedCategories(events: readonly SyncEvent[]): readonly number[] {
  return events.flatMap((event) =>
    event.type === "sidebar.category.upserted" || event.type === "sidebar.category.removed"
      ? [event.data.id]
      : [],
  );
}

/** The rooms a sync batch says were read, so an older mark-unread reply leaves them be. */
function syncReadRooms(events: readonly SyncEvent[]): readonly number[] {
  return events.flatMap((event) => (event.type === "room.read" ? [event.data.roomId] : []));
}

/** A typist who posted stops typing at once (their message is the end of it). */
export function stopTypingFor(state: State, topic: string, userId: number): State {
  return setTyping(state, topic, userId, false, 0);
}

/** Drops typists and tombstones whose time is up. */
export function prune(state: State, now: number): State {
  let typingChanged = false;
  const typing: Record<string, Readonly<Record<number, number>>> = {};

  for (const [topic, typists] of Object.entries(state.typing)) {
    const kept: Record<number, number> = {};

    for (const [userId, expiresAt] of Object.entries(typists)) {
      if (expiresAt > now) {
        kept[Number(userId)] = expiresAt;
      } else {
        typingChanged = true;
      }
    }

    typing[topic] = kept;
  }

  let tombstonesChanged = false;
  const tombstones: Record<number, number> = {};

  for (const [id, expiresAt] of Object.entries(state.tombstones)) {
    if (expiresAt > now) {
      tombstones[Number(id)] = expiresAt;
    } else {
      tombstonesChanged = true;
    }
  }

  if (!typingChanged && !tombstonesChanged) {
    return state;
  }

  return {
    ...state,
    typing: typingChanged ? typing : state.typing,
    tombstones: tombstonesChanged ? tombstones : state.tombstones,
  };
}

/** When the soonest typing entry or tombstone lapses (ms), or `null` when none is held. */
export function nextExpiry(state: State): number | null {
  let soonest: number | null = null;

  for (const typists of Object.values(state.typing)) {
    for (const expiresAt of Object.values(typists)) {
      soonest = soonest === null ? expiresAt : Math.min(soonest, expiresAt);
    }
  }

  for (const expiresAt of Object.values(state.tombstones)) {
    soonest = soonest === null ? expiresAt : Math.min(soonest, expiresAt);
  }

  return soonest;
}
