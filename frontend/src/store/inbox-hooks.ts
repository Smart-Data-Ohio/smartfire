/**
 * The hooks the S3 screens read the activity inbox, saved items and scheduled messages with.
 * Each list hook loads its first page when it's first shown (and again when the list went stale:
 * the sync engine missed events it couldn't replay) and keeps up with live events after that,
 * with no refetch. Reads come from the store; loads and writes go through `actions`.
 */
import { useEffect, useMemo } from "react";
import type { ActivityItem } from "../gen/ActivityItem.ts";
import type { ActivityState } from "../gen/ActivityState.ts";
import type { ActivityTab } from "../gen/ActivityTab.ts";
import type { ConversationName } from "../gen/ConversationName.ts";
import type { SavedFilter } from "../gen/SavedFilter.ts";
import type { SavedItem } from "../gen/SavedItem.ts";
import type { ScheduledMessage } from "../gen/ScheduledMessage.ts";
import { actions } from "../sync/runtime.ts";
import { activityListOf } from "./activity.ts";
import { conversationNameOf } from "./conversations.ts";
import type { LoadStatus, MessageDTO, User } from "./model.ts";
import type { PagedList } from "./paged-list.ts";
import { savedListOf } from "./saved-list.ts";
import { type ScheduledListKey, scheduledListOf } from "./scheduled.ts";
import { useStore } from "./store.ts";

/** What every list hook answers besides its rows. */
export interface PagedView {
  /** The first page: `idle`/`loading` (show a skeleton), `ready`, or `error` (nothing shown). */
  readonly status: LoadStatus;
  /** A next page is on its way (show a row spinner at the end). */
  readonly loadingMore: boolean;
  /** More pages lie beyond the loaded rows: call `loadMore` near the end. */
  readonly hasMore: boolean;
  /** The last load's failure (first page or more), fit to show; `null` after a success. */
  readonly error: string | null;
  readonly loadMore: () => void;
  /** Loads the first page again (a Retry, or pull to refresh). */
  readonly reload: () => void;
}

/** Loads a list's first page when shown while it's never loaded or stale. */
function useFirstPage(list: PagedList, load: () => void): void {
  const due = list.status === "idle" || (list.stale && list.status === "ready");

  useEffect(() => {
    if (due) {
      load();
    }
  }, [due, load]);
}

function pagedView(list: PagedList, loadMore: () => void, reload: () => void): PagedView {
  return {
    status: list.status,
    loadingMore: list.loadingMore,
    hasMore: list.nextCursor !== null,
    error: list.error,
    loadMore,
    reload,
  };
}

// --- activity ---

export interface ActivityListView extends PagedView {
  /** Newest `updatedAt` first. Authors: `useStore((state) => state.users[id])`. */
  readonly items: readonly ActivityItem[];
}

/** One inbox tab in one state (`status`), loaded on first show and kept live. */
export function useActivityList(tab: ActivityTab, status: ActivityState): ActivityListView {
  const list = useStore((state) => activityListOf(state, tab, status));
  const byId = useStore((state) => state.activity.items);

  const callbacks = useMemo(
    () => ({
      load: () => void actions.activity.load(tab, status),
      more: () => void actions.activity.loadMore(tab, status),
    }),
    [tab, status],
  );

  useFirstPage(list, callbacks.load);

  return useMemo(
    () => ({
      ...pagedView(list, callbacks.more, callbacks.load),
      items: list.ids.flatMap((id) => byId[id] ?? []),
    }),
    [list, byId, callbacks],
  );
}

let unreadLoad: Promise<number> | null = null;

/** The badge: unread items across every type; `null` until known (loaded on first use). */
export function useActivityUnread(): number | null {
  const count = useStore((state) => state.activity.unreadCount);

  useEffect(() => {
    if (count === null && unreadLoad === null) {
      unreadLoad = actions.activity.loadUnreadCount();
      unreadLoad
        .catch(() => null)
        .finally(() => {
          unreadLoad = null;
        });
    }
  }, [count]);

  return count;
}

// --- saved ---

/** One row of the Saved page: the item with its message, author and conversation. */
export interface SavedRow {
  readonly item: SavedItem;
  /** `null` only while a row that arrived live waits for its list's reload. */
  readonly message: MessageDTO | null;
  readonly author: User | null;
  readonly conversation: ConversationName | null;
}

export interface SavedListView extends PagedView {
  /** Newest saved first. */
  readonly rows: readonly SavedRow[];
}

/** The Saved page for one filter, loaded on first show and kept live. */
export function useSavedList(filter: SavedFilter): SavedListView {
  const list = useStore((state) => savedListOf(state, filter));
  const items = useStore((state) => state.savedList.items);
  const messages = useStore((state) => state.messages);
  const users = useStore((state) => state.users);
  const conversationNames = useStore((state) => state.conversationNames);
  const sidebar = useStore((state) => state.sidebar);
  const threads = useStore((state) => state.threads);

  const callbacks = useMemo(
    () => ({
      load: () => void actions.saved.load(filter),
      more: () => void actions.saved.loadMore(filter),
    }),
    [filter],
  );

  useFirstPage(list, callbacks.load);

  return useMemo(() => {
    const names = { conversationNames, sidebar, threads };

    const rows = list.ids.flatMap((id): SavedRow[] => {
      const item = items[id];

      if (item === undefined) {
        return [];
      }

      const message = messages[item.messageId] ?? null;

      return [
        {
          item,
          message,
          author: message === null ? null : (users[message.creatorId] ?? null),
          conversation:
            message === null ? null : conversationNameOf(names, message.roomId, message.threadId),
        },
      ];
    });

    return { ...pagedView(list, callbacks.more, callbacks.load), rows };
  }, [list, items, messages, users, conversationNames, sidebar, threads, callbacks]);
}

// --- scheduled ---

/** One scheduled message with the conversation it goes to. */
export interface ScheduledRow {
  readonly message: ScheduledMessage;
  readonly conversation: ConversationName | null;
}

export interface ScheduledListView extends PagedView {
  /** Pending lists soonest first; Past most recent first. */
  readonly rows: readonly ScheduledRow[];
}

/**
 * A scheduled list: `"pending"` (upcoming, and stranded ones with `sendable: false`), `"past"`
 * (sent or dropped), or one room's pending ones (`roomScheduledKey(roomId)`). `enabled: false`
 * reads without loading.
 */
export function useScheduledList(key: ScheduledListKey, enabled = true): ScheduledListView {
  const list = useStore((state) => scheduledListOf(state, key));
  const items = useStore((state) => state.scheduled.items);
  const conversationNames = useStore((state) => state.conversationNames);
  const sidebar = useStore((state) => state.sidebar);
  const threads = useStore((state) => state.threads);

  const callbacks = useMemo(
    () => ({
      load: () => void actions.scheduled.load(key),
      more: () => void actions.scheduled.loadMore(key),
      none: () => undefined,
    }),
    [key],
  );

  useFirstPage(list, enabled ? callbacks.load : callbacks.none);

  return useMemo(() => {
    const names = { conversationNames, sidebar, threads };

    const rows = list.ids.flatMap((id): ScheduledRow[] => {
      const message = items[id];

      return message === undefined
        ? []
        : [{ message, conversation: conversationNameOf(names, message.roomId, message.threadId) }];
    });

    return { ...pagedView(list, callbacks.more, callbacks.load), rows };
  }, [list, items, conversationNames, sidebar, threads, callbacks]);
}
