/**
 * The Saved page in the store: the viewer's saved items by id and one keyset-paged list per
 * status filter (newest saved first). The saved messages live with every other message in
 * `state.messages`; the per-message mark (`state.saved`, the "Saved for later" flag) stays in
 * step with the items here.
 */
import type { SavedFilter } from "../gen/SavedFilter.ts";
import type { SavedItem } from "../gen/SavedItem.ts";
import type { SavedItemList } from "../gen/SavedItemList.ts";
import { mergeConversationNames } from "./conversations.ts";
import { setSavedMark } from "./message-extras.ts";
import type { MessageDTO } from "./model.ts";
import { mergeUserList } from "./ordering.ts";
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

export interface SavedListSlice {
  /** By saved item id. */
  readonly items: Readonly<Record<number, SavedItem>>;
  /** By `SavedFilter`. */
  readonly lists: Readonly<Record<string, PagedList>>;
}

export const emptySavedList: SavedListSlice = { items: {}, lists: {} };

/** The filters, as the Saved page offers them. */
export const SAVED_FILTERS: readonly SavedFilter[] = ["all", "in_progress", "done"];

function belongsTo(filter: string, item: SavedItem): boolean {
  return filter === "all" || filter === item.status;
}

/** Newest saved first, then the higher id. */
function savedOrder(items: SavedListSlice["items"]): IdOrder {
  return (left, right) => {
    const a = items[left];
    const b = items[right];

    if (a === undefined || b === undefined) {
      return 0;
    }

    if (a.createdAt !== b.createdAt) {
      return a.createdAt < b.createdAt ? 1 : -1;
    }

    return right - left;
  };
}

/** One filter's list, or an empty one never loaded. */
export function savedListOf(state: State, filter: SavedFilter): PagedList {
  return state.savedList.lists[filter] ?? emptyPagedList;
}

/** The viewer's saved item for a message, if the store knows it. */
export function savedItemForMessage(state: State, messageId: number): SavedItem | null {
  const id = state.saved[messageId];

  return id === undefined ? null : (state.savedList.items[id] ?? null);
}

function updateList(
  state: State,
  filter: SavedFilter,
  change: (list: PagedList) => PagedList,
): State {
  return {
    ...state,
    savedList: {
      ...state.savedList,
      lists: { ...state.savedList.lists, [filter]: change(savedListOf(state, filter)) },
    },
  };
}

export function setSavedListLoading(state: State, filter: SavedFilter, more: boolean): State {
  return updateList(state, filter, more ? pagedLoadingMore : pagedLoading);
}

export function setSavedListFailed(state: State, filter: SavedFilter, error: string): State {
  return updateList(state, filter, (list) => pagedFailed(list, error));
}

/** Messages join the store unless deleted here or older than the copy held. */
function mergeMessages(state: State, messages: readonly MessageDTO[]): State {
  if (messages.length === 0) {
    return state;
  }

  const merged = { ...state.messages };

  for (const message of messages) {
    const held = merged[message.id];

    if (
      state.tombstones[message.id] === undefined &&
      (held === undefined || held.updatedAt <= message.updatedAt)
    ) {
      merged[message.id] = message;
    }
  }

  return { ...state, messages: merged };
}

/**
 * A page of one filter landed: the items, their messages, the messages' authors and the
 * conversation names join the store, and each message's saved mark follows its item.
 */
export function landSavedPage(
  state: State,
  filter: SavedFilter,
  page: SavedItemList,
  mode: "replace" | "more",
): State {
  const items = { ...state.savedList.items };
  const saved = { ...state.saved };

  for (const item of page.items) {
    items[item.id] = item;
    saved[item.messageId] = item.id;
  }

  // Until the contract's opaque cursors land, the numeric cursor travels as text.
  const cursor: Cursor | null = page.nextCursor === null ? null : String(page.nextCursor);

  const next = mergeConversationNames(mergeMessages(state, page.messages), page.conversations);

  return {
    ...next,
    users: mergeUserList(next.users, page.users),
    saved,
    savedList: {
      items,
      lists: {
        ...next.savedList.lists,
        [filter]: pagedLanded(
          savedListOf(next, filter),
          page.items.map((item) => item.id),
          cursor,
          mode,
        ),
      },
    },
  };
}

/**
 * A message was saved, re-saved, marked done, reopened or reminded (`item`), or unsaved
 * (`null`), here or elsewhere (`saved.changed`, a reply, or an optimistic change): the mark, the
 * item and every loaded list follow. A newly saved message the store doesn't hold marks the
 * lists it belongs in for a reload, since a row needs its message.
 */
export function applySavedChange(state: State, messageId: number, item: SavedItem | null): State {
  const marked = setSavedMark(state, messageId, item?.id ?? null);
  const previousId = state.saved[messageId];

  const { [previousId ?? -1]: _previous, ...rest } = marked.savedList.items;
  const items = item === null ? rest : { ...rest, [item.id]: item };
  const order = savedOrder(items);
  const unknownMessage = item !== null && marked.messages[messageId] === undefined;

  const lists = eachPaged(marked.savedList.lists, (list, filter) => {
    let placed = list;

    if (previousId !== undefined && previousId !== item?.id) {
      placed = pagedWithout(placed, previousId);
    }

    if (item === null) {
      return placed;
    }

    const belongs = belongsTo(filter, item);

    if (belongs && unknownMessage && !list.ids.includes(item.id)) {
      return pagedStale(placed);
    }

    return pagedPlaced(placed, item.id, belongs, order);
  });

  return { ...marked, savedList: { items, lists } };
}

/** A message was deleted: its saved item goes too (the server deletes it with the message). */
export function dropSavedForMessage(state: State, messageId: number): State {
  const id = Object.values(state.savedList.items).find((item) => item.messageId === messageId)?.id;

  if (id === undefined) {
    return state;
  }

  const { [id]: _gone, ...items } = state.savedList.items;

  return {
    ...state,
    savedList: {
      items,
      lists: eachPaged(state.savedList.lists, (list) => pagedWithout(list, id)),
    },
  };
}

/** The server couldn't replay what was missed: every loaded list reloads when next shown. */
export function markSavedStale(state: State): State {
  const lists = eachPaged(state.savedList.lists, pagedStale);

  return lists === state.savedList.lists
    ? state
    : { ...state, savedList: { ...state.savedList, lists } };
}
