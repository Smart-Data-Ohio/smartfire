import {
  Clock,
  Context,
  Effect,
  Layer,
  Predicate,
  Queue,
  Ref,
  Result,
  Schedule,
  type Scope,
  Stream,
} from "effect";
import { board } from "../api/board-endpoints.ts";
import type { ApiClient } from "../api/client.ts";
import { messages, openRoomPreview, room, sidebar, users } from "../api/endpoints.ts";
import { threadMessages } from "../api/thread-endpoints.ts";
import type { ClientFrame } from "../gen/ClientFrame.ts";
import type { RoomDetail } from "../gen/RoomDetail.ts";
import type { ServerFrame } from "../gen/ServerFrame.ts";
import type { SyncEvent } from "../gen/SyncEvent.ts";
import { beginRoomRequest } from "../store/join-state.ts";
import type { ConnectionStatus, Timeline } from "../store/model.ts";
import { nextExpiry } from "../store/reducers.ts";
import type { SidebarState } from "../store/state.ts";
import { mutations, sidebarRowClock, store } from "../store/store.ts";
import { captureWorkRead, workDetailStale } from "../store/work.ts";
import { ACTIVITY_REQUEST_TIMEOUT, loadUnreadCount } from "./activity-actions.ts";
import { Cursor } from "./cursor.ts";
import { Lifecycle } from "./lifecycle.ts";
import { SyncLink } from "./link.ts";
import { Presence } from "./presence.ts";
import {
  invalidateRoom,
  markRoomsChanged,
  markSidebarSnapshot,
  recoverRejectedRoomRead,
  roomRefreshIds,
  roomRevision,
} from "./room-refresh.ts";
import { roomVisitToken } from "./session.ts";
import { paneProblem, UNAVAILABLE } from "./settle.ts";
import { emitResync, emitSyncEvents } from "./signals.ts";
import { SyncSocket, SyncSocketError } from "./socket.ts";
import {
  beginThreadLoad,
  finishThreadLoad,
  isGoneFocus,
  isLatestThreadLoad,
  loadThreadHeader,
  openAtNewest,
  pendingThreadFocus,
} from "./thread-loads.ts";
import { Topics } from "./topics.ts";
import { refresh as refreshWork } from "./work-actions.ts";

/**
 * A loaded window that stops short of the present: a permalink, a jump back, or a window the
 * newest page no longer meets. A resync's newest page would replace it and yank the reader away,
 * so the page around its middle is re-read in place instead (dropping what was deleted meanwhile);
 * the pages towards the present load fresh as they scroll down. (Checked when the newest page
 * lands, so a permalink that loads while the refetch is in flight wins.)
 */
function readingHistory(timeline: Timeline | undefined): boolean {
  return timeline !== undefined && timeline.status === "ready" && timeline.after !== null;
}

/** The message in the middle of a window away from the present, to re-read the page around. */
function middleOf(timeline: Timeline | undefined): number | null {
  if (timeline === undefined || !readingHistory(timeline)) {
    return null;
  }

  return timeline.ids[Math.floor(timeline.ids.length / 2)] ?? null;
}

/** The server pings after 15 s idle; this long without any frame means the socket is dead. */
export const SILENCE_LIMIT_MS = 30_000;

/** A connection that lasted this long resets the reconnect backoff. */
export const STABLE_AFTER_MS = 60_000;

/** Reconnect delays: exponential from 250 ms, factor 2, jittered, capped at 30 s. */
export const reconnectSchedule = Schedule.min([
  Schedule.exponential("250 millis", 2).pipe(Schedule.jittered),
  Schedule.spaced("30 seconds"),
]);

/**
 * Events covered by fresh sidebar and activity snapshots. After a reload, replay starts at the
 * stored cursor, so replaying covered events would change the freshly loaded state.
 */
const SNAPSHOT_EVENTS: ReadonlySet<SyncEvent["type"]> = new Set([
  "room.unread",
  "room.read",
  "sidebar.row.upserted",
  "sidebar.row.removed",
  "activity.item",
  "activity.removed",
]);

/** Frames this close together land in one store commit. */
const COALESCE_WINDOW = "16 millis";

const COALESCE_MAX = 256;

const setConnection = (status: ConnectionStatus) =>
  Effect.sync(() => mutations.setConnection(status));

/** `12` for `room:12`, else `null`. */
function roomIdOf(topic: string): number | null {
  const match = /^room:(\d+)$/.exec(topic);

  return match === null ? null : Number(match[1]);
}

/** `88` for `thread:88`, else `null`. */
function threadIdOf(topic: string): number | null {
  const match = /^thread:(\d+)$/.exec(topic);

  return match === null ? null : Number(match[1]);
}

/**
 * The loaded sidebar gained a room (joined, added, or access regained). The server sends no
 * `activity.item` for what the viewer can see again there, so the inbox has to be read afresh.
 */
export function roomsAdded(previous: SidebarState, next: SidebarState): boolean {
  if (previous.status !== "ready" || next.rows === previous.rows) {
    return false;
  }

  return Object.keys(next.rows).some((roomId) => !Object.hasOwn(previous.rows, roomId));
}

/** Authors of new messages the store has no user record for. */
function unknownAuthors(events: readonly SyncEvent[]): readonly number[] {
  const known = store.getState().users;
  const missing = new Set<number>();

  for (const event of events) {
    if (
      (event.type === "message.created" || event.type === "thread.created") &&
      known[event.data.creatorId] === undefined
    ) {
      missing.add(event.data.creatorId);
    }

    if (event.type === "activity.item") {
      const creatorId = event.data.item.source.creatorId;

      if (creatorId !== null && known[creatorId] === undefined) {
        missing.add(creatorId);
      }
    }

    if (event.type === "sidebar.row.upserted") {
      for (const userId of event.data.directMemberIds) {
        if (known[userId] === undefined) {
          missing.add(userId);
        }
      }
    }
  }

  return [...missing];
}

function landRefreshedRoom(
  detail: RoomDetail,
  revision: number,
  started: number,
): "applied" | "skipped" | "rejected" {
  const state = store.getState();
  const roomId = detail.room.id;
  const view = state.rooms[roomId];

  if (roomRevision(roomId) !== revision || view == null) {
    return "skipped";
  }

  // Unavailable, or a join preview, has no detail this refresh may replace. A first load does.
  if (view.detail == null && view.status !== "loading") {
    return "skipped";
  }

  // A read/unread or placement row may have landed while this request was pending.
  const row = state.sidebar.rows[roomId];

  const landed = mutations.setRoomDetail(
    row === undefined
      ? detail
      : {
          ...detail,
          room: row.room,
          membership: row.membership,
          displayName: row.displayName,
          directMemberIds: row.directMemberIds,
        },
    started,
  );

  return landed ? "applied" : "rejected";
}

/**
 * The sync engine's main fiber: connect, `hello` (resume point and topics), then apply what the
 * server sends, reconnecting with backoff whenever the socket drops, goes silent, or the server
 * says `bye{reconnect:true}`. Also drops each typing entry and tombstone the moment it lapses
 * and runs the presence heartbeat. Everything reads time from Effect's `Clock`.
 */
export class Engine extends Context.Service<
  Engine,
  {
    /** Runs until `bye{reconnect:false}` or interruption. */
    readonly run: Effect.Effect<void>;
  }
>()("smartfire/sync/Engine") {
  static readonly layer = Layer.effect(
    Engine,
    Effect.gen(function* () {
      const socket = yield* SyncSocket;
      const link = yield* SyncLink;
      const cursor = yield* Cursor;
      const topics = yield* Topics;
      const presence = yield* Presence;
      const lifecycle = yield* Lifecycle;
      const api = yield* Effect.context<ApiClient>();
      /** The cursor came from storage (a reload) and no `welcome` has been handled yet. */
      const restored = yield* Ref.make((yield* cursor.get) !== null);
      /** Snapshot events through this sequence are covered by the initial refetch. */
      const snapshotThrough = yield* Ref.make(Number.NEGATIVE_INFINITY);

      const activitySnapshotThrough = yield* Ref.make({
        generation: store.getState().activity.generation,
        through: Number.NEGATIVE_INFINITY,
      });

      /**
       * A refresh lost to a membership fact. A later read, at the room's current revision and a
       * new sequence, replaces it. Losses while that re-read is in flight share one follow-up.
       * The fact itself is not read again.
       */
      let retryRejectedRefresh: (roomId: number) => Effect.Effect<void> = () => Effect.void;

      /** Refetch room metadata at its current management revision, including lost access. */
      const resyncRoomDetail = Effect.fnUntraced(function* (roomId: number, revision: number) {
        const started = beginRoomRequest();
        const detail = yield* Effect.result(room(roomId));

        if (Result.isSuccess(detail) && roomRevision(roomId) === revision) {
          markRoomsChanged([roomId]);

          if (landRefreshedRoom(detail.success, revision, started) === "rejected") {
            yield* retryRejectedRefresh(roomId);
          }
        } else if (
          Result.isFailure(detail) &&
          roomRevision(roomId) === revision &&
          Predicate.isTagged(detail.failure, "NotFound")
        ) {
          // A previewed room is not a membership: this 404 is expected. Reload the preview
          // instead of treating the reconnect as lost access.
          let unavailableAt = started;

          if (store.getState().rooms[roomId]?.preview != null) {
            unavailableAt = beginRoomRequest();
            const preview = yield* Effect.result(openRoomPreview(roomId));

            if (roomRevision(roomId) !== revision) {
              return "preview";
            }

            if (Result.isSuccess(preview)) {
              // A join can install the membership while this refetch is in flight.
              if (store.getState().rooms[roomId]?.detail == null) {
                mutations.setRoomPreview(roomId, preview.success, unavailableAt);
              }

              return "preview";
            }

            if (!Predicate.isTagged(preview.failure, "NotFound")) {
              return "preview";
            }
          }

          markRoomsChanged([roomId]);
          mutations.setRoomUnavailable(roomId, unavailableAt);

          return false;
        }

        return true;
      });

      /**
       * A room's newest page, merged into the window the reader is on (`resync`); then, for a
       * window away from the present, the page around its middle re-read in place.
       */
      const resyncRoom = Effect.fnUntraced(function* (roomId: number) {
        const state = store.getState();
        const held = state.boards[roomId];
        const revision = roomRevision(roomId);

        const kind =
          state.rooms[roomId]?.detail?.room.kind ?? state.sidebar.rows[roomId]?.room.kind;

        if (kind === "board") {
          // A missed `board.automations.changed` can't be replayed either.
          mutations.boardAutomationsChanged(roomId);

          if (held !== undefined) mutations.setBoardLoading(roomId, held.query);
          const generation = store.getState().boards[roomId]?.generation;

          const read = captureWorkRead(store.getState());

          const [available, listing] = yield* Effect.all(
            [
              resyncRoomDetail(roomId, revision),
              held === undefined
                ? Effect.succeed(null)
                : Effect.result(board(roomId, { ...held.query, page: held.page })),
            ],
            { concurrency: 2 },
          );

          if (available !== true) {
            if (available === false && generation !== undefined) {
              mutations.setBoardError(roomId, generation, "This room is no longer available.");
            }

            return;
          }

          if (listing !== null && generation !== undefined) {
            if (Result.isSuccess(listing)) {
              mutations.loadBoardListing(listing.success, generation, read);
            } else {
              mutations.setBoardError(roomId, generation, listing.failure.message);
              yield* Effect.logWarning("sync: board resync failed", listing.failure.message);
            }
          }

          return;
        }

        mutations.setPageReplacing(roomId);

        const [available, newest] = yield* Effect.all(
          [resyncRoomDetail(roomId, revision), Effect.result(messages(roomId, null))],
          { concurrency: 2 },
        );

        if (available !== true) {
          if (available === false) {
            mutations.setPageFailed(roomId);
          }

          return;
        }

        if (Result.isFailure(newest)) {
          return yield* Effect.fail(newest.failure);
        }

        mutations.applyPage(roomId, newest.success, "resync");

        const anchor = middleOf(store.getState().timelines[roomId]);

        if (anchor === null) {
          return;
        }

        const page = yield* messages(roomId, { around: anchor });

        if (readingHistory(store.getState().timelines[roomId])) {
          mutations.applyPage(roomId, page, "refresh");
        }
      });

      /** As `resyncRoom` for a thread's replies, plus its header: status, permissions, membership. */
      const resyncThread = Effect.fnUntraced(function* (threadId: number) {
        // A permalink's load this supersedes still wants its reply: open around it instead.
        const focus = pendingThreadFocus(threadId);
        const load = beginThreadLoad(threadId, focus);

        mutations.setThreadPageReplacing(threadId);

        const [detail, newest] = yield* Effect.all(
          [
            loadThreadHeader(threadId, load),
            Effect.result(threadMessages(threadId, focus === null ? null : { around: focus })),
          ],
          { concurrency: 2 },
        );

        // A newer load (the pane's Try again, or another resync) decides what the pane shows.
        if (!isLatestThreadLoad(threadId, load)) {
          return;
        }

        const problem = paneProblem(detail);
        const pane = store.getState().threadPanes[threadId];

        // A pane already showing the thread keeps it through a failed refresh; one still loading
        // (whose own load this superseded) or failed says why, with Try again.
        if (
          problem !== null &&
          pane !== undefined &&
          (problem === UNAVAILABLE || pane.status !== "ready")
        ) {
          mutations.setThreadPaneError(threadId, problem);
        }

        if (Result.isFailure(newest)) {
          // The permalink's reply may be gone while the thread is still there (see `openAtNewest`).
          if (focus !== null && problem === null && isGoneFocus(newest.failure)) {
            return yield* openAtNewest(threadId, load);
          }

          return yield* Effect.fail(newest.failure);
        }

        // Only once its page is in does a permalink's focus stop carrying over (see `loadPane`).
        if (focus !== null) {
          mutations.applyThreadPage(threadId, newest.success, "replace");
          finishThreadLoad(threadId, load);

          return;
        }

        mutations.applyThreadPage(threadId, newest.success, "resync");

        const anchor = middleOf(store.getState().threadTimelines[threadId]);

        if (anchor === null) {
          return;
        }

        const page = yield* threadMessages(threadId, { around: anchor });

        if (
          isLatestThreadLoad(threadId, load) &&
          readingHistory(store.getState().threadTimelines[threadId])
        ) {
          mutations.applyThreadPage(threadId, page, "refresh");
        }
      });

      /** The badge from the server; a failure keeps the count shown. */
      const refreshUnreadCount = (through?: number) => {
        const generation = store.getState().activity.generation;

        return loadUnreadCount(generation).pipe(
          Effect.timeout(ACTIVITY_REQUEST_TIMEOUT),
          Effect.tap(() =>
            through === undefined
              ? Effect.void
              : Ref.update(activitySnapshotThrough, (held) =>
                  held.generation === generation &&
                  store.getState().activity.generation === generation
                    ? { generation, through: Math.max(held.through, through) }
                    : held,
                ),
          ),
          Effect.catch((error) =>
            Effect.logWarning("sync: activity count refresh failed", error.message),
          ),
          Effect.provideContext(api),
        );
      };

      /** REST refetch for topics the server can't replay: the sidebar, a room or a thread. */
      const resync = Effect.fnUntraced(function* (
        topicList: readonly string[],
        activityThrough?: number,
      ) {
        // What the store doesn't hold (an open calendar's events) reads itself again meanwhile.
        emitResync(topicList);

        for (const topic of topicList) {
          const roomId = roomIdOf(topic);
          const threadId = threadIdOf(topic);

          if (topic === "user") {
            const since = sidebarRowClock();

            yield* sidebar().pipe(
              Effect.tap((data) =>
                Effect.sync(() => {
                  mutations.resyncSidebar(data, since);
                  // The snapshot is newer than any room write still on its way.
                  markSidebarSnapshot();
                }),
              ),
              Effect.catch((error) =>
                Effect.logWarning("sync: sidebar resync failed", error.message),
              ),
              Effect.provideContext(api),
            );
            // The inbox, saved and scheduled lists can't be replayed either: they reload when
            // next shown, and the badge refreshes now.
            mutations.markInboxStale();
            yield* Effect.forkChild(refreshUnreadCount(activityThrough));
          } else if (roomId !== null) {
            yield* resyncRoom(roomId).pipe(
              Effect.catch((error) =>
                Effect.sync(() => mutations.setPageFailed(roomId)).pipe(
                  Effect.andThen(Effect.logWarning(`sync: ${topic} resync failed`, error.message)),
                ),
              ),
              Effect.provideContext(api),
            );
          } else if (threadId !== null) {
            yield* resyncThread(threadId).pipe(
              Effect.catch((error) =>
                Effect.sync(() => mutations.setThreadPageFailed(threadId)).pipe(
                  Effect.andThen(Effect.logWarning(`sync: ${topic} resync failed`, error.message)),
                ),
              ),
              Effect.provideContext(api),
            );
          }
        }
      });

      const fetchUsers = (ids: readonly number[]) =>
        users(ids).pipe(
          Effect.tap((list) => Effect.sync(() => mutations.mergeUsers(list.users))),
          Effect.catch((error) => Effect.logWarning("sync: author lookup failed", error.message)),
          Effect.provideContext(api),
        );

      const refreshRoom = (roomId: number, revision: number, allow: () => boolean = () => true) =>
        Effect.suspend(() => {
          if (!allow()) return Effect.void;

          const started = beginRoomRequest();

          return room(roomId).pipe(
            Effect.tap((detail) =>
              Effect.gen(function* () {
                if (!allow()) return;

                if (landRefreshedRoom(detail, revision, started) === "rejected") {
                  yield* retryRejectedRefresh(roomId);
                }
              }),
            ),
            Effect.catch((error) => {
              if (!allow()) return Effect.void;

              return Effect.sync(() => {
                if (roomRevision(roomId) === revision && Predicate.isTagged(error, "NotFound")) {
                  mutations.setRoomUnavailable(roomId, started);
                }
              }).pipe(
                Effect.andThen(
                  Effect.logWarning("sync: room metadata refresh failed", error.message),
                ),
              );
            }),
          );
        }).pipe(Effect.provideContext(api));

      retryRejectedRefresh = Effect.fnUntraced(function* (roomId: number) {
        const visit = roomVisitToken(roomId);
        const owns = () => roomVisitToken(roomId) === visit;

        yield* recoverRejectedRoomRead(roomId, visit, owns, () =>
          refreshRoom(roomId, roomRevision(roomId), owns),
        );
      });

      /** Applies batch events past the cursor in one store commit, then advances the cursor. */
      const applyEvents = Effect.fnUntraced(function* (
        events: readonly SyncEvent[],
        scope: Scope.Scope,
      ) {
        const point = yield* cursor.get;
        const covered = yield* Ref.get(snapshotThrough);
        const activitySnapshot = yield* Ref.get(activitySnapshotThrough);

        const activityCovered =
          activitySnapshot.generation === store.getState().activity.generation
            ? activitySnapshot.through
            : Number.NEGATIVE_INFINITY;

        const fresh: SyncEvent[] = [];
        const start = point?.seq ?? Number.NEGATIVE_INFINITY;
        let seq = start;

        for (const event of events) {
          if (event.seq > seq) {
            seq = event.seq;

            const through =
              event.type === "activity.item" || event.type === "activity.removed"
                ? activityCovered
                : covered;

            if (!(event.seq <= through && SNAPSHOT_EVENTS.has(event.type))) {
              fresh.push(event);
            }
          }
        }

        if (seq === start) {
          return;
        }

        if (fresh.length === 0) {
          yield* cursor.set({ epoch: point?.epoch ?? "", seq });

          return;
        }

        const now = yield* Clock.currentTimeMillis;

        mutations.applyEvents(fresh, now);
        emitSyncEvents(fresh);
        const open = new Set(yield* topics.subscribed);

        const workIds = new Set(
          fresh.flatMap((event) =>
            event.type === "thread.updated" &&
            open.has(`thread:${event.data.id}`) &&
            workDetailStale(store.getState(), event.data.id)
              ? [event.data.id]
              : [],
          ),
        );

        for (const threadId of workIds) {
          yield* Effect.forkIn(refreshWork(threadId).pipe(Effect.provideContext(api)), scope);
        }

        const refreshes = roomRefreshIds(fresh)
          .map((roomId) => ({
            roomId,
            revision: invalidateRoom(roomId),
          }))
          .filter(({ roomId }) => {
            const view = store.getState().rooms[roomId];

            // A revocation can arrive before the first GET installs detail. Refresh anyway.
            return view?.detail != null || view?.status === "loading";
          });

        if (refreshes.length > 0) {
          yield* Effect.forkIn(
            Effect.forEach(refreshes, ({ roomId, revision }) => refreshRoom(roomId, revision), {
              concurrency: 4,
            }),
            scope,
          );
        }

        yield* cursor.set({ epoch: point?.epoch ?? "", seq });

        const missing = unknownAuthors(fresh);

        if (missing.length > 0) {
          yield* Effect.forkIn(fetchUsers(missing), scope);
        }
      });

      const welcome = Effect.fnUntraced(function* (frame: Extract<ServerFrame, { t: "welcome" }>) {
        const point = yield* cursor.get;
        const afterReload = yield* Ref.getAndSet(restored, false);

        const newEpoch = point?.epoch !== frame.epoch;

        // The epoch fence relies on restores restarting the server (docs/backups.md, "Restore onto the VM").
        mutations.beginActivityGeneration(newEpoch);
        yield* Ref.set(activitySnapshotThrough, {
          generation: store.getState().activity.generation,
          through: Number.NEGATIVE_INFINITY,
        });

        if (frame.resumed && point !== null) {
          yield* cursor.set({ epoch: frame.epoch, seq: point.seq });

          // The stored cursor predates the page reload. Refetch after every replayed event
          // happened, then skip sidebar and activity events the fresh snapshots cover.
          if (afterReload && frame.seq > point.seq) {
            yield* Ref.set(snapshotThrough, frame.seq);
            yield* resync(["user"], frame.seq);
          } else {
            yield* Effect.forkChild(refreshUnreadCount(frame.seq));
          }

          return;
        }

        yield* cursor.set({ epoch: frame.epoch, seq: frame.seq });
        yield* resync(["user", ...(yield* topics.subscribed)], frame.seq);
      });

      /**
       * Handles one coalesced group of frames, in order: batch events gather into one commit,
       * flushed before any other frame. Returns `bye`'s `reconnect`, or `null` to keep going.
       */
      const handleFrames = Effect.fnUntraced(function* (
        frames: readonly ServerFrame[],
        scope: Scope.Scope,
      ) {
        let events: SyncEvent[] = [];

        const flush = Effect.suspend(() => {
          const batch = events;

          events = [];

          return applyEvents(batch, scope);
        });

        for (const frame of frames) {
          switch (frame.t) {
            case "batch":
              events.push(...frame.events);
              break;
            case "ping":
              break;
            case "welcome":
              yield* flush;
              yield* welcome(frame);
              break;
            case "resync":
              yield* flush;
              yield* resync(frame.topics);
              break;
            case "bye":
              yield* flush;

              return frame.reconnect;
          }
        }

        yield* flush;

        return null;
      });

      const greeting = Effect.gen(function* () {
        const hello: ClientFrame = {
          t: "hello",
          v: 1,
          resume: yield* cursor.get,
          topics: [...(yield* topics.subscribed)],
        };

        return [hello, ...(yield* presence.greeting)];
      });

      /**
       * One connection, start to end. Records when it opened in `openedAt`; returns whether to
       * reconnect (`false` only after `bye{reconnect:false}`).
       */
      const session = (openedAt: Ref.Ref<number | null>, scope: Scope.Scope) =>
        Effect.scoped(
          Effect.gen(function* () {
            const connection = yield* socket.connect;

            // Detach before the socket's own finalizers run.
            yield* Effect.addFinalizer(() => link.detach);
            yield* link.attach(connection, greeting);
            yield* Ref.set(openedAt, yield* Clock.currentTimeMillis);
            yield* setConnection("online");

            const bye = yield* Ref.make<boolean | null>(null);

            yield* connection.incoming.pipe(
              Stream.timeoutOrElse({
                duration: SILENCE_LIMIT_MS,
                orElse: () => Stream.fail(new SyncSocketError({ reason: "silent" })),
              }),
              Stream.groupedWithin(COALESCE_MAX, COALESCE_WINDOW),
              Stream.runForEachWhile((frames) =>
                handleFrames(frames, scope).pipe(
                  Effect.tap((reconnect) => Ref.set(bye, reconnect)),
                  Effect.map((reconnect) => reconnect === null),
                ),
              ),
            );

            return (yield* Ref.get(bye)) ?? true;
          }),
        ).pipe(
          Effect.catchTag("SyncSocketError", (error) =>
            Effect.as(Effect.logDebug("sync: connection ended", error.reason), true),
          ),
        );

      const connectLoop = Effect.fnUntraced(function* (scope: Scope.Scope) {
        let step = yield* Schedule.toStep(reconnectSchedule);

        yield* setConnection("connecting");

        while (true) {
          const openedAt = yield* Ref.make<number | null>(null);
          const reconnect = yield* session(openedAt, scope);

          if (!reconnect) {
            yield* setConnection("offline");

            return;
          }

          // Shown while waiting out the backoff, not only once the next dial starts.
          yield* setConnection("reconnecting");

          const now = yield* Clock.currentTimeMillis;
          const opened = yield* Ref.get(openedAt);

          if (opened !== null && now - opened >= STABLE_AFTER_MS) {
            step = yield* Schedule.toStep(reconnectSchedule);
          }

          const [, delay] = yield* Effect.orDie(step(now, undefined));

          // Back online or shown again: skip the rest of the wait.
          yield* Effect.raceFirst(
            Effect.sleep(delay),
            Effect.asVoid(Stream.runHead(lifecycle.wakeups)),
          );
        }
      });

      /**
       * Drops typing entries and tombstones exactly when they lapse: sleeps until the soonest
       * expiry, waking early whenever either map changes (a new typist may lapse sooner).
       */
      const expire = Effect.gen(function* () {
        const changed = yield* Queue.sliding<void>(1);

        yield* Effect.acquireRelease(
          Effect.sync(() =>
            store.subscribe((state, previous) => {
              if (state.typing !== previous.typing || state.tombstones !== previous.tombstones) {
                Queue.offerUnsafe(changed, undefined);
              }
            }),
          ),
          (unsubscribe) => Effect.sync(unsubscribe),
        );

        while (true) {
          const now = yield* Clock.currentTimeMillis;

          mutations.prune(now);
          // Prune's own change is already applied; only later changes should wake the wait.
          yield* Queue.clear(changed);

          const soonest = nextExpiry(store.getState());
          const wake = Queue.take(changed);

          yield* soonest === null
            ? wake
            : Effect.raceFirst(Effect.sleep(Math.max(0, soonest - now)), wake);
        }
      });

      /**
       * Whenever the sidebar gains a room, every activity list reloads when next shown (at once
       * if shown now) and the badge refreshes. Rooms landing close together refresh once.
       */
      const followRooms = Effect.gen(function* () {
        const grew = yield* Queue.sliding<void>(1);

        yield* Effect.acquireRelease(
          Effect.sync(() =>
            store.subscribe((state, previous) => {
              if (roomsAdded(previous.sidebar, state.sidebar)) {
                Queue.offerUnsafe(grew, undefined);
              }
            }),
          ),
          (unsubscribe) => Effect.sync(unsubscribe),
        );

        while (true) {
          yield* Queue.take(grew);
          mutations.markActivityStale();
          yield* refreshUnreadCount();
        }
      });

      const run = Effect.scoped(
        Effect.gen(function* () {
          const scope = yield* Effect.scope;

          yield* Effect.forkIn(Effect.scoped(expire), scope);
          yield* Effect.forkIn(Effect.scoped(followRooms), scope);
          yield* Effect.forkIn(presence.run, scope);
          yield* connectLoop(scope);
        }),
      );

      return Engine.of({ run });
    }),
  );
}
