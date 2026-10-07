/**
 * The activity inbox in the store: every item seen, one keyset-paged list per tab and state, and
 * the unread badge. Pure reducers and selectors; `store.ts` wraps the reducers in `mutations`.
 *
 * An item that changes state moves between the lists at once (handled leaves Unread and joins
 * Handled, if that list's loaded window reaches its place). The unread count projects changes
 * against a server snapshot until confirmations establish which adjustments the next count covers.
 *
 * Server copies (pages, replies, `activity.item`) replace a held item only when their `updatedAt`
 * is newer than the newest server copy seen, so a late reply or page can't undo a newer event.
 * An optimistic change shows at once and counts until a server snapshot confirms it; it settles to the
 * server's reply, or back to the copy it started from, only while it is still what's shown.
 */

import type { ActivityAction } from "../gen/ActivityAction.ts";
import type { ActivityEventType } from "../gen/ActivityEventType.ts";
import type { ActivityItem } from "../gen/ActivityItem.ts";
import type { ActivityList } from "../gen/ActivityList.ts";
import type { ActivityState } from "../gen/ActivityState.ts";
import type { ActivityTab } from "../gen/ActivityTab.ts";
import type { ActivityUnreadCount } from "../gen/ActivityUnreadCount.ts";
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

interface PendingUnread {
  readonly itemId: number;
  readonly unread: boolean;
  readonly delta: number;
  readonly revision: number | null;
  readonly version: string | undefined;
  readonly removed: boolean;
  /** A snapshot at this revision or later includes the change. */
  readonly coveredAtRevision: number | null;
  /** A successful reply can finish before the shared count covers every adjustment. */
  readonly settled: boolean;
  readonly before: ActivityItem;
  readonly optimistic: ActivityItem;
}

export interface ActivitySlice {
  /** Every item any list or event brought, by id. */
  readonly items: Readonly<Record<number, ActivityItem>>;
  /** By `activityListKey(tab, status)`. */
  readonly lists: Readonly<Record<string, PagedList>>;
  /** The badge: unread items across every type; `null` until first known. */
  readonly unreadCount: number | null;
  /** The `updatedAt` of the newest server copy of each item (optimistic copies don't count). */
  readonly versions: Readonly<Record<number, string>>;
  /** The server count used as the base for adjustments not yet absorbed by a snapshot. */
  readonly serverUnread: ActivityUnreadCount | null;
  /** The newest snapshot waiting for confirmations to resolve its pending overlap. */
  readonly deferredUnread: ActivityUnreadCount | null;
  /** Local fence for replies started before a reconnect, deadline or new server epoch. */
  readonly generation: number;
  /** The generation the held count belongs to; an older one is only a display fallback. */
  readonly serverUnreadGeneration: number;
  /** How each change on its way moves the badge, by its token. */
  readonly pendingUnread: Readonly<Record<number, PendingUnread>>;
}

export const emptyActivity: ActivitySlice = {
  items: {},
  lists: {},
  unreadCount: null,
  versions: {},
  serverUnread: null,
  deferredUnread: null,
  generation: 0,
  serverUnreadGeneration: 0,
  pendingUnread: {},
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
    Object.values(pending).reduce((total, change) => total + change.delta, server),
  );
}

/** Drops adjustments a count already covers, retaining active requests for row settlement. */
function absorbedUnread(
  pending: ActivitySlice["pendingUnread"],
  revision: number | null,
): ActivitySlice["pendingUnread"] {
  return Object.fromEntries(
    Object.entries(pending).flatMap(([token, change]) => {
      const covered =
        change.delta === 0 ||
        (revision !== null &&
          change.coveredAtRevision !== null &&
          change.coveredAtRevision <= revision);

      return !covered
        ? [[token, change]]
        : change.settled
          ? []
          : [[token, { ...change, delta: 0 }]];
    }),
  );
}

/** Installs the newest snapshot only when it covers every outstanding badge adjustment. */
function withCounts(
  activity: ActivitySlice,
  unread: ActivityUnreadCount | null,
  pendingUnread: ActivitySlice["pendingUnread"] = activity.pendingUnread,
): ActivitySlice {
  const held = activity.serverUnread;
  let snapshot = activity.serverUnreadGeneration === activity.generation ? held : null;
  const projected = absorbedUnread(pendingUnread, snapshot?.unreadRevision ?? null);

  // Equal revisions keep their first count, including a candidate still waiting for coverage.
  for (const candidate of [activity.deferredUnread, unread]) {
    if (
      candidate !== null &&
      (snapshot === null || candidate.unreadRevision > snapshot.unreadRevision)
    ) {
      snapshot = { unreadCount: candidate.unreadCount, unreadRevision: candidate.unreadRevision };
    }
  }

  const waiting = Object.values(projected).some(
    (change) =>
      change.delta !== 0 &&
      (change.coveredAtRevision === null ||
        snapshot === null ||
        snapshot.unreadRevision < change.coveredAtRevision),
  );

  const serverUnread = waiting ? held : (snapshot ?? held);

  const serverUnreadGeneration =
    !waiting && snapshot !== null ? activity.generation : activity.serverUnreadGeneration;

  const remaining = waiting
    ? projected
    : absorbedUnread(projected, snapshot?.unreadRevision ?? null);

  return {
    ...activity,
    serverUnread,
    serverUnreadGeneration,
    deferredUnread: waiting && snapshot !== held ? snapshot : null,
    pendingUnread: remaining,
    unreadCount: shownCount(serverUnread?.unreadCount ?? null, remaining),
  };
}

/** An item-bearing snapshot confirms only the changes its count already includes. */
function confirmedUnread(
  activity: ActivitySlice,
  unread: ActivityUnreadCount | null,
  items: readonly ActivityItem[],
  removedId?: number,
): ActivitySlice["pendingUnread"] {
  const held = activity.serverUnread;

  if (
    unread === null ||
    (held !== null &&
      activity.serverUnreadGeneration === activity.generation &&
      (unread.unreadRevision < held.unreadRevision ||
        (unread.unreadRevision === held.unreadRevision && unread.unreadCount !== held.unreadCount)))
  ) {
    return activity.pendingUnread;
  }

  return Object.fromEntries(
    Object.entries(activity.pendingUnread).map(([token, change]) => {
      const covered = change.revision === null || unread.unreadRevision >= change.revision;
      const removed = covered && change.itemId === removedId;

      const confirmed =
        covered &&
        items.some((item) => {
          const version = activity.versions[item.id];

          return (
            item.id === change.itemId &&
            (item.state === "unread") === change.unread &&
            (change.version === undefined || !newerThan(change.version, item.updatedAt)) &&
            (version === undefined || !newerThan(version, item.updatedAt))
          );
        });

      return [
        token,
        removed || confirmed
          ? {
              ...change,
              coveredAtRevision: Math.min(
                change.coveredAtRevision ?? Infinity,
                unread.unreadRevision,
              ),
              removed: removed || change.removed,
            }
          : change,
      ];
    }),
  );
}

/** Fences old requests and abandons optimism; only a new boot resets server ordering. */
export function beginActivityGeneration(state: State, newEpoch = true): State {
  let activity = state.activity;

  for (const change of Object.values(activity.pendingUnread)) {
    if (change.coveredAtRevision === null && activity.items[change.itemId] === change.optimistic) {
      activity = showItem(activity, change.before);
    }
  }

  // On the same server, the held candidate becomes authoritative once optimism is abandoned.
  if (!newEpoch) {
    activity = withCounts(activity, null, {});
  }

  return withActivity(state, {
    ...activity,
    generation: activity.generation + 1,
    serverUnreadGeneration:
      !newEpoch && activity.serverUnreadGeneration === activity.generation
        ? activity.generation + 1
        : activity.serverUnreadGeneration,
    deferredUnread: null,
    versions: newEpoch ? {} : activity.versions,
    pendingUnread: {},
    unreadCount: activity.serverUnread?.unreadCount ?? null,
    lists: eachPaged(activity.lists, (list) => ({
      ...pagedStale(list),
      status: list.status === "ready" ? "ready" : "idle",
      loadingMore: false,
      generation: list.generation + 1,
    })),
  });
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
  activityGeneration = state.activity.generation,
): State {
  if (activityGeneration !== state.activity.generation) {
    return state;
  }

  return updateList(state, tab, status, (list) => pagedFailed(list, error, generation));
}

/** The list generation a page belongs to. */
export interface ActivityLoadStart {
  readonly generation: number;
  readonly activityGeneration: number;
}

/** What a load of `tab` in `status` started from; pass it back to `landActivityPage`. */
export function activityLoadStart(
  state: State,
  tab: ActivityTab,
  status: ActivityState,
): ActivityLoadStart {
  return {
    generation: activityListOf(state, tab, status).generation,
    activityGeneration: state.activity.generation,
  };
}

/**
 * A page of one tab and state landed: its items join the store where newer than the copy held,
 * its people too, and the badge takes the page's count only if its revision is newer. A
 * page from a load the list has since restarted contributes only its count snapshot.
 */
export function landActivityPage(
  state: State,
  tab: ActivityTab,
  status: ActivityState,
  page: ActivityList,
  mode: "replace" | "more",
  start?: ActivityLoadStart,
): State {
  if (start !== undefined && start.activityGeneration !== state.activity.generation) {
    return state;
  }

  const counted = withCounts(
    state.activity,
    page,
    confirmedUnread(state.activity, page, page.items),
  );

  if (!pagedCurrent(activityListOf(state, tab, status), start?.generation)) {
    return withActivity(state, counted);
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
 * or newer copy is already held. `unread` is the server's snapshot afterwards (`null` keeps
 * the badge); it lands even when the item doesn't.
 */
export function applyActivityItem(
  state: State,
  item: ActivityItem,
  unread: ActivityUnreadCount | null,
): State {
  const next = withCounts(
    serverCopy(state.activity, item),
    unread,
    confirmedUnread(state.activity, unread, [item]),
  );

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
    withCounts(shown, null, {
      ...state.activity.pendingUnread,
      [token]: {
        itemId: item.id,
        unread: item.state === "unread",
        delta: unreadDelta,
        revision:
          state.activity.serverUnreadGeneration === state.activity.generation
            ? (state.activity.serverUnread?.unreadRevision ?? null)
            : null,
        removed: false,
        version: state.activity.versions[item.id],
        coveredAtRevision: null,
        settled: false,
        before: state.activity.items[item.id] ?? item,
        optimistic: item,
      },
    }),
  );
}

/** How a change on its way ended. */
export interface ActivityChangeEnd {
  /** The generation the request started in. */
  readonly generation: number;
  /** The change's token. Its adjustment stays until a snapshot covers it. */
  readonly token: number;
  /** The copy it showed, if it showed one. */
  readonly optimistic: ActivityItem | null;
  /** The server's reply, or the copy it started from when the server refused. */
  readonly settled: ActivityItem | null;
  /** The server's count snapshot afterwards; `null` keeps it. */
  readonly unread: ActivityUnreadCount | null;
}

/**
 * A request ended: its badge adjustment stays until all adjustments have snapshot coverage.
 * `settled` replaces the optimistic copy while that's still shown. If an event replaced it
 * meanwhile, `settled` counts only as a server copy, so an older one changes nothing.
 */
export function endActivityChange(state: State, end: ActivityChangeEnd): State {
  if (end.generation !== state.activity.generation) {
    return state;
  }

  const change = state.activity.pendingUnread[end.token];
  const removed = change?.removed ?? false;
  const { [end.token]: _done, ...others } = state.activity.pendingUnread;
  let pendingUnread = others;

  if (change !== undefined && end.unread !== null && change.delta !== 0) {
    pendingUnread = {
      ...others,
      [end.token]: {
        ...change,
        settled: true,
        coveredAtRevision: Math.min(
          change.coveredAtRevision ?? Infinity,
          end.unread.unreadRevision,
        ),
      },
    };
  }

  let activity: ActivitySlice = state.activity;

  if (end.settled !== null && !removed) {
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

  return withActivity(state, withCounts(activity, end.unread, pendingUnread));
}

/** An item went with its source (`activity.removed`): out of the store and every list. */
export function removeActivityItem(state: State, id: number, unread: ActivityUnreadCount): State {
  const activity = state.activity;
  const pendingUnread = confirmedUnread(activity, unread, [], id);

  if (activity.items[id] === undefined) {
    return withActivity(state, withCounts(activity, unread, pendingUnread));
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
      unread,
      pendingUnread,
    ),
  );
}

/** The badge, from `GET /activity/unread_count`. */
export function setActivityUnreadCount(
  state: State,
  unread: ActivityUnreadCount,
  generation = state.activity.generation,
): State {
  const activity = state.activity;

  if (generation !== activity.generation) {
    return state;
  }

  return withActivity(state, withCounts(activity, unread));
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
