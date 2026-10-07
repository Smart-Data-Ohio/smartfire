/**
 * The activity inbox's actions as Effect programs. Loads land in the store (failures too, as the
 * list's error) unless the list reloaded meanwhile; state changes show at once, moving the item
 * between lists and the badge, and roll back if the server refuses. Delayed replies keep newer
 * items and counts already in the store.
 */
import { Clock, Effect } from "effect";
import * as api from "../api/activity-endpoints.ts";
import { NetworkError } from "../api/errors.ts";
import type { ActivityItem } from "../gen/ActivityItem.ts";
import type { ActivityState } from "../gen/ActivityState.ts";
import type { ActivityTab } from "../gen/ActivityTab.ts";
import {
  type ActivityAction,
  activityListOf,
  activityLoadStart,
  nextActivityItem,
  unreadDelta,
} from "../store/activity.ts";
import { mutations, store } from "../store/store.ts";
import { keyedSerial } from "./serial.ts";

/** Loads (or reloads) a tab's first page in one state; a failure lands as the list's error. */
export const load = Effect.fn("activity.load")(function* (tab: ActivityTab, status: ActivityState) {
  mutations.setActivityListLoading(tab, status, false);

  const start = activityLoadStart(store.getState(), tab, status);

  yield* api.activityList(status, tab, null).pipe(
    Effect.tap((page) =>
      Effect.sync(() => mutations.landActivityPage(tab, status, page, "replace", start)),
    ),
    Effect.catch((error) =>
      Effect.sync(() =>
        mutations.setActivityListFailed(
          tab,
          status,
          error.message,
          start.generation,
          start.activityGeneration,
        ),
      ),
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

  const start = activityLoadStart(store.getState(), tab, status);

  yield* api.activityList(status, tab, list.nextCursor).pipe(
    Effect.tap((page) =>
      Effect.sync(() => mutations.landActivityPage(tab, status, page, "more", start)),
    ),
    Effect.catch((error) =>
      Effect.sync(() =>
        mutations.setActivityListFailed(
          tab,
          status,
          error.message,
          start.generation,
          start.activityGeneration,
        ),
      ),
    ),
  );
});

/** Refreshes the badge. Server revisions order overlapping replies. */
export const loadUnreadCount = Effect.fn("activity.loadUnreadCount")(function* () {
  const generation = store.getState().activity.generation;
  const unread = yield* api.activityUnreadCount();

  mutations.setActivityUnreadCount(unread, generation);

  return unread.unreadCount;
});

/** Changes to one item go one at a time, so each rolls back to a copy no other is holding. */
const serial = keyedSerial<number>();

/** Tells the changes on their way apart in the badge. */
let nextToken = 0;

/** An unanswered request must not hold the badge, an item lock or sync frames indefinitely. */
export const ACTIVITY_REQUEST_TIMEOUT = "15 seconds";

const stalled = (generation: number) =>
  Effect.gen(function* () {
    if (generation === store.getState().activity.generation) {
      mutations.beginActivityGeneration(false);
      yield* Effect.forkDetach(
        loadUnreadCount().pipe(
          Effect.timeout(ACTIVITY_REQUEST_TIMEOUT),
          Effect.catch((error) => Effect.logWarning("activity count refresh failed", error)),
        ),
      );
    }

    return yield* Effect.fail(
      new NetworkError({ message: "The activity change timed out. Please try again." }),
    );
  });

/**
 * Applies `action` here at once (the item moves lists, the badge follows), then on the server,
 * whose reply settles the change while preserving newer server values. A refusal drops an
 * unconfirmed adjustment; confirmed adjustments stay until the displayed count includes them.
 */
const change = (
  activityItemId: number,
  action: ActivityAction,
  request: (id: number) => ReturnType<typeof api.openActivityItem>,
) =>
  serial.run(
    activityItemId,
    Effect.gen(function* () {
      const generation = store.getState().activity.generation;
      const before = store.getState().activity.items[activityItemId];
      const token = nextToken++;
      let optimistic: ActivityItem | null = null;

      if (before !== undefined) {
        const now = new Date(yield* Clock.currentTimeMillis).toISOString();
        const after = nextActivityItem(before, action, now);

        if (after !== before) {
          optimistic = after;
          mutations.showActivityChange(after, token, unreadDelta(before, after));
        }
      }

      // A failure or interruption removes only unconfirmed badge adjustments.
      const reply = yield* request(activityItemId).pipe(
        Effect.timeoutOrElse({
          duration: ACTIVITY_REQUEST_TIMEOUT,
          orElse: () => stalled(generation),
        }),
        Effect.onError(() =>
          Effect.sync(() =>
            mutations.endActivityChange({
              generation,
              token,
              optimistic,
              settled: before ?? null,
              unread: null,
            }),
          ),
        ),
      );

      mutations.endActivityChange({
        generation,
        token,
        optimistic,
        settled: reply.item,
        unread: reply,
      });

      return reply.item;
    }),
  );

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
