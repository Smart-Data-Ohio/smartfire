/**
 * The Saved page's actions as Effect programs. Loads land in the store (failures as the list's
 * error) unless the list reloaded meanwhile. Marking done, reopening and removing show at once,
 * one change per item at a time, and roll back if the server refuses, unless an event changed
 * the item meanwhile; if the server no longer has the item (404), the lists reload instead.
 * Saving with a reminder goes through `POST /saved` again, as the server wants.
 */
import { Clock, Effect, Predicate } from "effect";
import { saveMessage, unsave } from "../api/message-endpoints.ts";
import * as api from "../api/saved-endpoints.ts";
import type { SavedFilter } from "../gen/SavedFilter.ts";
import type { SavedItem } from "../gen/SavedItem.ts";
import type { SavedStatus } from "../gen/SavedStatus.ts";
import { savedListOf } from "../store/saved-list.ts";
import { mutations, store } from "../store/store.ts";
import { keyedSerial } from "./serial.ts";

/** Loads (or reloads) a filter's first page; a failure lands as the list's error. */
export const load = Effect.fn("saved.load")(function* (filter: SavedFilter) {
  mutations.setSavedListLoading(filter, false);

  const { generation } = savedListOf(store.getState(), filter);

  yield* api.savedList(filter, null).pipe(
    Effect.tap((page) =>
      Effect.sync(() => mutations.landSavedPage(filter, page, "replace", generation)),
    ),
    Effect.catch((error) =>
      Effect.sync(() => mutations.setSavedListFailed(filter, error.message, generation)),
    ),
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
    Effect.tap((page) =>
      Effect.sync(() => mutations.landSavedPage(filter, page, "more", list.generation)),
    ),
    Effect.catch((error) =>
      Effect.sync(() => mutations.setSavedListFailed(filter, error.message, list.generation)),
    ),
  );
});

/** Changes to one item go one at a time, so each rolls back to a copy no other is holding. */
const serial = keyedSerial<number>();

/**
 * After a refused change: a 404 means the item is already gone, so the lists reload; otherwise
 * `rollBack` puts it back if nothing changed it meanwhile.
 */
const refused = (error: { readonly _tag: string }, rollBack: () => void) =>
  Effect.sync(() => {
    if (Predicate.isTagged(error, "NotFound")) {
      mutations.markSavedStale();
    } else {
      rollBack();
    }
  });

/** After an interrupted change, whose outcome is unknown: roll back, and reload to be sure. */
const interrupted = (rollBack: () => void) =>
  Effect.sync(() => {
    rollBack();
    mutations.markSavedStale();
  });

/** Marks it done or reopens it, at once; the server's item wins. */
export const setStatus = Effect.fn("saved.setStatus")(function* (
  savedItemId: number,
  status: SavedStatus,
) {
  return yield* serial.run(
    savedItemId,
    Effect.gen(function* () {
      const before = store.getState().savedList.items[savedItemId];

      const optimistic =
        before !== undefined && before.status !== status ? { ...before, status } : null;

      if (optimistic !== null) {
        mutations.applySavedChange(optimistic.messageId, optimistic);
      }

      const rollBack = () => {
        if (
          before !== undefined &&
          optimistic !== null &&
          store.getState().savedList.items[savedItemId] === optimistic
        ) {
          mutations.applySavedChange(before.messageId, before);
        }
      };

      const item = yield* api.updateSavedItem(savedItemId, status).pipe(
        Effect.tapError((error) => refused(error, rollBack)),
        Effect.onInterrupt(() => interrupted(rollBack)),
      );

      mutations.applySavedChange(item.messageId, item);

      return item;
    }),
  );
});

/** Unsaves it, at once (its reminder items go too, on the server). */
export const remove = Effect.fn("saved.remove")(function* (savedItemId: number) {
  yield* serial.run(
    savedItemId,
    Effect.gen(function* () {
      const before = store.getState().savedList.items[savedItemId];

      if (before !== undefined) {
        mutations.applySavedChange(before.messageId, null);
      }

      const rollBack = () => {
        if (before !== undefined && store.getState().saved[before.messageId] === undefined) {
          mutations.applySavedChange(before.messageId, before);
        }
      };

      yield* unsave(savedItemId).pipe(
        Effect.tapError((error) => refused(error, rollBack)),
        Effect.onInterrupt(() => interrupted(rollBack)),
      );
    }),
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

/**
 * Saves a removed item's message again (Undo): with its reminder, while that's still to come,
 * and marked done again if it was. The server gives it a new id and `createdAt`, so it returns
 * at the top of its lists. Answers the restored item.
 */
export const restore = Effect.fn("saved.restore")(function* (removed: SavedItem) {
  const now = yield* Clock.currentTimeMillis;

  const remindAt =
    removed.remindedAt === null && removed.remindAt !== null && Date.parse(removed.remindAt) > now
      ? removed.remindAt
      : null;

  const saved = yield* saveMessage(removed.messageId, remindAt);

  mutations.applySavedChange(removed.messageId, saved);

  if (removed.status === saved.status) {
    return saved;
  }

  return yield* setStatus(saved.id, removed.status);
});
