/**
 * The activity inbox's actions as Effect programs. Loads land in the store (failures too, as the
 * list's error); state changes show at once, moving the item between lists and the badge, and
 * roll back if the server refuses. Every reply carries the item and the unread count, which win.
 */
import { Clock, Effect } from "effect";
import * as api from "../api/activity-endpoints.ts";
import type { ActivityState } from "../gen/ActivityState.ts";
import type { ActivityTab } from "../gen/ActivityTab.ts";
import {
  type ActivityAction,
  activityListOf,
  nextActivityItem,
  unreadDelta,
} from "../store/activity.ts";
import { mutations, store } from "../store/store.ts";

/** Loads (or reloads) a tab's first page in one state; a failure lands as the list's error. */
export const load = Effect.fn("activity.load")(function* (tab: ActivityTab, status: ActivityState) {
  mutations.setActivityListLoading(tab, status, false);

  yield* api.activityList(status, tab, null).pipe(
    Effect.tap((page) =>
      Effect.sync(() => mutations.landActivityPage(tab, status, page, "replace")),
    ),
    Effect.catch((error) =>
      Effect.sync(() => mutations.setActivityListFailed(tab, status, error.message)),
    ),
  );
});

/** Loads the next page, if there is one and none is on its way. */
export const loadMore = Effect.fn("activity.loadMore")(function* (
  tab: ActivityTab,
  status: ActivityState,
) {
  const list = activityListOf(store.getState(), tab, status);

  if (list.nextCursor === null || list.loadingMore || list.status !== "ready") {
    return;
  }

  mutations.setActivityListLoading(tab, status, true);

  yield* api.activityList(status, tab, list.nextCursor).pipe(
    Effect.tap((page) => Effect.sync(() => mutations.landActivityPage(tab, status, page, "more"))),
    Effect.catch((error) =>
      Effect.sync(() => mutations.setActivityListFailed(tab, status, error.message)),
    ),
  );
});

/** Refreshes the badge. */
export const loadUnreadCount = Effect.fn("activity.loadUnreadCount")(function* () {
  const { unreadCount } = yield* api.activityUnreadCount();

  mutations.setActivityUnreadCount(unreadCount);

  return unreadCount;
});

/**
 * Applies `action` here at once (the item moves lists, the badge follows), then on the server,
 * whose reply wins; a refusal puts the item and the badge back.
 */
const change = (
  activityItemId: number,
  action: ActivityAction,
  request: (id: number) => ReturnType<typeof api.openActivityItem>,
) =>
  Effect.gen(function* () {
    const state = store.getState();
    const before = state.activity.items[activityItemId];
    const count = state.activity.unreadCount;

    if (before !== undefined) {
      const now = new Date(yield* Clock.currentTimeMillis).toISOString();
      const after = nextActivityItem(before, action, now);

      if (after !== before) {
        mutations.applyActivityItem(
          after,
          count === null ? null : Math.max(0, count + unreadDelta(before, after)),
        );
      }
    }

    const reply = yield* request(activityItemId).pipe(
      Effect.tapError(() =>
        Effect.sync(() => {
          if (before !== undefined) {
            mutations.applyActivityItem(before, count);
          }
        }),
      ),
    );

    mutations.applyActivityItem(reply.item, reply.unreadCount);

    return reply.item;
  });

/** Marks it read or unread, handled or not; answers the item as the server left it. */
export const setState = Effect.fn("activity.setState")(function* (
  activityItemId: number,
  action: ActivityAction,
) {
  return yield* change(activityItemId, action, (id) => api.updateActivityItem(id, action));
});

/**
 * Opens it: marks it read (never handled) here and on the server. Answers the item; go to its
 * source with `activityDestination(item)`.
 */
export const open = Effect.fn("activity.open")(function* (activityItemId: number) {
  return yield* change(activityItemId, "read", api.openActivityItem);
});
