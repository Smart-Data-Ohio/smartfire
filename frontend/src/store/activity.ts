/**
 * The activity inbox in the store: every item seen, one keyset-paged list per tab and state, and
 * the unread badge. Pure reducers and selectors; `store.ts` wraps the reducers in `mutations`.
 *
 * An item that changes state moves between the lists at once (handled leaves Unread and joins
 * Handled, if that list's loaded window reaches its place), and the unread count is always the
 * server's latest word: every reply and event carries it.
 */

import type { ActivityAction } from "../gen/ActivityAction.ts";
import type { ActivityEventType } from "../gen/ActivityEventType.ts";
import type { ActivityItem } from "../gen/ActivityItem.ts";
import type { ActivityList } from "../gen/ActivityList.ts";
import type { ActivityState } from "../gen/ActivityState.ts";
import type { ActivityTab } from "../gen/ActivityTab.ts";
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

/** What `PATCH /activity/:id` does: "unhandled" clears handled and keeps it read. */
export type { ActivityAction };

export interface ActivitySlice {
  /** Every item any list or event brought, by id. */
  readonly items: Readonly<Record<number, ActivityItem>>;
  /** By `activityListKey(tab, status)`. */
  readonly lists: Readonly<Record<string, PagedList>>;
  /** The badge: unread items across every type; `null` until first known. */
  readonly unreadCount: number | null;
}

export const emptyActivity: ActivitySlice = { items: {}, lists: {}, unreadCount: null };

/** The tabs in the inbox's order. */
export const ACTIVITY_TABS: readonly ActivityTab[] = [
  "all",
  "mentions",
  "threads",
  "events",
  "agents",
  "github",
  "huddles",
  "reminders",
  "security",
];

/** The states, as the inbox's status filter offers them. */
export const ACTIVITY_STATUSES: readonly ActivityState[] = ["unread", "read", "handled"];

/**
 * The event types each tab holds (`ActivityItem::query_accessible`). `all` holds every type, and
 * is the only tab with `scheduled_message_dropped`.
 */
export const ACTIVITY_TAB_TYPES: Readonly<
  Record<Exclude<ActivityTab, "all">, readonly ActivityEventType[]>
> = {
  mentions: ["mention", "reply", "keyword_alert"],
  threads: ["thread_activity", "work_update", "work_assignment", "work_sla"],
  events: ["event_invitation", "event_update", "event_cancelled", "event_reminder"],
  agents: ["agent_approval_request", "agent_budget_exceeded"],
  github: ["pr_review_request"],
  huddles: ["huddle_started", "huddle_missed"],
  reminders: ["message_reminder"],
  security: ["new_sign_in", "two_factor_lockout"],
};

/** The key of one tab's list in one state. */
export function activityListKey(tab: ActivityTab, status: ActivityState): string {
  return `${tab}:${status}`;
}

/** Whether `tab` shows items of `type`. */
export function inActivityTab(tab: ActivityTab, type: ActivityEventType): boolean {
  return tab === "all" || ACTIVITY_TAB_TYPES[tab].includes(type);
}

/** Which list a key names: one tab in one state. */
interface ActivityListName {
  readonly tab: ActivityTab;
  readonly status: ActivityState;
}

/** The tab and state a list key names; `null` for a key this module didn't make. */
function parseListKey(key: string): ActivityListName | null {
  const [rawTab, rawStatus] = key.split(":");
  const tab = ACTIVITY_TABS.find((candidate) => candidate === rawTab);
  const status = ACTIVITY_STATUSES.find((candidate) => candidate === rawStatus);

  return tab === undefined || status === undefined ? null : { tab, status };
}

/** The inbox's order: newest `updatedAt` first, then the higher id. */
function activityOrder(items: ActivitySlice["items"]): IdOrder {
  return (left, right) => {
    const a = items[left];
    const b = items[right];

    if (a === undefined || b === undefined) {
      return 0;
    }

    if (a.updatedAt !== b.updatedAt) {
      return a.updatedAt < b.updatedAt ? 1 : -1;
    }

    return right - left;
  };
}

/** One tab's list in one state, or an empty one never loaded. */
export function activityListOf(state: State, tab: ActivityTab, status: ActivityState): PagedList {
  return state.activity.lists[activityListKey(tab, status)] ?? emptyPagedList;
}

function withActivity(state: State, activity: ActivitySlice): State {
  return activity === state.activity ? state : { ...state, activity };
}

function updateList(
  state: State,
  tab: ActivityTab,
  status: ActivityState,
  change: (list: PagedList) => PagedList,
): State {
  const key = activityListKey(tab, status);

  return withActivity(state, {
    ...state.activity,
    lists: { ...state.activity.lists, [key]: change(activityListOf(state, tab, status)) },
  });
}

/** A first page (`more: false`) or a next page is on its way. */
export function setActivityListLoading(
  state: State,
  tab: ActivityTab,
  status: ActivityState,
  more: boolean,
): State {
  return updateList(state, tab, status, more ? pagedLoadingMore : pagedLoading);
}

export function setActivityListFailed(
  state: State,
  tab: ActivityTab,
  status: ActivityState,
  error: string,
): State {
  return updateList(state, tab, status, (list) => pagedFailed(list, error));
}

/**
 * A page of one tab and state landed: its items join the store (the page's copy is the server's
 * latest), its people too, and the badge takes the page's count.
 */
export function landActivityPage(
  state: State,
  tab: ActivityTab,
  status: ActivityState,
  page: ActivityList,
  mode: "replace" | "more",
): State {
  const items = { ...state.activity.items };

  for (const item of page.items) {
    items[item.id] = item;
  }

  const cursor: Cursor | null = page.nextCursor;
  const key = activityListKey(tab, status);

  return {
    ...state,
    users: mergeUserList(state.users, page.users),
    activity: {
      items,
      unreadCount: page.unreadCount,
      lists: {
        ...state.activity.lists,
        [key]: pagedLanded(
          activityListOf(state, tab, status),
          page.items.map((item) => item.id),
          cursor,
          mode,
        ),
      },
    },
  };
}

/**
 * An item as it is now (a reply, an `activity.item` event or an optimistic change): stored, and
 * moved into or out of every loaded list. `unreadCount` is the server's count afterwards; `null`
 * keeps the badge (an optimistic change adjusts it itself).
 */
export function applyActivityItem(
  state: State,
  item: ActivityItem,
  unreadCount: number | null,
): State {
  const items = { ...state.activity.items, [item.id]: item };
  const order = activityOrder(items);

  const lists = eachPaged(state.activity.lists, (list, key) => {
    const parsed = parseListKey(key);

    if (parsed === null) {
      return list;
    }

    const belongs = item.state === parsed.status && inActivityTab(parsed.tab, item.eventType);

    return pagedPlaced(list, item.id, belongs, order);
  });

  return withActivity(state, {
    items,
    lists,
    unreadCount: unreadCount ?? state.activity.unreadCount,
  });
}

/** An item went with its source (`activity.removed`): out of the store and every list. */
export function removeActivityItem(state: State, id: number, unreadCount: number): State {
  const { [id]: _gone, ...items } = state.activity.items;

  return withActivity(state, {
    items,
    lists: eachPaged(state.activity.lists, (list) => pagedWithout(list, id)),
    unreadCount,
  });
}

/** The badge, from `GET /activity/unread_count`. */
export function setActivityUnreadCount(state: State, unreadCount: number): State {
  return state.activity.unreadCount === unreadCount
    ? state
    : withActivity(state, { ...state.activity, unreadCount });
}

/** The server couldn't replay what was missed: every loaded list reloads when next shown. */
export function markActivityStale(state: State): State {
  const lists = eachPaged(state.activity.lists, pagedStale);

  return lists === state.activity.lists ? state : withActivity(state, { ...state.activity, lists });
}

/** The state the timestamps make: handled, else read, else unread. */
function derivedState(readAt: string | null, handledAt: string | null): ActivityState {
  if (handledAt !== null) {
    return "handled";
  }

  return readAt === null ? "unread" : "read";
}

/**
 * The item after `action`, as the server applies it (`PATCH /activity/:id`, `open` is `read`):
 * read sets `readAt` (a no-op on a handled item); unread clears both; handled sets `handledAt`
 * and `readAt` if unset; unhandled clears `handledAt`. A change bumps `updatedAt`, the inbox's
 * sort key (`save_state`), so the item lands at the top of its new list.
 */
export function nextActivityItem(
  item: ActivityItem,
  action: ActivityAction,
  now: string,
): ActivityItem {
  let { readAt, handledAt } = item;

  switch (action) {
    case "read":
      readAt ??= now;
      break;
    case "unread":
      readAt = null;
      handledAt = null;
      break;
    case "handled":
      readAt ??= now;
      handledAt ??= now;
      break;
    case "unhandled":
      handledAt = null;
      break;
  }

  if (readAt === item.readAt && handledAt === item.handledAt) {
    return item;
  }

  return {
    ...item,
    readAt,
    handledAt,
    updatedAt: now,
    state: derivedState(readAt, handledAt),
  };
}

/** How the badge moves when an item goes from `before` to `after`. */
export function unreadDelta(before: ActivityItem, after: ActivityItem): number {
  const was = before.state === "unread" ? 1 : 0;
  const is = after.state === "unread" ? 1 : 0;

  return is - was;
}

/**
 * Where opening an item leads: a message (at it, in its thread when it has one), a thread, a
 * room, the Saved or Scheduled pages, or a classic page the SPA has no screen for yet.
 */
export type ActivityDestination =
  | {
      readonly kind: "message";
      readonly roomId: number;
      readonly messageId: number;
      readonly threadId: number | null;
    }
  | { readonly kind: "thread"; readonly roomId: number; readonly threadId: number }
  | { readonly kind: "room"; readonly roomId: number }
  | { readonly kind: "scheduled" }
  | { readonly kind: "classic"; readonly path: string };

/** Where opening `item` should go, from its source's ids (see `ActivityDestination`). */
export function activityDestination(item: ActivityItem): ActivityDestination {
  const source = item.source;
  const { roomId, threadId, messageId } = source;

  switch (source.sourceType) {
    case "message":
    case "saved_item":
      if (roomId !== null && messageId !== null) {
        return { kind: "message", roomId, messageId, threadId };
      }

      break;
    case "work_thread_event":
    case "board_sla_nudge":
      if (roomId !== null && threadId !== null) {
        return { kind: "thread", roomId, threadId };
      }

      break;
    case "huddle_grant":
      if (roomId !== null) {
        return { kind: "room", roomId };
      }

      break;
    case "scheduled_message":
      return { kind: "scheduled" };
    default:
      break;
  }

  return { kind: "classic", path: source.path };
}
