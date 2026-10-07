/**
 * The activity inbox in the store: every item seen, one keyset-paged list per tab and state, and
 * the unread badge. Pure reducers and selectors; `store.ts` wraps the reducers in `mutations`.
 *
 * An item that changes state moves between the lists at once (handled leaves Unread and joins
 * Handled, if that list's loaded window reaches its place), and the unread count is the
 * server's latest word (every reply and event carries it) plus the changes still on their way.
 *
 * Server copies (pages, replies, `activity.item`) replace a held item only when their `updatedAt`
 * is newer than the newest server copy seen, so a late reply or page can't undo a newer event.
 * An optimistic change shows at once and counts in the badge until it settles; it settles to the
 * server's reply, or back to the copy it started from, only while it is still what's shown.
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
  pagedCurrent,
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
  /** The `updatedAt` of the newest server copy of each item (optimistic copies don't count). */
  readonly versions: Readonly<Record<number, string>>;
  /** The server's latest unread count, before the changes still on their way. */
  readonly serverUnreadCount: number | null;
  /** How each change on its way moves the badge, by its token. */
  readonly pendingUnread: Readonly<Record<number, number>>;
  /** Bumped whenever a server count lands; REST replies count only if none landed since they left. */
  readonly countEpoch: number;
}

export const emptyActivity: ActivitySlice = {
  items: {},
  lists: {},
  unreadCount: null,
  versions: {},
  serverUnreadCount: null,
  pendingUnread: {},
  countEpoch: 0,
};

/** Whether `incoming` is a later server copy than the `held` version. */
function newerThan(incoming: string, held: string | undefined): boolean {
  if (held === undefined) {
    return true;
  }

  const a = Date.parse(incoming);
  const b = Date.parse(held);

  return Number.isNaN(a) || Number.isNaN(b) ? incoming > held : a > b;
}

/** The badge: the server's count moved by every change on its way. */
function shownCount(server: number | null, pending: ActivitySlice["pendingUnread"]): number | null {
  if (server === null) {
    return null;
  }

  return Math.max(
    0,
    Object.values(pending).reduce((total, delta) => total + delta, server),
  );
}

/** `activity` with the server's count `unreadCount` (`null` keeps it) and its badge worked out. */
function withCounts(
  activity: ActivitySlice,
  unreadCount: number | null,
  pendingUnread: ActivitySlice["pendingUnread"] = activity.pendingUnread,
): ActivitySlice {
  const server = unreadCount ?? activity.serverUnreadCount;

  return {
    ...activity,
    serverUnreadCount: server,
    pendingUnread,
    unreadCount: shownCount(server, pendingUnread),
    countEpoch: unreadCount === null ? activity.countEpoch : activity.countEpoch + 1,
  };
}

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
  generation?: number,
): State {
  return updateList(state, tab, status, (list) => pagedFailed(list, error, generation));
}

/** Where a load started: the list's `generation` and the badge's `countEpoch` then. */
export interface ActivityLoadStart {
  readonly generation: number;
  readonly countEpoch: number;
}

/** What a load of `tab` in `status` started from; pass it back to `landActivityPage`. */
export function activityLoadStart(
  state: State,
  tab: ActivityTab,
  status: ActivityState,
): ActivityLoadStart {
  return {
    generation: activityListOf(state, tab, status).generation,
    countEpoch: state.activity.countEpoch,
  };
}

/**
 * A page of one tab and state landed: its items join the store where newer than the copy held,
 * its people too, and the badge takes the page's count unless a newer one landed meanwhile. A
 * page from a load the list has since restarted changes nothing (`start`, when given).
 */
export function landActivityPage(
  state: State,
  tab: ActivityTab,
  status: ActivityState,
  page: ActivityList,
  mode: "replace" | "more",
  start?: ActivityLoadStart,
): State {
  if (!pagedCurrent(activityListOf(state, tab, status), start?.generation)) {
    return state;
  }

  const items = { ...state.activity.items };
  const versions = { ...state.activity.versions };

  for (const item of page.items) {
    if (newerThan(item.updatedAt, versions[item.id])) {
      items[item.id] = item;
      versions[item.id] = item.updatedAt;
    }
  }

  const cursor: Cursor | null = page.nextCursor;
  const key = activityListKey(tab, status);
  const countHolds = start === undefined || start.countEpoch === state.activity.countEpoch;
  const counted = withCounts(state.activity, countHolds ? page.unreadCount : null);

  return {
    ...state,
    users: mergeUserList(state.users, page.users),
    activity: {
      ...counted,
      items,
      versions,
      lists: {
        ...state.activity.lists,
        [key]: pagedLanded(
          activityListOf(state, tab, status),
          // A row whose newer copy (an event's) has left this list meanwhile stays out.
          page.items
            .filter((item) => {
              const held = items[item.id] ?? item;

              return held.state === status && inActivityTab(tab, held.eventType);
            })
            .map((item) => item.id),
          cursor,
          mode,
        ),
      },
    },
  };
}

/** `item` shown: stored, and moved into or out of every loaded list. */
function showItem(activity: ActivitySlice, item: ActivityItem): ActivitySlice {
  const items = { ...activity.items, [item.id]: item };
  const order = activityOrder(items);

  const lists = eachPaged(activity.lists, (list, key) => {
    const parsed = parseListKey(key);

    if (parsed === null) {
      return list;
    }

    const belongs = item.state === parsed.status && inActivityTab(parsed.tab, item.eventType);

    return pagedPlaced(list, item.id, belongs, order);
  });

  return { ...activity, items, lists };
}

/** A server copy: shown when newer than the newest server copy seen, else only recorded. */
function serverCopy(activity: ActivitySlice, item: ActivityItem): ActivitySlice {
  if (!newerThan(item.updatedAt, activity.versions[item.id])) {
    return activity;
  }

  return {
    ...showItem(activity, item),
    versions: { ...activity.versions, [item.id]: item.updatedAt },
  };
}

/**
 * An item as the server has it now (a reply or an `activity.item` event): shown unless an equal
 * or newer copy is already held. `unreadCount` is the server's count afterwards (`null` keeps
 * the badge); it lands even when the item doesn't.
 */
export function applyActivityItem(
  state: State,
  item: ActivityItem,
  unreadCount: number | null,
): State {
  const next = withCounts(serverCopy(state.activity, item), unreadCount);

  return withActivity(state, next);
}

/** A change on its way, shown at once: `item` as it will be, moving the badge by `unreadDelta`. */
export function showActivityChange(
  state: State,
  item: ActivityItem,
  token: number,
  unreadDelta: number,
): State {
  const shown = showItem(state.activity, item);

  return withActivity(
    state,
    withCounts(shown, null, { ...state.activity.pendingUnread, [token]: unreadDelta }),
  );
}

/** How a change on its way ended. */
export interface ActivityChangeEnd {
  /** The change's token (its badge move goes). */
  readonly token: number;
  /** The copy it showed, if it showed one. */
  readonly optimistic: ActivityItem | null;
  /** The server's reply, or the copy it started from when the server refused. */
  readonly settled: ActivityItem | null;
  /** The server's count afterwards; `null` keeps it. */
  readonly unreadCount: number | null;
  /** The badge epoch before the request; a newer server count keeps precedence. */
  readonly countEpoch: number;
}

/**
 * A change on its way ended: its badge move goes, and `settled` replaces the optimistic copy
 * while that's still shown. If something newer replaced it meanwhile (an event), `settled`
 * counts only as a server copy, so an older one changes nothing.
 */
export function endActivityChange(state: State, end: ActivityChangeEnd): State {
  const { [end.token]: _done, ...pendingUnread } = state.activity.pendingUnread;
  let activity: ActivitySlice = state.activity;

  if (end.settled !== null) {
    const id = end.settled.id;
    const stillShown = end.optimistic !== null && activity.items[id] === end.optimistic;

    if (stillShown) {
      const held = activity.versions[id];

      const version =
        held === undefined || newerThan(end.settled.updatedAt, held) ? end.settled.updatedAt : held;

      activity = {
        ...showItem(activity, end.settled),
        versions: { ...activity.versions, [id]: version },
      };
    } else {
      activity = serverCopy(activity, end.settled);
    }
  }

  const countHolds = end.countEpoch === activity.countEpoch;

  return withActivity(
    state,
    withCounts(activity, countHolds ? end.unreadCount : null, pendingUnread),
  );
}

/** An item went with its source (`activity.removed`): out of the store and every list. */
export function removeActivityItem(state: State, id: number, unreadCount: number): State {
  const activity = state.activity;

  if (activity.items[id] === undefined) {
    return withActivity(state, withCounts(activity, unreadCount));
  }

  const { [id]: _gone, ...items } = activity.items;
  const { [id]: _version, ...versions } = activity.versions;

  return withActivity(
    state,
    withCounts(
      {
        ...activity,
        items,
        versions,
        lists: eachPaged(activity.lists, (list) => pagedWithout(list, id)),
      },
      unreadCount,
    ),
  );
}

/** The badge, from `GET /activity/unread_count`. */
export function setActivityUnreadCount(
  state: State,
  unreadCount: number,
  countEpoch: number,
): State {
  return state.activity.countEpoch !== countEpoch
    ? state
    : withActivity(state, withCounts(state.activity, unreadCount));
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
