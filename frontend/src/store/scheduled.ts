/**
 * The viewer's scheduled messages in the store: every one seen, by id, and keyset-paged lists:
 * `pending` (upcoming and stranded, soonest first), `past` (sent or dropped, most recent first)
 * and `room:<id>` (one room's pending ones, soonest first: the composer's "N scheduled"). A
 * message that's sent or dropped moves from the pending lists to Past at once.
 */
import type { ScheduledMessage } from "../gen/ScheduledMessage.ts";
import type { ScheduledMessageFilter } from "../gen/ScheduledMessageFilter.ts";
import type { ScheduledMessageList } from "../gen/ScheduledMessageList.ts";
import { mergeConversationNames } from "./conversations.ts";
import {
  type Cursor,
  eachPaged,
  emptyPagedList,
  type IdOrder,
  type PagedList,
  pagedFailed,
  pagedLanded,
  pagedLoading,
  pagedLoadingMore,
  pagedPlaced,
  pagedStale,
  pagedWithout,
} from "./paged-list.ts";
import type { State } from "./state.ts";

/** Which list: every pending one, every past one, or one room's pending ones. */
export type ScheduledListKey = ScheduledMessageFilter | `room:${number}`;

export interface ScheduledSlice {
  /** By scheduled message id. */
  readonly items: Readonly<Record<number, ScheduledMessage>>;
  /** By `ScheduledListKey`. */
  readonly lists: Readonly<Record<string, PagedList>>;
}

export const emptyScheduled: ScheduledSlice = { items: {}, lists: {} };

/** The key of one room's pending list (the composer's). */
export function roomScheduledKey(roomId: number): ScheduledListKey {
  return `room:${roomId}`;
}

/** What a list asks the server for. */
export interface ScheduledQuery {
  readonly status: ScheduledMessageFilter;
  /** One room's, or `null` for every room's. */
  readonly roomId: number | null;
}

/** What a key asks the server for: the filter and the room, if one. */
export function scheduledQueryOf(key: ScheduledListKey): ScheduledQuery {
  if (key === "pending" || key === "past") {
    return { status: key, roomId: null };
  }

  return { status: "pending", roomId: Number(key.slice("room:".length)) };
}

/** Not yet sent or dropped (a `sending` one is still pending to the list). */
export function isPendingScheduled(message: ScheduledMessage): boolean {
  return message.state === "pending" || message.state === "sending";
}

function belongsTo(key: string, message: ScheduledMessage): boolean {
  if (key === "past") {
    return !isPendingScheduled(message);
  }

  if (key === "pending") {
    return isPendingScheduled(message);
  }

  return isPendingScheduled(message) && key === roomScheduledKey(message.roomId);
}

/** Pending lists: soonest first; Past: most recent first. Ties by id the same way. */
function scheduledOrder(items: ScheduledSlice["items"], key: string): IdOrder {
  const direction = key === "past" ? -1 : 1;

  return (left, right) => {
    const a = items[left];
    const b = items[right];

    if (a === undefined || b === undefined) {
      return 0;
    }

    if (a.sendAt !== b.sendAt) {
      return (a.sendAt < b.sendAt ? -1 : 1) * direction;
    }

    return (left - right) * direction;
  };
}

/** One list, or an empty one never loaded. */
export function scheduledListOf(state: State, key: ScheduledListKey): PagedList {
  return state.scheduled.lists[key] ?? emptyPagedList;
}

function updateList(
  state: State,
  key: ScheduledListKey,
  change: (list: PagedList) => PagedList,
): State {
  return {
    ...state,
    scheduled: {
      ...state.scheduled,
      lists: { ...state.scheduled.lists, [key]: change(scheduledListOf(state, key)) },
    },
  };
}

export function setScheduledListLoading(state: State, key: ScheduledListKey, more: boolean): State {
  return updateList(state, key, more ? pagedLoadingMore : pagedLoading);
}

export function setScheduledListFailed(state: State, key: ScheduledListKey, error: string): State {
  return updateList(state, key, (list) => pagedFailed(list, error));
}

/** A page of one list landed: its messages and conversation names join the store. */
export function landScheduledPage(
  state: State,
  key: ScheduledListKey,
  page: ScheduledMessageList,
  mode: "replace" | "more",
): State {
  const items = { ...state.scheduled.items };

  for (const message of page.scheduledMessages) {
    items[message.id] = message;
  }

  const cursor: Cursor | null = page.nextCursor;
  const next = mergeConversationNames(state, page.conversations);

  return {
    ...next,
    scheduled: {
      items,
      lists: {
        ...next.scheduled.lists,
        [key]: pagedLanded(
          scheduledListOf(next, key),
          page.scheduledMessages.map((message) => message.id),
          cursor,
          mode,
        ),
      },
    },
  };
}

/**
 * A scheduled message as it is now (a reply, `scheduled.changed`, or an optimistic change):
 * stored, and moved into or out of every loaded list.
 */
export function applyScheduled(state: State, message: ScheduledMessage): State {
  const items = { ...state.scheduled.items, [message.id]: message };

  const lists = eachPaged(state.scheduled.lists, (list, key) =>
    pagedPlaced(list, message.id, belongsTo(key, message), scheduledOrder(items, key)),
  );

  return { ...state, scheduled: { items, lists } };
}

/** Cancelled (`scheduled.removed`, or here): out of the store and every list. */
export function removeScheduled(state: State, id: number): State {
  if (state.scheduled.items[id] === undefined) {
    return state;
  }

  const { [id]: _gone, ...items } = state.scheduled.items;

  return {
    ...state,
    scheduled: {
      items,
      lists: eachPaged(state.scheduled.lists, (list) => pagedWithout(list, id)),
    },
  };
}

/** Every loaded list reloads when next shown (missed events, or a send that may have moved). */
export function markScheduledStale(state: State): State {
  const lists = eachPaged(state.scheduled.lists, pagedStale);

  return lists === state.scheduled.lists
    ? state
    : { ...state, scheduled: { ...state.scheduled, lists } };
}
