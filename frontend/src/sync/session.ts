/**
 * What the UI asks for, as Effect programs. `runtime.ts` runs them for React as `actions`.
 * Failures land in the store (room errors, failed pages, failed sends), so these rarely fail.
 */
import { Effect, Option } from "effect";
import { readBoot } from "../api/boot.ts";
import {
  me,
  messages,
  type PageCursor,
  markRead as postRead,
  presence,
  room,
  sidebar,
} from "../api/endpoints.ts";
import type { Sidebar } from "../gen/Sidebar.ts";
import { mutations, store } from "../store/store.ts";
import { Outbox, type SendOptions } from "./outbox.ts";
import { Presence } from "./presence.ts";
import { Topics } from "./topics.ts";
import { Typing } from "./typing.ts";

/** Above this many unread messages a room opens at the first unread one, not the newest page. */
export const OPEN_AT_UNREAD_ABOVE = 40;

const roomTopic = (roomId: number) => `room:${roomId}`;

/** Everyone the sidebar shows a presence dot for: direct-message members and placeholders. */
function presenceIds(data: Sidebar): readonly number[] {
  const ids = new Set<number>(data.directPlaceholderUserIds);

  for (const row of data.rows) {
    for (const id of row.directMemberIds) {
      ids.add(id);
    }
  }

  return [...ids];
}

/**
 * Boots the store: boot data, me, the sidebar (its failure is shown, not thrown) and presence
 * for the people the sidebar shows. Fails only when boot data or `me` can't be had.
 */
export const start = Effect.fn("session.start")(function* () {
  mutations.setBoot(yield* readBoot());
  mutations.setMe(yield* me());
  mutations.setSidebarLoading();

  const loaded = yield* sidebar().pipe(
    Effect.tapError(() => Effect.sync(() => mutations.setSidebarFailed())),
    Effect.option,
  );

  if (Option.isNone(loaded)) {
    return;
  }

  mutations.loadSidebar(loaded.value);

  const ids = presenceIds(loaded.value);

  if (ids.length > 0) {
    yield* presence(ids).pipe(
      Effect.tap((list) => Effect.sync(() => mutations.setPresence(list.presences))),
      Effect.catch((error) => Effect.logWarning("presence lookup failed", error.message)),
    );
  }
});

/** The room's detail, then its first page: at the focused message, the unread divider or the end. */
const loadRoom = Effect.fnUntraced(function* (roomId: number, focusMessageId: number | null) {
  mutations.setRoomLoading(roomId);

  const detail = yield* room(roomId).pipe(
    Effect.tapError((error) =>
      Effect.sync(() => {
        mutations.setPageFailed(roomId);
        mutations.setRoomError(roomId, error.message);
      }),
    ),
    Effect.option,
  );

  if (Option.isNone(detail)) {
    return;
  }

  mutations.setRoomDetail(detail.value);

  if (detail.value.room.kind === "board") return;

  mutations.setPageReplacing(roomId);
  const unread = detail.value.unread;
  let cursor: PageCursor = null;

  if (focusMessageId !== null) {
    cursor = { around: focusMessageId };
  } else if (unread !== null && unread.count > OPEN_AT_UNREAD_ABOVE) {
    cursor = { around: unread.firstUnreadMessageId };
  }

  yield* messages(roomId, cursor).pipe(
    Effect.tap((page) => Effect.sync(() => mutations.applyPage(roomId, page, "replace"))),
    Effect.catch((error) =>
      Effect.sync(() => {
        mutations.setPageFailed(roomId);
        mutations.setRoomError(roomId, error.message);
      }),
    ),
  );
});

/**
 * Opens a room view: subscribes to its topic, says present, loads the detail and the first page
 * (around `focusMessageId`, else around the first unread when many are unread, else the newest).
 */
export const openRoom = Effect.fn("session.openRoom")(function* (
  roomId: number,
  focusMessageId: number | null,
) {
  const topics = yield* Topics;
  const presenceService = yield* Presence;

  yield* topics.acquire(roomTopic(roomId));
  yield* presenceService.enter(roomId);
  yield* loadRoom(roomId, focusMessageId);
});

/** Loads an open room again after an error; the subscription and presence `openRoom` took hold. */
export const reloadRoom = Effect.fn("session.reloadRoom")(function* (
  roomId: number,
  focusMessageId: number | null,
) {
  yield* loadRoom(roomId, focusMessageId);
});

/** Closes a room view: releases its topic, says absent, stops typing, forgets the divider. */
export const closeRoom = Effect.fn("session.closeRoom")(function* (roomId: number) {
  const topics = yield* Topics;
  const presenceService = yield* Presence;
  const typing = yield* Typing;

  yield* typing.set(roomTopic(roomId), false);
  yield* topics.release(roomTopic(roomId));
  yield* presenceService.leave(roomId);
  mutations.clearUnreadDivider(roomId);
});

const loadPage = Effect.fnUntraced(function* (roomId: number, direction: "older" | "newer") {
  if (store.getState().rooms[roomId]?.detail?.room.kind === "board") return;
  const timeline = store.getState().timelines[roomId];

  if (timeline === undefined) {
    return;
  }

  const from = direction === "older" ? timeline.before : timeline.after;
  const busy = direction === "older" ? timeline.loadingOlder : timeline.loadingNewer;

  if (from === null || busy) {
    return;
  }

  // A replace bumps this id. A late page or error is dropped once it no longer matches.
  const request = mutations.setPageLoading(roomId, direction);

  yield* messages(roomId, direction === "older" ? { before: from } : { after: from }).pipe(
    Effect.tap((page) => Effect.sync(() => mutations.applyPage(roomId, page, direction, request))),
    Effect.catch(() => Effect.sync(() => mutations.setPageFailed(roomId, direction, request))),
    Effect.onInterrupt(() =>
      Effect.sync(() => mutations.setPageFailed(roomId, direction, request)),
    ),
  );
});

/**
 * Loads the window around `messageId` into an open room, for views that need a message the loaded
 * window doesn't hold (the new-thread pane opened from a link). A failed load leaves the window.
 */
export const loadAround = Effect.fn("session.loadAround")(function* (
  roomId: number,
  messageId: number,
) {
  if (store.getState().rooms[roomId]?.detail?.room.kind === "board") return;
  mutations.setPageReplacing(roomId);

  yield* messages(roomId, { around: messageId }).pipe(
    Effect.tap((page) => Effect.sync(() => mutations.applyPage(roomId, page, "replace"))),
    Effect.catch((error) =>
      Effect.sync(() => mutations.setPageFailed(roomId)).pipe(
        Effect.andThen(Effect.logWarning("message lookup failed", error.message)),
      ),
    ),
  );
});

/** The page before the loaded window; a no-op at the start of the room or while one loads. */
export const loadOlder = Effect.fn("session.loadOlder")(function* (roomId: number) {
  yield* loadPage(roomId, "older");
});

/** The page after the loaded window; a no-op at the present or while one loads. */
export const loadNewer = Effect.fn("session.loadNewer")(function* (roomId: number) {
  yield* loadPage(roomId, "newer");
});

/** Replaces the window with the newest page. */
export const jumpToPresent = Effect.fn("session.jumpToPresent")(function* (roomId: number) {
  if (store.getState().rooms[roomId]?.detail?.room.kind === "board") return;
  mutations.setPageReplacing(roomId);

  yield* messages(roomId, null).pipe(
    Effect.tap((page) => Effect.sync(() => mutations.applyPage(roomId, page, "replace"))),
    Effect.catch(() => Effect.sync(() => mutations.setPageFailed(roomId))),
  );
});

/** Sends a message (optimistically), to the room or one of its threads; typing stops. */
export const send = Effect.fn("session.send")(function* (
  roomId: number,
  markdown: string,
  options: SendOptions = {},
) {
  const typing = yield* Typing;
  const outbox = yield* Outbox;

  const threadId = options.threadId ?? null;

  yield* typing.set(threadId === null ? roomTopic(roomId) : `thread:${threadId}`, false);

  return yield* outbox.send(roomId, markdown, options);
});

/** Marks the room read here at once, then tells the server. */
export const markRead = Effect.fn("session.markRead")(function* (roomId: number) {
  mutations.markRoomRead(roomId);

  yield* postRead(roomId).pipe(
    Effect.catch((error) => Effect.logWarning("mark read failed", error.message)),
  );
});
