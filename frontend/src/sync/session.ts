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
import {
  beginRoomRequest,
  clearRoomJoin,
  detailInstalledGeneration,
  joinedAtEpoch,
  noteJoined,
  resetJoinState,
} from "../store/join-state.ts";
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
 * One open of a room. `openRoom` replaces it before yielding, so a preview, a timeline page, or a
 * join that started on an older visit cannot paint over the one on screen. The focus message is
 * this visit's, never the one captured when Join was clicked.
 */
interface RoomVisit {
  readonly token: number;
  readonly focusMessageId: number | null;
  readonly load: number;
}

let nextVisitToken = 0,
  nextLoad = 0;

const visits = new Map<number, RoomVisit>();

function beginVisit(roomId: number, focusMessageId: number | null): RoomVisit {
  const visit: RoomVisit = { token: ++nextVisitToken, focusMessageId, load: 0 };

  visits.set(roomId, visit);

  return visit;
}

function sameVisit(roomId: number, token: number): boolean {
  return visits.get(roomId)?.token === token;
}

/** Test isolation. Visit tokens and join epochs outlive the store. */
export function resetRoomVisits(): void {
  visits.clear();
  resetJoinState();
  nextVisitToken = 0;
  nextLoad = 0;
}

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

/** The first page: at the visit's focused message, the unread divider or the end. */
const loadFirstPage = Effect.fnUntraced(function* (
  roomId: number,
  token: number,
  unread: { readonly firstUnreadMessageId: number; readonly count: number } | null,
) {
  if (!sameVisit(roomId, token)) {
    return;
  }

  const focusMessageId = visits.get(roomId)?.focusMessageId ?? null;
  let cursor: PageCursor = null;

  if (focusMessageId !== null) {
    cursor = { around: focusMessageId };
  } else if (unread !== null && unread.count > OPEN_AT_UNREAD_ABOVE) {
    cursor = { around: unread.firstUnreadMessageId };
  }

  yield* messages(roomId, cursor).pipe(
    Effect.tap((page) =>
      Effect.sync(() => {
        if (!sameVisit(roomId, token)) {
          return;
        }

        mutations.applyPage(roomId, page, "replace");
      }),
    ),
    Effect.catch((error) =>
      Effect.sync(() => {
        if (!sameVisit(roomId, token)) {
          return;
        }

        mutations.setPageFailed(roomId);
        mutations.setRoomError(roomId, error.message);
      }),
    ),
  );
});

/** Boards keep their posts. A chat room replaces the window with this visit's first page. */
const loadTimeline = Effect.fnUntraced(function* (
  roomId: number,
  token: number,
  detail: RoomDetail,
) {
  if (detail.room.kind === "board") {
    return;
  }

  mutations.setPageReplacing(roomId);
  yield* loadFirstPage(roomId, token, detail.unread);
});

/** How many times a superseded join re-reads the room before it gives up and shows the load error. */
const RECOVERY_ATTEMPTS = 3;

/** Detail a join installed, and only while that join's epoch is still current. */
function currentJoinDetail(roomId: number): RoomDetail | null {
  const at = joinedAtEpoch(roomId);

  if (at === undefined || changedSince(roomId, at)) {
    return null;
  }

  return store.getState().rooms[roomId]?.detail ?? null;
}

/**
 * The room's detail, then its first page. A 404 fetches the join preview; a preview 404 means the
 * room is unavailable. A preview applies only when its load is still the visit's latest and no
 * detail from a request that began later has been installed. Cached membership never overrides a 404.
 * The exception is detail this session's join installed while that join's epoch is still current.
 */
const loadRoom = Effect.fnUntraced(function* (roomId: number, token: number) {
  const open = visits.get(roomId);
  const load = open?.token === token ? ++nextLoad : 0;
  const started = beginRoomRequest();

  if (open !== undefined && open.token === token) {
    visits.set(roomId, { ...open, load });
  }

  mutations.setRoomLoading(roomId);

  const loaded = yield* Effect.result(room(roomId));

  if (!sameVisit(roomId, token)) {
    return;
  }

  if (Result.isSuccess(loaded)) {
    clearRoomJoin(roomId);
    mutations.setRoomDetail(loaded.success, started);
    yield* loadTimeline(roomId, token, loaded.success);

    return;
  }

  const kept = currentJoinDetail(roomId);

  if (kept !== null) {
    mutations.setRoomDetail(kept, started);
    yield* loadTimeline(roomId, token, kept);

    return;
  }

  mutations.setPageFailed(roomId);

  if (Predicate.isTagged(loaded.failure, "NotFound")) {
    const preview = yield* Effect.result(openRoomPreview(roomId));
    const latest = sameVisit(roomId, token) && visits.get(roomId)?.load === load;

    if (!latest || detailInstalledGeneration(roomId) > started) {
      return;
    }

    if (Result.isSuccess(preview)) {
      clearRoomJoin(roomId);
      mutations.setRoomPreview(roomId, preview.success);

      return;
    }

    if (Predicate.isTagged(preview.failure, "NotFound")) {
      mutations.setRoomUnavailable(roomId);

      return;
    }

    mutations.setRoomError(roomId, preview.failure.message);

    return;
  }

  mutations.setRoomError(roomId, loaded.failure.message);
});

/** The sidebar row's facts win over a room read that raced a rename or a membership change. */
function withSidebarRow(detail: RoomDetail, row: SidebarRow | undefined): RoomDetail {
  if (row === undefined) {
    return detail;
  }

  return {
    ...detail,
    room: row.room,
    membership: row.membership,
    displayName: row.displayName,
    directMemberIds: row.directMemberIds,
  };
}

/**
 * Lands a join the server just confirmed. The visit on screen, if any, loads around its own
 * focus. No visit records the membership and leaves the timeline to the next open.
 */
const installJoined = Effect.fnUntraced(function* (
  roomId: number,
  detail: RoomDetail,
  row: SidebarRow | null,
  started: number,
) {
  mutations.setRoomDetail(detail, started);

  if (row !== null) {
    const viewerId = store.getState().me?.user.id ?? store.getState().boot?.user.id ?? 0;

    mutations.applyEvents(
      [{ seq: 0, topic: `user:${viewerId}`, type: "sidebar.row.upserted", data: row }],
      yield* Clock.currentTimeMillis,
    );
  }

  invalidateRoom(roomId);
  noteJoined(roomId, managementEpoch());

  // The preview's `present` was refused. Say it again only while this visit is still open.
  const visitBefore = visits.get(roomId);

  if (visitBefore !== undefined) {
    const presenceService = yield* Presence;

    yield* presenceService.enter(roomId);
  }

  const visit = visits.get(roomId);

  if (visit === undefined) {
    if (visitBefore !== undefined) {
      const presenceService = yield* Presence;

      yield* presenceService.leave(roomId);
    }

    return;
  }

  yield* loadTimeline(roomId, visit.token, detail);
});

/**
 * The join body is stale: the epoch moved, or the visit changed, while the POST was in flight.
 * Re-read the room. A 404 whose revision stays put is final. Anything else, including a read a
 * newer change superseded, tries again — three times, then the room's ordinary load error.
 * Try again repeats this read. The stale body is never installed.
 */
const recoverJoin = Effect.fnUntraced(function* (roomId: number) {
  let failure = "Couldn't load this room";

  for (let attempt = 0; attempt < RECOVERY_ATTEMPTS; attempt++) {
    if (attempt > 0) {
      yield* Effect.sleep(attempt === 1 ? "200 millis" : "800 millis");
    }

    const revision = invalidateRoom(roomId);
    const since = managementEpoch();
    const started = beginRoomRequest();
    const fetched = yield* Effect.result(room(roomId));
    const superseded = roomRevision(roomId) !== revision || changedSince(roomId, since);

    if (Result.isSuccess(fetched) && !superseded) {
      const row = store.getState().sidebar.rows[roomId];

      yield* installJoined(roomId, withSidebarRow(fetched.success, row), null, started);

      return;
    }

    if (
      Result.isFailure(fetched) &&
      Predicate.isTagged(fetched.failure, "NotFound") &&
      !superseded
    ) {
      mutations.setRoomUnavailable(roomId);

      return;
    }

    if (Result.isFailure(fetched)) {
      failure = fetched.failure.message;
    }
  }

  clearRoomJoin(roomId);
  mutations.setPageFailed(roomId);
  mutations.setRoomError(roomId, failure);
});

/**
 * Joins an open room. The POST body is a hint: it lands only when the epoch and the visit are
 * still the ones the request started with. Otherwise the room is read from the server.
 */
export const joinOpenRoom = Effect.fn("session.joinOpenRoom")(function* (roomId: number) {
  const since = managementEpoch();
  const token = visits.get(roomId)?.token ?? null;
  const started = beginRoomRequest();
  const joined = yield* postJoin(roomId);
  const topics = yield* Topics;

  yield* topics.forgetRejected(roomTopic(roomId));

  if (changedSince(roomId, since) || (visits.get(roomId)?.token ?? null) !== token) {
    yield* recoverJoin(roomId);

    return;
  }

  yield* installJoined(roomId, joined.detail, joined.row, started);
});

/**
 * Opens a room view: subscribes to its topic, says present, loads the detail and the first page
 * (around `focusMessageId`, else around the first unread when many are unread, else the newest).
 */
export const openRoom = Effect.fn("session.openRoom")(function* (
  roomId: number,
  focusMessageId: number | null,
) {
  const visit = beginVisit(roomId, focusMessageId);
  const topics = yield* Topics;
  const presenceService = yield* Presence;

  yield* topics.acquire(roomTopic(roomId));
  yield* presenceService.enter(roomId);
  yield* loadRoom(roomId, visit.token);
});

/** Loads an open room again after an error; the subscription and presence `openRoom` took hold. */
export const reloadRoom = Effect.fn("session.reloadRoom")(function* (
  roomId: number,
  focusMessageId: number | null,
) {
  const visit = beginVisit(roomId, focusMessageId);

  yield* loadRoom(roomId, visit.token);
});

/** Closes a room view: releases its topic, says absent, stops typing, forgets the divider. */
export const closeRoom = Effect.fn("session.closeRoom")(function* (roomId: number) {
  visits.delete(roomId);

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
