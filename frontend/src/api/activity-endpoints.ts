/**
 * The S3 activity inbox's endpoints: a page of the inbox, the unread badge, a state change and
 * `open`. Each validates the reply with its pinned schema (see endpoints.ts).
 */
import { Effect } from "effect";
import type { ActivityAction } from "../gen/ActivityAction.ts";
import type { ActivityItemChanged } from "../gen/ActivityItemChanged.ts";
import type { ActivityList } from "../gen/ActivityList.ts";
import type { ActivityState } from "../gen/ActivityState.ts";
import type { ActivityTab } from "../gen/ActivityTab.ts";
import type { ActivityUnreadCount } from "../gen/ActivityUnreadCount.ts";
import { call, get } from "./call.ts";
import {
  ActivityItemChanged as ActivityItemChangedSchema,
  ActivityList as ActivityListSchema,
  ActivityUnreadCount as ActivityUnreadCountSchema,
} from "./schema/activity.ts";
import { wire } from "./wire.ts";

/**
 * `GET /activity?status=&type=&before=`: up to 100 items in one state and tab, newest
 * `updatedAt` first. `before` is the previous page's `nextCursor`.
 */
export const activityList = Effect.fn("api.activityList")(function* (
  status: ActivityState,
  tab: ActivityTab,
  before: string | null,
) {
  const query = before === null ? { status, type: tab } : { status, type: tab, before };

  return yield* call(get("/activity", query), wire<ActivityList>(ActivityListSchema));
});

/** `GET /activity/unread_count`: the badge, across every type. */
export const activityUnreadCount = Effect.fn("api.activityUnreadCount")(function* () {
  return yield* call(
    get("/activity/unread_count"),
    wire<ActivityUnreadCount>(ActivityUnreadCountSchema),
  );
});

/**
 * `PATCH /activity/:id {action}`: mark read or unread, handled or not. The reply carries the
 * item and the unread count afterwards.
 */
export const updateActivityItem = Effect.fn("api.updateActivityItem")(function* (
  activityItemId: number,
  action: ActivityAction,
) {
  return yield* call(
    { method: "PATCH", path: `/activity/${activityItemId}`, body: { action } },
    wire<ActivityItemChanged>(ActivityItemChangedSchema),
  );
});

/** `POST /activity/:id/open`: marks it read (never handled) before the app goes to the source. */
export const openActivityItem = Effect.fn("api.openActivityItem")(function* (
  activityItemId: number,
) {
  return yield* call(
    { method: "POST", path: `/activity/${activityItemId}/open` },
    wire<ActivityItemChanged>(ActivityItemChangedSchema),
  );
});
