/**
 * Scheduled messages' actions as Effect programs, for the Scheduled page and the composer. Loads
 * land in the store (failures as the list's error); writes land the server's reply in every list
 * (the `scheduled.*` event that follows is then a no-op). Cancelling shows at once and comes back
 * if the server refuses (409 while it's sending).
 */
import { Effect } from "effect";
import * as api from "../api/composer-endpoints.ts";
import type { CreateScheduledMessage } from "../gen/CreateScheduledMessage.ts";
import type { UpdateScheduledMessage } from "../gen/UpdateScheduledMessage.ts";
import { type ScheduledListKey, scheduledListOf, scheduledQueryOf } from "../store/scheduled.ts";
import { mutations, store } from "../store/store.ts";

/** Loads (or reloads) a list's first page; a failure lands as the list's error. */
export const load = Effect.fn("scheduled.load")(function* (key: ScheduledListKey) {
  const { status, roomId } = scheduledQueryOf(key);

  mutations.setScheduledListLoading(key, false);

  yield* api.scheduledMessagePage(status, roomId, null).pipe(
    Effect.tap((page) => Effect.sync(() => mutations.landScheduledPage(key, page, "replace"))),
    Effect.catch((error) =>
      Effect.sync(() => mutations.setScheduledListFailed(key, error.message)),
    ),
  );
});

/** Loads the next page, if there is one and none is on its way. */
export const loadMore = Effect.fn("scheduled.loadMore")(function* (key: ScheduledListKey) {
  const list = scheduledListOf(store.getState(), key);
  const { status, roomId } = scheduledQueryOf(key);

  if (list.nextCursor === null || list.loadingMore || list.status !== "ready") {
    return;
  }

  mutations.setScheduledListLoading(key, true);

  yield* api.scheduledMessagePage(status, roomId, list.nextCursor).pipe(
    Effect.tap((page) => Effect.sync(() => mutations.landScheduledPage(key, page, "more"))),
    Effect.catch((error) =>
      Effect.sync(() => mutations.setScheduledListFailed(key, error.message)),
    ),
  );
});

/** Schedules a message in the room (or a thread of it); it joins the pending lists. */
export const create = Effect.fn("scheduled.create")(function* (
  roomId: number,
  body: CreateScheduledMessage,
) {
  const created = yield* api.scheduleMessage(roomId, body);

  mutations.applyScheduled(created);

  return created;
});

/** New text, a new time, or both (an absent field keeps its value). */
export const update = Effect.fn("scheduled.update")(function* (
  scheduledMessageId: number,
  body: UpdateScheduledMessage,
) {
  const updated = yield* api.updateScheduledMessage(scheduledMessageId, body);

  mutations.applyScheduled(updated);

  return updated;
});

/**
 * Posts it at once. Answers `"sent"` (it moved to Past) or `"held"` (202: another runner holds it
 * or its thread is locked; it stays scheduled). A drop instead rejects with the reason; the lists
 * reload, since the dropped copy isn't in the reply.
 */
export const sendNow = Effect.fn("scheduled.sendNow")(function* (scheduledMessageId: number) {
  const message = yield* api
    .sendScheduledNow(scheduledMessageId)
    .pipe(
      Effect.tapErrorTag("Validation", () => Effect.sync(() => mutations.markScheduledStale())),
    );

  mutations.applyScheduled(message);

  return message.state === "sent" ? ("sent" as const) : ("held" as const);
});

/** Cancels it, at once; it comes back if the server refuses. */
export const cancel = Effect.fn("scheduled.cancel")(function* (scheduledMessageId: number) {
  const before = store.getState().scheduled.items[scheduledMessageId];

  mutations.removeScheduled(scheduledMessageId);

  yield* api.cancelScheduledMessage(scheduledMessageId).pipe(
    Effect.tapError(() =>
      Effect.sync(() => {
        if (before !== undefined) {
          mutations.applyScheduled(before);
        }
      }),
    ),
  );
});
