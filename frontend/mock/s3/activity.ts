/**
 * The activity inbox: the list (by state and type tab, keyset-paged newest first), the unread
 * badge, state changes and "open". Other modules record items through `record` (a reminder went
 * out, a scheduled message was dropped, the ambient loop) and drop them with `removeWhere`; every
 * change is published on the viewer's `user` topic with the unread count.
 */
import type { ActivityAction } from "../../src/gen/ActivityAction.ts";
import type { ActivityEventType } from "../../src/gen/ActivityEventType.ts";
import type { ActivityItem } from "../../src/gen/ActivityItem.ts";
import type { ActivityItemChanged } from "../../src/gen/ActivityItemChanged.ts";
import type { ActivityList } from "../../src/gen/ActivityList.ts";
import type { ActivitySource } from "../../src/gen/ActivitySource.ts";
import type { ActivityState } from "../../src/gen/ActivityState.ts";
import type { ActivityTab } from "../../src/gen/ActivityTab.ts";
import { notFound, ok, validation } from "../http.ts";
import { type Json, stringField } from "../json.ts";
import type { ScheduledHooks } from "../s2/composer.ts";
import { firstId, type Route, route, type S2Context } from "../s2/context.ts";
import { iso } from "../s2/model.ts";
import type { Namer } from "./conversations.ts";
import { activityStateOf, beforeOf, keysetPage, oneOf, withActivityState } from "./model.ts";
import { droppedSource } from "./sources.ts";

/** At most this many items a page. */
export const ACTIVITY_PAGE_SIZE = 100;

const STATES: readonly ActivityState[] = ["unread", "read", "handled"];

const TABS: readonly ActivityTab[] = [
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

/** `ActivityItem::query_accessible`: the types each tab holds ("all" holds every type). */
export const TAB_TYPES: Readonly<
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

const ACTIONS: readonly ActivityAction[] = ["read", "unread", "handled", "unhandled"];

/** An item to record: what happened, what it's about, and when. */
export interface ActivityDraft {
  readonly eventType: ActivityEventType;
  readonly source: ActivitySource;
  /** When it was recorded (ms); now by default. */
  readonly at?: number;
}

/** The activity module. */
export interface Activity {
  readonly routes: readonly Route[];
  /**
   * Records an item and publishes it. An item for the same source already in the inbox (one per
   * source) is re-armed instead (`refresh_unread`): unread again, moved to the top, with the new
   * type and details.
   */
  record(draft: ActivityDraft): ActivityItem;
  /** Deletes every item `matches` picks (their source went), publishing each removal. */
  removeWhere(matches: (item: ActivityItem) => boolean): void;
  /** The viewer's unread items across every type. */
  unreadCount(): number;
}

/** Whether `tab` shows items of `type`. */
export function inTab(tab: ActivityTab, type: ActivityEventType): boolean {
  return tab === "all" || TAB_TYPES[tab].includes(type);
}

/** The item after `action`, stamped `now`; the same object when nothing changes. */
export function applyAction(item: ActivityItem, action: ActivityAction, now: string): ActivityItem {
  let readAt = item.readAt;
  let handledAt = item.handledAt;

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
      readAt ??= now;
      handledAt = null;
      break;
  }

  if (readAt === item.readAt && handledAt === item.handledAt) return item;

  // Every state change bumps the sort key (`save_state`): the item moves to the top of its list.
  return withActivityState({ ...item, readAt, handledAt, updatedAt: now });
}

/** The action a body asks for (`{action}`); 422 for anything else. */
function actionOf(body: Json | undefined): ActivityAction {
  const action = ACTIONS.find((candidate) => candidate === stringField(body, "action"));

  if (action === undefined) {
    throw validation("action", "Action must be read, unread, handled or unhandled");
  }

  return action;
}

/** Creates the activity module. */
export function createActivity(ctx: S2Context): Activity {
  const items = () => ctx.world().activity;

  const unreadCount = () => {
    let count = 0;

    for (const item of items().values()) if (activityStateOf(item) === "unread") count++;

    return count;
  };

  const itemOr404 = (id: number): ActivityItem => {
    const item = items().get(id);

    if (item === undefined) throw notFound("Activity item not found");

    return item;
  };

  const publish = (item: ActivityItem) => {
    const data: ActivityItemChanged = { item, unreadCount: unreadCount() };

    ctx.publish([{ topic: "user", type: "activity.item", data }]);
  };

  const keyOf = (item: ActivityItem) => ({ at: item.updatedAt, id: item.id });

  const list = (query: URLSearchParams): ActivityList => {
    const status = oneOf(query.get("status"), STATES, "unread");
    const tab = oneOf(query.get("type"), TABS, "all");

    const matching = [...items().values()].filter(
      (item) => item.state === status && inTab(tab, item.eventType),
    );

    const page = keysetPage(matching, keyOf, beforeOf(query), ACTIVITY_PAGE_SIZE);

    const creators = page.rows.flatMap((item) => {
      const creatorId = item.source.creatorId;

      return creatorId === null ? [] : [creatorId];
    });

    return {
      items: [...page.rows],
      users: ctx.usersFor(creators),
      unreadCount: unreadCount(),
      nextCursor: page.nextCursor,
    };
  };

  const change = (id: number, action: ActivityAction): ActivityItemChanged => {
    const current = itemOr404(id);
    const next = applyAction(current, action, iso(ctx.now()));

    if (next !== current) {
      items().set(id, next);
      publish(next);
    }

    return { item: next, unreadCount: unreadCount() };
  };

  const record = (draft: ActivityDraft): ActivityItem => {
    const world = ctx.world();
    const at = iso(draft.at ?? ctx.now());

    const existing = [...items().values()].find(
      (item) =>
        item.source.sourceType === draft.source.sourceType &&
        item.source.sourceId === draft.source.sourceId,
    );

    const item: ActivityItem =
      existing === undefined
        ? {
            id: world.nextActivityId++,
            eventType: draft.eventType,
            state: "unread",
            readAt: null,
            handledAt: null,
            createdAt: at,
            updatedAt: at,
            source: draft.source,
          }
        : {
            ...existing,
            eventType: draft.eventType,
            state: "unread",
            readAt: null,
            handledAt: null,
            updatedAt: at,
            source: draft.source,
          };

    items().set(item.id, item);
    publish(item);

    return item;
  };

  const removeWhere = (matches: (item: ActivityItem) => boolean) => {
    for (const item of [...items().values()]) {
      if (!matches(item)) continue;

      items().delete(item.id);
      ctx.publish([
        {
          topic: "user",
          type: "activity.removed",
          data: { id: item.id, unreadCount: unreadCount() },
        },
      ]);
    }
  };

  return {
    routes: [
      route("GET", /^\/activity$/, (request) => ok(list(request.query))),
      route("GET", /^\/activity\/unread_count$/, () => ok({ unreadCount: unreadCount() })),
      route("PATCH", /^\/activity\/(\d+)$/, (request) => {
        const id = firstId(request);

        itemOr404(id);

        return ok(change(id, actionOf(request.body)));
      }),
      route("POST", /^\/activity\/(\d+)\/open$/, (request) => ok(change(firstId(request), "read"))),
    ],
    record,
    removeWhere,
    unreadCount,
  };
}

/** The composer's scheduled-message hooks that keep the inbox in step: drops recorded, cancels cleared. */
export function scheduledInboxHooks(names: Namer, activity: Activity): ScheduledHooks {
  return {
    dropped: (message) =>
      activity.record({
        eventType: "scheduled_message_dropped",
        source: droppedSource(names, message),
      }),
    cancelled: (id) =>
      activity.removeWhere(
        (item) => item.source.sourceType === "scheduled_message" && item.source.sourceId === id,
      ),
  };
}
