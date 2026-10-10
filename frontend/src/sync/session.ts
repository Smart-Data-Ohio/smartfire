/**
 * What the UI asks for, as Effect programs. `runtime.ts` runs them for React as `actions`.
 * Failures land in the store (room errors, failed pages, failed sends), so these rarely fail.
 */
import { Clock, Effect, Option, Predicate, Result } from "effect";
import { readBoot } from "../api/boot.ts";
import type { ApiClient } from "../api/client.ts";
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
  joinedAtEpoch,
  noteJoined,
  resetJoinState,
  resetRoomRereads,
} from "../store/join-state.ts";
import { mutations, store } from "../store/store.ts";
import { Outbox, type SendOptions } from "./outbox.ts";
import { Presence } from "./presence.ts";
import {
  changedSince,
  interruptRoomRecovery,
  invalidateRoom,
  managementEpoch,
  recoverRejectedRoomRead,
  resetRoomHandoffs,
  roomRevision,
} from "./room-refresh.ts";
import { rowTicket, withRowTicket } from "./row-ticket.ts";
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
  /** This visit's reads a newer room outcome (a membership fact) refused. */
  readonly refused: number;
}

let nextVisitToken = 0,
  nextLoad = 0;

const visits = new Map<number, RoomVisit>();

function beginVisit(roomId: number, focusMessageId: number | null): RoomVisit {
  const visit: RoomVisit = { token: ++nextVisitToken, focusMessageId, load: 0, refused: 0 };

  visits.set(roomId, visit);

  return visit;
}

function sameVisit(roomId: number, token: number): boolean {
  return visits.get(roomId)?.token === token;
}

/** The visit on screen, or `null` when the room is closed. Recovery loops stop when it changes. */
export function roomVisitToken(roomId: number): number | null {
  return visits.get(roomId)?.token ?? null;
}

/** Test isolation. Visit tokens and join epochs outlive the store. */
export function resetRoomVisits(): void {
  visits.clear();
  resetJoinState();
  resetRoomRereads();
  resetRoomHandoffs();
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

  const loaded = yield* withRowTicket((since) =>
    sidebar().pipe(
      Effect.tap((data) => Effect.sync(() => mutations.loadSidebar(data, since))),
      Effect.tapError(() => Effect.sync(() => mutations.setSidebarFailed())),
      Effect.option,
    ),
  );

  if (Option.isNone(loaded)) {
    return;
  }

  const ids = presenceIds(loaded.value);

  if (ids.length > 0) {
    yield* presence(ids).pipe(
      Effect.tap((list) => Effect.sync(() => mutations.setPresence(list.presences))),
      Effect.catch((error) => Effect.logWarning("presence lookup failed", error.message)),
    );
  }
});

/** Whether the viewer sent to the room's timeline while its fresh window loaded. */
function sentWhileLoading(roomId: number): boolean {
  const state = store.getState();
  const viewerId = state.me?.user.id ?? state.boot?.user.id ?? null;

  return (
    (state.pendingByRoom[roomId]?.length ?? 0) > 0 ||
    (state.timelines[roomId]?.arrived ?? []).some(
      (id) => state.messages[id]?.creatorId === viewerId,
    )
  );
}

/** The first page: at the visit's focused message, the unread divider or the end. */
const loadFirstPage = Effect.fnUntraced(function* (
  roomId: number,
  token: number,
  unread: { readonly firstUnreadMessageId: number; readonly count: number } | null,
  load: number | null,
) {
  // A newer read of this visit (a recovery) loads its own page: this one's page, or its
  // failure, no longer counts.
  const current = () => sameVisit(roomId, token) && (load === null || latestLoad(roomId, load));

  if (!current()) {
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
    // A send from here while the page loaded lands at the present, beyond a window opened at the
    // first unread or a permalink: open at the present instead, where the reader sees it.
    Effect.flatMap((page) =>
      page.after !== null && current() && sentWhileLoading(roomId)
        ? messages(roomId, null)
        : Effect.succeed(page),
    ),
    Effect.tap((page) =>
      Effect.sync(() => {
        if (!current()) {
          return;
        }

        mutations.applyPage(roomId, page, "replace");
      }),
    ),
    Effect.catch((error) =>
      Effect.sync(() => {
        // The room left the screen meanwhile (unavailable, or a preview): that outcome stays.
        if (!current() || store.getState().rooms[roomId]?.detail == null) {
          return;
        }

        mutations.setPageFailed(roomId);
        mutations.setRoomError(roomId, error.message);
      }),
    ),
  );
});

/**
 * Boards keep their posts. A chat room replaces the window with this visit's first page. `load`
 * is the visit load of the read that landed `detail` (`null` for a join's), so a newer read's
 * page wins.
 */
const loadTimeline = Effect.fnUntraced(function* (
  roomId: number,
  token: number,
  detail: RoomDetail,
  load: number | null,
) {
  if (detail.room.kind === "board") {
    return;
  }

  mutations.setPageReplacing(roomId);
  yield* loadFirstPage(roomId, token, detail.unread, load);
});

/** Whether `load` is still the visit's latest room read. */
function latestLoad(roomId: number, load: number): boolean {
  return visits.get(roomId)?.load === load;
}

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
 * The read lost to a membership fact that arrived while it was in flight. A later read, with a
 * new sequence, fills what that fact did not (member counts, or the first detail). Losses while
 * that re-read is in flight share one follow-up. Confirmed mutations are not reads.
 */
let retryRejectedLoad: (roomId: number, token: number) => Effect.Effect<void, never, ApiClient> =
  () => Effect.void;

/**
 * The room's detail, then its first page. A 404 fetches the join preview; a preview 404 means the
 * room is unavailable. A preview applies only when its load is still the visit's latest. An older
 * room outcome cannot replace a newer one. Cached membership never overrides a 404, except detail
 * this session's join installed while that join's epoch is still current.
 *
 * A recovery re-read does not call `setRoomLoading`: it keeps whatever is on screen and replaces
 * it only when the fresh response still belongs to this visit. Only the visit's own initial load
 * sets `loading`.
 */
const readRoom = Effect.fnUntraced(function* (roomId: number, token: number) {
  if (!sameVisit(roomId, token)) return;

  const rowsSince = yield* rowTicket;

  const open = visits.get(roomId);
  const load = open?.token === token ? ++nextLoad : 0;
  const started = beginRoomRequest();

  if (open !== undefined && open.token === token) {
    visits.set(roomId, { ...open, load });
  }

  const loaded = yield* Effect.result(room(roomId));

  if (!sameVisit(roomId, token)) {
    return;
  }

  if (Result.isSuccess(loaded)) {
    const detail = withSidebarRow(loaded.success, store.getState().sidebar.rows[roomId]);

    if (mutations.setRoomDetail(detail, started)) {
      clearRoomJoin(roomId);
      yield* loadTimeline(roomId, token, detail, load);

      return;
    }

    if (!(yield* rereadRefused(roomId, token))) {
      return;
    }

    // At the cap. A removal since (the row is gone) outranks this older detail: revocations win,
    // so the room shows unavailable rather than content from before the removal.
    const row = store.getState().sidebar.rows[roomId];

    if (row === undefined) {
      if (mutations.setRoomUnavailable(roomId, beginRoomRequest(), rowsSince)) {
        mutations.setPageFailed(roomId);
      }

      return;
    }

    const capped = withSidebarRow(loaded.success, row);

    if (mutations.setRoomDetail(capped, beginRoomRequest())) {
      clearRoomJoin(roomId);
      yield* loadTimeline(roomId, token, capped, load);
    }

    return;
  }

  // A failure from a read this visit has since read again (a recovery) changes nothing: the newer
  // read owns the screen.
  if (!latestLoad(roomId, load)) {
    return;
  }

  const kept = currentJoinDetail(roomId);

  if (kept !== null) {
    if (mutations.setRoomDetail(kept, started)) {
      yield* loadTimeline(roomId, token, kept, load);
    }

    return;
  }

  if (Predicate.isTagged(loaded.failure, "NotFound")) {
    const preview = yield* Effect.result(openRoomPreview(roomId));
    const latest = sameVisit(roomId, token) && latestLoad(roomId, load);

    if (!latest) {
      return;
    }

    // A newer room outcome (a membership fact) outranks this preview or 404: read the room
    // again, as for a rejected detail, up to the cap; then this outcome lands.
    if (Result.isSuccess(preview)) {
      const landed =
        mutations.setRoomPreview(roomId, preview.success, started) ||
        ((yield* rereadRefused(roomId, token)) &&
          mutations.setRoomPreview(roomId, preview.success, beginRoomRequest()));

      if (landed) {
        clearRoomJoin(roomId);
        mutations.setPageFailed(roomId);
      }

      return;
    }

    if (Predicate.isTagged(preview.failure, "NotFound")) {
      const landed =
        mutations.setRoomUnavailable(roomId, started, rowsSince) ||
        ((yield* rereadRefused(roomId, token)) &&
          mutations.setRoomUnavailable(roomId, beginRoomRequest(), rowsSince));

      if (landed) {
        mutations.setPageFailed(roomId);
      }

      return;
    }

    yield* failRead(roomId, token, started, preview.failure.message);

    return;
  }

  yield* failRead(roomId, token, started, loaded.failure.message);
}, Effect.scoped);

/**
 * A room read (the visit's latest, outcome sequence `started`) failed: the error shows, and the
 * page stops waiting, only when no newer room outcome has landed. When one has (a membership
 * fact), the room is read again rather than left as it was, up to the cap.
 */
const failRead = Effect.fnUntraced(function* (
  roomId: number,
  token: number,
  started: number,
  message: string,
) {
  const landed =
    mutations.setRoomError(roomId, message, started) ||
    ((yield* rereadRefused(roomId, token)) &&
      mutations.setRoomError(roomId, message, beginRoomRequest()));

  if (landed) {
    mutations.setPageFailed(roomId);
  }
});

/** A visit reads its room at most this many times while newer membership facts refuse each read. */
const REFUSED_READS = 3;

/**
 * A newer room outcome refused this visit's read. Below the cap the room is read again (false).
 * At the cap there is no further read: answers whether the caller lands its own outcome under a
 * new sequence, which it does only while the room is still loading. Pushes that keep coming
 * still get their one follow-up each, and a read no fact overtakes lands as usual.
 */
const rereadRefused = Effect.fnUntraced(function* (roomId: number, token: number) {
  const open = visits.get(roomId);

  if (open === undefined || open.token !== token) return false;

  const refused = open.refused + 1;

  visits.set(roomId, { ...open, refused });

  if (refused < REFUSED_READS) {
    yield* retryRejectedLoad(roomId, token);

    return false;
  }

  return store.getState().rooms[roomId]?.status === "loading";
});

/** The visit's own initial load. Recovery calls `readRoom` and leaves the visible state alone. */
const loadRoom = Effect.fnUntraced(function* (roomId: number, token: number) {
  if (!sameVisit(roomId, token)) return;

  mutations.setRoomLoading(roomId);
  yield* readRoom(roomId, token);
});

retryRejectedLoad = Effect.fnUntraced(function* (roomId: number, token: number) {
  if (!sameVisit(roomId, token)) return;

  const context = yield* Effect.context<ApiClient>();
  const read = () => readRoom(roomId, token);

  // A metadata refresh's re-read may hold the slot: it then reads the room in full for this visit,
  // so the visit still reaches ready, error, unavailable or a preview.
  yield* recoverRejectedRoomRead(
    roomId,
    token,
    () => sameVisit(roomId, token),
    read,
    () => read().pipe(Effect.provideContext(context)),
  );
});

/**
 * Rooms a resync gave the viewer a membership in (`mutations.resyncSidebar`'s answer): an open
 * visit with no detail on screen (unavailable after a 404, a join preview, or a first read whose
 * outcome the resync claimed) reads its room again. One re-read per visit at a time, started only
 * for a read whose outcome a newer fact claimed, so it can't loop.
 */
export const recoverResyncedRooms = Effect.fnUntraced(function* (roomIds: readonly number[]) {
  for (const roomId of roomIds) {
    const token = roomVisitToken(roomId);

    if (token !== null && store.getState().rooms[roomId]?.detail == null) {
      yield* retryRejectedLoad(roomId, token);
    }
  }
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
  rowsSince: number,
) {
  if (!mutations.setRoomDetail(detail, started)) {
    return;
  }

  if (row !== null) {
    const viewerId = store.getState().me?.user.id ?? store.getState().boot?.user.id ?? 0;

    mutations.landReplyRows(
      [{ seq: 0, topic: `user:${viewerId}`, type: "sidebar.row.upserted", data: row }],
      yield* Clock.currentTimeMillis,
      rowsSince,
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

  yield* loadTimeline(roomId, visit.token, detail, null);
});

/**
 * The join body is stale: the epoch moved, or the visit changed, while the POST was in flight.
 * Re-read the room. A 404 whose revision stays put is final. Anything else, including a read a
 * newer change superseded, tries again — three times, then the room's ordinary load error.
 * Try again repeats this read. The stale body is never installed.
 */
const recoverJoin = Effect.fnUntraced(function* (roomId: number) {
  let failure = "Couldn't load this room";
  let started = 0;

  for (let attempt = 0; attempt < RECOVERY_ATTEMPTS; attempt++) {
    if (attempt > 0) {
      yield* Effect.sleep(attempt === 1 ? "200 millis" : "800 millis");
    }

    const outcome = yield* rereadJoined(roomId);

    if (outcome === true) {
      return;
    }

    started = outcome.started;

    if (outcome.failure !== null) {
      failure = outcome.failure;
    }
  }

  clearRoomJoin(roomId);

  // The error lands only when no room outcome newer than the last read has.
  if (mutations.setRoomError(roomId, failure, started)) {
    mutations.setPageFailed(roomId);
  }
});

/**
 * One of `recoverJoin`'s reads: true once the room is settled (installed, or unavailable), else
 * the read's outcome sequence and its failure's message (null for a read a newer change
 * superseded).
 */
const rereadJoined = Effect.fnUntraced(function* (roomId: number) {
  const revision = invalidateRoom(roomId);
  const since = managementEpoch();
  const rowsSince = yield* rowTicket;
  const started = beginRoomRequest();
  const fetched = yield* Effect.result(room(roomId));
  const superseded = roomRevision(roomId) !== revision || changedSince(roomId, since);

  if (Result.isSuccess(fetched) && !superseded) {
    const row = store.getState().sidebar.rows[roomId];

    yield* installJoined(roomId, withSidebarRow(fetched.success, row), null, started, rowsSince);

    return true;
  }

  if (Result.isFailure(fetched) && Predicate.isTagged(fetched.failure, "NotFound") && !superseded) {
    mutations.setRoomUnavailable(roomId, started, rowsSince);

    return true;
  }

  return { failure: Result.isFailure(fetched) ? fetched.failure.message : null, started };
}, Effect.scoped);

/**
 * Joins an open room. The POST body is a hint: it lands only when the epoch and the visit are
 * still the ones the request started with. Otherwise the room is read from the server.
 */
export const joinOpenRoom = Effect.fn("session.joinOpenRoom")(function* (roomId: number) {
  const since = managementEpoch();
  const rowsSince = yield* rowTicket;
  const token = visits.get(roomId)?.token ?? null;
  const joined = yield* postJoin(roomId);
  const topics = yield* Topics;

  yield* topics.forgetRejected(roomTopic(roomId));

  if (changedSince(roomId, since) || (visits.get(roomId)?.token ?? null) !== token) {
    yield* recoverJoin(roomId);

    return;
  }

  yield* installJoined(roomId, joined.detail, joined.row, beginRoomRequest(), rowsSince);
}, Effect.scoped);

/**
 * Opens a room view: subscribes to its topic, says present, loads the detail and the first page
 * (around `focusMessageId`, else around the first unread when many are unread, else the newest).
 */
export const openRoom = Effect.fn("session.openRoom")(function* (
  roomId: number,
  focusMessageId: number | null,
) {
  const visit = beginVisit(roomId, focusMessageId);

  yield* interruptRoomRecovery(roomId);

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

  yield* interruptRoomRecovery(roomId);
  yield* loadRoom(roomId, visit.token);
});

/** Closes a room view: releases its topic, says absent, stops typing, forgets the divider. */
export const closeRoom = Effect.fn("session.closeRoom")(function* (roomId: number) {
  visits.delete(roomId);
  yield* interruptRoomRecovery(roomId);

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

  // A replacement on its way (the present, or around a message) supersedes newer pages. One that
  // landed first would go in above a send made meanwhile and push its pending row out of view.
  const replacing = direction === "newer" && timeline.arrived !== null;

  if (from === null || busy || replacing) {
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
