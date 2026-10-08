/**
 * What the UI asks for, as Effect programs. `runtime.ts` runs them for React as `actions`.
 * Failures land in the store (room errors, failed pages, failed sends), so these rarely fail.
 */
import { Clock, Effect, Option, Predicate, Result } from "effect";
import { readBoot } from "../api/boot.ts";
import {
  me,
  messages,
  openRoomPreview,
  type PageCursor,
  joinOpenRoom as postJoin,
  markRead as postRead,
  presence,
  room,
  sidebar,
} from "../api/endpoints.ts";
import type { RoomDetail } from "../gen/RoomDetail.ts";
import type { Sidebar } from "../gen/Sidebar.ts";
import type { SidebarRow } from "../gen/SidebarRow.ts";
import { mutations, store } from "../store/store.ts";
import { Outbox, type SendOptions } from "./outbox.ts";
import { Presence } from "./presence.ts";
import { changedSince, invalidateRoom, managementEpoch, roomRevision } from "./room-refresh.ts";
import { Topics } from "./topics.ts";
import { Typing } from "./typing.ts";

/** Above this many unread messages a room opens at the first unread one, not the newest page. */
export const OPEN_AT_UNREAD_ABOVE = 40;

const roomTopic = (roomId: number) => `room:${roomId}`;

/**
 * Rooms on screen. `openRoom` adds one and `closeRoom` removes it before either yields, so a join
 * that answers after the viewer has left still records the membership without saying present or
 * loading the timeline.
 */
const viewedRooms = new Set<number>();

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

/** The first page: at the focused message, the unread divider or the end. */
const loadFirstPage = Effect.fnUntraced(function* (
  roomId: number,
  focusMessageId: number | null,
  unread: { readonly firstUnreadMessageId: number; readonly count: number } | null,
) {
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
 * The room's detail, then its first page. A 404 on an open room the viewer may join lands the
 * preview instead of an error; every other failure stays an error.
 */
const loadRoom = Effect.fnUntraced(function* (roomId: number, focusMessageId: number | null) {
  mutations.setRoomLoading(roomId);
  mutations.setPageReplacing(roomId);

  const loaded = yield* Effect.result(room(roomId));

  if (Result.isSuccess(loaded)) {
    mutations.setRoomDetail(loaded.success);
    yield* loadFirstPage(roomId, focusMessageId, loaded.success.unread);

    return;
  }

  mutations.setPageFailed(roomId);

  if (Predicate.isTagged(loaded.failure, "NotFound")) {
    const preview = yield* Effect.result(openRoomPreview(roomId));

    if (Result.isSuccess(preview)) {
      mutations.setRoomPreview(roomId, preview.success);

      return;
    }
  }

  mutations.setRoomError(roomId, loaded.failure.message);
});

/**
 * Lands a join: the membership, and the sidebar row when the membership is visible. Presence and
 * the timeline follow only while the room is still the one on screen (a reply can arrive after
 * the viewer has left). The topic was subscribed when the preview opened, before the membership
 * existed, so it is subscribed again now when this client still holds it.
 */
const landJoined = Effect.fnUntraced(function* (
  roomId: number,
  detail: RoomDetail,
  row: SidebarRow | null,
  focusMessageId: number | null,
) {
  mutations.setRoomDetail(detail);

  if (row !== null) {
    const viewerId = store.getState().me?.user.id ?? store.getState().boot?.user.id ?? 0;

    mutations.applyEvents(
      [{ seq: 0, topic: `user:${viewerId}`, type: "sidebar.row.upserted", data: row }],
      yield* Clock.currentTimeMillis,
    );
  }

  invalidateRoom(roomId);

  const topics = yield* Topics;

  yield* topics.resubscribe(roomTopic(roomId));

  if (!viewedRooms.has(roomId)) {
    return;
  }

  const presenceService = yield* Presence;

  yield* presenceService.enter(roomId);
  mutations.setPageReplacing(roomId);
  yield* loadFirstPage(roomId, focusMessageId, detail.unread);
});

/**
 * The join reply is older than a management change that landed while it was in flight (the room
 * was deleted, say). Read the room again instead of restoring what that change removed.
 */
const recoverSupersededJoin = Effect.fnUntraced(function* (
  roomId: number,
  focusMessageId: number | null,
) {
  const revision = invalidateRoom(roomId);
  const since = managementEpoch();
  const fetched = yield* Effect.result(room(roomId));

  if (roomRevision(roomId) !== revision || changedSince(roomId, since)) {
    return;
  }

  if (Result.isFailure(fetched)) {
    if (Predicate.isTagged(fetched.failure, "NotFound")) {
      mutations.setRoomUnavailable(roomId);
    }

    return;
  }

  const row = store.getState().sidebar.rows[roomId];

  yield* landJoined(
    roomId,
    row === undefined
      ? fetched.success
      : {
          ...fetched.success,
          room: row.room,
          membership: row.membership,
          displayName: row.displayName,
          directMemberIds: row.directMemberIds,
        },
    null,
    focusMessageId,
  );
});

/**
 * Joins an open room. The membership and sidebar row always land, unless a management change
 * during the request made the reply stale. Presence and the first page (around `focusMessageId`
 * when the join came from a permalink) land only if the room is still on screen.
 */
export const joinOpenRoom = Effect.fn("session.joinOpenRoom")(function* (
  roomId: number,
  focusMessageId: number | null,
) {
  const since = managementEpoch();
  const joined = yield* postJoin(roomId);

  if (changedSince(roomId, since)) {
    yield* recoverSupersededJoin(roomId, focusMessageId);

    return;
  }

  yield* landJoined(roomId, joined.detail, joined.row, focusMessageId);
});

/**
 * Opens a room view: subscribes to its topic, says present, loads the detail and the first page
 * (around `focusMessageId`, else around the first unread when many are unread, else the newest).
 */
export const openRoom = Effect.fn("session.openRoom")(function* (
  roomId: number,
  focusMessageId: number | null,
) {
  viewedRooms.add(roomId);

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
  viewedRooms.delete(roomId);

  const topics = yield* Topics;
  const presenceService = yield* Presence;
  const typing = yield* Typing;

  yield* typing.set(roomTopic(roomId), false);
  yield* topics.release(roomTopic(roomId));
  yield* presenceService.leave(roomId);
  mutations.clearUnreadDivider(roomId);
});

const loadPage = Effect.fnUntraced(function* (roomId: number, direction: "older" | "newer") {
  const timeline = store.getState().timelines[roomId];

  if (timeline === undefined) {
    return;
  }

  const from = direction === "older" ? timeline.before : timeline.after;
  const busy = direction === "older" ? timeline.loadingOlder : timeline.loadingNewer;

  if (from === null || busy) {
    return;
  }

  mutations.setPageLoading(roomId, direction);

  yield* messages(roomId, direction === "older" ? { before: from } : { after: from }).pipe(
    Effect.tap((page) => Effect.sync(() => mutations.applyPage(roomId, page, direction))),
    Effect.catch(() => Effect.sync(() => mutations.setPageFailed(roomId))),
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
