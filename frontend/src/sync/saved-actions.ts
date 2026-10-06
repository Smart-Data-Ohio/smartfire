/**
 * The Saved page's actions as Effect programs. Loads land in the store (failures as the list's
 * error); marking done, reopening and removing show at once and roll back if the server refuses.
 * Saving with a reminder goes through `POST /saved` again, as the server wants.
 */
import { Effect } from "effect";
import { saveMessage, unsave } from "../api/message-endpoints.ts";
import * as api from "../api/saved-endpoints.ts";
import type { SavedFilter } from "../gen/SavedFilter.ts";
import type { SavedStatus } from "../gen/SavedStatus.ts";
import { savedListOf } from "../store/saved-list.ts";
import { mutations, store } from "../store/store.ts";

/** Loads (or reloads) a filter's first page; a failure lands as the list's error. */
export const load = Effect.fn("saved.load")(function* (filter: SavedFilter) {
  mutations.setSavedListLoading(filter, false);

  yield* api.savedList(filter, null).pipe(
    Effect.tap((page) => Effect.sync(() => mutations.landSavedPage(filter, page, "replace"))),
    Effect.catch((error) => Effect.sync(() => mutations.setSavedListFailed(filter, error.message))),
  );
});

/** Loads the next page, if there is one and none is on its way. */
export const loadMore = Effect.fn("saved.loadMore")(function* (filter: SavedFilter) {
  const list = savedListOf(store.getState(), filter);

  if (list.nextCursor === null || list.loadingMore || list.status !== "ready") {
    return;
  }

  mutations.setSavedListLoading(filter, true);

  yield* api.savedList(filter, list.nextCursor).pipe(
    Effect.tap((page) => Effect.sync(() => mutations.landSavedPage(filter, page, "more"))),
    Effect.catch((error) => Effect.sync(() => mutations.setSavedListFailed(filter, error.message))),
  );
});

/** Marks it done or reopens it, at once; the server's item wins. */
export const setStatus = Effect.fn("saved.setStatus")(function* (
  savedItemId: number,
  status: SavedStatus,
) {
  const before = store.getState().savedList.items[savedItemId];

  if (before !== undefined && before.status !== status) {
    mutations.applySavedChange(before.messageId, { ...before, status });
  }

  const item = yield* api.updateSavedItem(savedItemId, status).pipe(
    Effect.tapError(() =>
      Effect.sync(() => {
        if (before !== undefined) {
          mutations.applySavedChange(before.messageId, before);
        }
      }),
    ),
  );

  mutations.applySavedChange(item.messageId, item);

  return item;
});

/** Unsaves it, at once (its reminder items go too, on the server). */
export const remove = Effect.fn("saved.remove")(function* (savedItemId: number) {
  const before = store.getState().savedList.items[savedItemId];

  if (before !== undefined) {
    mutations.applySavedChange(before.messageId, null);
  }

  yield* unsave(savedItemId).pipe(
    Effect.tapError(() =>
      Effect.sync(() => {
        if (before !== undefined) {
          mutations.applySavedChange(before.messageId, before);
        }
      }),
    ),
  );
});

/**
 * Sets (`remindAt`, RFC 3339, in the future) or clears (`null`) the reminder by saving the
 * message again; saves it if it wasn't. Answers the item.
 */
export const setReminder = Effect.fn("saved.setReminder")(function* (
  messageId: number,
  remindAt: string | null,
) {
  const item = yield* saveMessage(messageId, remindAt);

  mutations.applySavedChange(messageId, item);

  return item;
});
