/**
 * Pure state transitions. Each takes the current state (and the time, so tests and the sync
 * engine's clock agree) and returns the next. `store.ts` wraps them in `setState`.
 */
import type {
  Me,
  MessageDTO,
  MessagePage,
  PendingMessage,
  RoomDetail,
  Sidebar,
  SidebarRow,
  SyncEvent,
  Timeline,
  User,
  UserPresence,
} from "./model.ts";
import { emptyTimeline, type State, TOMBSTONE_TTL_MS, TYPING_TTL_MS } from "./state.ts";

/** `(createdAt, id)`: the server's timeline order. */
export function compareMessages(left: MessageDTO, right: MessageDTO): number {
  if (left.createdAt !== right.createdAt) {
    return left.createdAt < right.createdAt ? -1 : 1;
  }

  return left.id - right.id;
}

/** Inserts `message` into ordered `ids` (no-op when present). */
function insertOrdered(
  ids: readonly number[],
  message: MessageDTO,
  messages: Readonly<Record<number, MessageDTO>>,
): readonly number[] {
  if (ids.includes(message.id)) {
    return ids;
  }

  let low = 0;
  let high = ids.length;

  while (low < high) {
    const middle = (low + high) >>> 1;
    const other = messages[ids[middle] ?? -1];

    if (other !== undefined && compareMessages(other, message) < 0) {
      low = middle + 1;
    } else {
      high = middle;
    }
  }

  return [...ids.slice(0, low), message.id, ...ids.slice(low)];
}

function mergeUserList(users: State["users"], list: readonly User[]): State["users"] {
  if (list.length === 0) {
    return users;
  }

  const next = { ...users };

  for (const user of list) {
    next[user.id] = user;
  }

  return next;
}

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

export function loadSidebar(state: State, sidebar: Sidebar): State {
  const rows: Record<number, SidebarRow> = {};

  for (const row of sidebar.rows) {
    rows[row.room.id] = row;
  }

  return {
    ...state,
    users: mergeUserList(state.users, sidebar.users),
    sidebar: {
      status: "ready",
      order: sidebar.rows.map((row) => row.room.id),
      rows,
      categories: sidebar.categories,
      placeholderUserIds: sidebar.directPlaceholderUserIds,
      canCreateRooms: sidebar.canCreateRooms,
    },
  };
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

/** The room was read here: counts clear and the membership stops being unread. */
export function markRoomRead(state: State, roomId: number): State {
  return updateRow(state, roomId, (row) =>
    row.unreadCount === 0 && row.mentionCount === 0 && row.membership.unreadAt === null
      ? row
      : {
          ...row,
          unreadCount: 0,
          mentionCount: 0,
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

export function setRoomLoading(state: State, roomId: number): State {
  const room = state.rooms[roomId];

  return {
    ...state,
    rooms: {
      ...state.rooms,
      [roomId]: { detail: room?.detail ?? null, status: "loading", error: null },
    },
  };
}

export function setRoomError(state: State, roomId: number, error: string): State {
  const room = state.rooms[roomId];

  return {
    ...state,
    rooms: { ...state.rooms, [roomId]: { detail: room?.detail ?? null, status: "error", error } },
  };
}

export function setRoomDetail(state: State, detail: RoomDetail): State {
  const roomId = detail.room.id;
  const timeline = timelineOf(state, roomId);

  return {
    ...state,
    users: mergeUserList(state.users, detail.users),
    rooms: { ...state.rooms, [roomId]: { detail, status: "ready", error: null } },
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

export type PageMode = "replace" | "older" | "newer";

/** Lands a page of messages: a fresh window, or one more page on either end. */
export function applyPage(state: State, roomId: number, page: MessagePage, mode: PageMode): State {
  const messages = { ...state.messages };

  for (const message of page.messages) {
    const held = messages[message.id];

    if (state.tombstones[message.id] !== undefined) {
      continue;
    }

    // A stale page must not undo a newer live edit.
    if (held === undefined || held.updatedAt <= message.updatedAt) {
      messages[message.id] = message;
    }
  }

  const timeline = timelineOf(state, roomId);

  const pageIds = page.messages
    .filter((message) => state.tombstones[message.id] === undefined)
    .map((message) => message.id);

  let ids: readonly number[];

  if (mode === "replace") {
    ids = pageIds;
  } else {
    ids = timeline.ids;

    for (const id of pageIds) {
      const message = messages[id];

      if (message !== undefined) {
        ids = insertOrdered(ids, message, messages);
      }
    }
  }

  const next: Timeline = {
    ...timeline,
    ids,
    status: "ready",
    before: mode === "newer" ? timeline.before : page.before,
    after: mode === "older" ? timeline.after : page.after,
    loadingOlder: mode === "older" ? false : timeline.loadingOlder,
    loadingNewer: mode === "newer" ? false : timeline.loadingNewer,
    generation: mode === "replace" ? timeline.generation + 1 : timeline.generation,
  };

  return withTimeline(
    { ...state, messages, users: mergeUserList(state.users, page.users) },
    roomId,
    next,
  );
}

export function setPageLoading(state: State, roomId: number, direction: "older" | "newer"): State {
  const timeline = timelineOf(state, roomId);

  return withTimeline(
    state,
    roomId,
    direction === "older"
      ? { ...timeline, loadingOlder: true }
      : { ...timeline, loadingNewer: true },
  );
}

export function setPageFailed(state: State, roomId: number): State {
  const timeline = timelineOf(state, roomId);

  return withTimeline(state, roomId, {
    ...timeline,
    loadingOlder: false,
    loadingNewer: false,
    status: timeline.status === "loading" ? "error" : timeline.status,
  });
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

  return {
    ...state,
    pending: rest,
    pendingByRoom: {
      ...state.pendingByRoom,
      [pending.roomId]: (state.pendingByRoom[pending.roomId] ?? []).filter(
        (id) => id !== clientMessageId,
      ),
    },
  };
}

/**
 * A confirmed message (the `POST` reply or the `message.created` event, whichever is first):
 * replaces its pending row and joins the timeline if the window reaches the present. The second
 * arrival is a no-op unless it's newer.
 */
export function receiveMessage(state: State, message: MessageDTO): State {
  if (state.tombstones[message.id] !== undefined || message.threadId !== null) {
    return removePending(state, message.clientMessageId);
  }

  const reconciled = removePending(state, message.clientMessageId);
  const held = reconciled.messages[message.id];

  if (held !== undefined && held.updatedAt >= message.updatedAt) {
    return reconciled;
  }

  const messages = { ...reconciled.messages, [message.id]: message };
  const timeline = reconciled.timelines[message.roomId];

  // Not loaded, or the window stops short of the present: the message is beyond it.
  if (timeline === undefined || timeline.status !== "ready" || timeline.after !== null) {
    return held === undefined ? reconciled : { ...reconciled, messages };
  }

  return withTimeline({ ...reconciled, messages }, message.roomId, {
    ...timeline,
    ids: insertOrdered(timeline.ids, message, messages),
  });
}

/** An edit lands only if it's newer than the copy held (and the message is held at all). */
export function updateMessage(state: State, message: MessageDTO): State {
  const held = state.messages[message.id];

  if (held === undefined || held.updatedAt >= message.updatedAt) {
    return state;
  }

  return { ...state, messages: { ...state.messages, [message.id]: message } };
}

export function removeMessage(state: State, messageId: number, roomId: number, now: number): State {
  const { [messageId]: _gone, ...messages } = state.messages;
  const timeline = state.timelines[roomId];
  const tombstones = { ...state.tombstones, [messageId]: now + TOMBSTONE_TTL_MS };

  if (timeline === undefined) {
    return { ...state, messages, tombstones };
  }

  return withTimeline({ ...state, messages, tombstones }, roomId, {
    ...timeline,
    ids: timeline.ids.filter((id) => id !== messageId),
  });
}

export function addPending(state: State, pending: PendingMessage): State {
  return {
    ...state,
    pending: { ...state.pending, [pending.clientMessageId]: pending },
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

  return {
    ...state,
    sidebar: {
      ...state.sidebar,
      rows,
      order: known && !renamed ? state.sidebar.order : sortSidebarOrder(rows),
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
    membership: {
      ...row.membership,
      unreadAt: row.membership.unreadAt ?? new Date(now).toISOString(),
    },
  }));
}

/** Applies one batch of sync events, in order, as a single state change. */
export function applyEvents(state: State, events: readonly SyncEvent[], now: number): State {
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
        next = removeMessage(next, event.data.id, event.data.roomId, now);
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
      case "presence":
        next = setPresence(next, [event.data]);
        break;
    }
  }

  return next;
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
