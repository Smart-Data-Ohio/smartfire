import {
  Clock,
  Context,
  Effect,
  Layer,
  Queue,
  Ref,
  Result,
  Schedule,
  type Scope,
  Stream,
} from "effect";
import { activityUnreadCount } from "../api/activity-endpoints.ts";
import { board } from "../api/board-endpoints.ts";
import type { ApiClient } from "../api/client.ts";
import { messages, sidebar, users } from "../api/endpoints.ts";
import { thread, threadMessages } from "../api/thread-endpoints.ts";
import type { ClientFrame } from "../gen/ClientFrame.ts";
import type { ServerFrame } from "../gen/ServerFrame.ts";
import type { SyncEvent } from "../gen/SyncEvent.ts";
import type { ConnectionStatus, Timeline } from "../store/model.ts";
import { nextExpiry } from "../store/reducers.ts";
import type { SidebarState } from "../store/state.ts";
import { mutations, store } from "../store/store.ts";
import { THREAD_DELETED } from "../store/threads.ts";
import { Cursor } from "./cursor.ts";
import { Lifecycle } from "./lifecycle.ts";
import { SyncLink } from "./link.ts";
import { Presence } from "./presence.ts";
import { refetchThread, settled } from "./settle.ts";
import { emitSyncEvents } from "./signals.ts";
import { SyncSocket, SyncSocketError } from "./socket.ts";
import { Topics } from "./topics.ts";
import { changedWorkPanes, refreshWorkPane } from "./work-refresh.ts";

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
 * Events whose effect a fresh sidebar snapshot already includes. After a reload they are replayed
 * from the stored cursor on top of a sidebar fetched since, so applying them would count twice.
 */
const SNAPSHOT_EVENTS: ReadonlySet<SyncEvent["type"]> = new Set([
  "room.unread",
  "room.read",
  "sidebar.row.upserted",
  "sidebar.row.removed",
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
  }

  return [...missing];
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
      /** Replayed sidebar events up to this seq are already in the refetched sidebar. */
      const snapshotThrough = yield* Ref.make(Number.NEGATIVE_INFINITY);

      /**
       * A room's newest page, merged into the window the reader is on (`resync`); then, for a
       * window away from the present, the page around its middle re-read in place.
       */
      const resyncRoom = Effect.fnUntraced(function* (roomId: number) {
        const state = store.getState();
        const held = state.boards[roomId];

        const kind =
          state.rooms[roomId]?.detail?.room.kind ?? state.sidebar.rows[roomId]?.room.kind;

        if (kind === "board") {
          if (held !== undefined) {
            mutations.setBoardLoading(roomId, held.query);
            const generation = store.getState().boards[roomId]?.generation;
            yield* board(roomId, { ...held.query, page: held.page }).pipe(
              Effect.tap((listing) =>
                Effect.sync(() => {
                  if (generation !== undefined) mutations.loadBoardListing(listing, generation);
                }),
              ),
              Effect.catch((error) =>
                Effect.sync(() => {
                  if (generation !== undefined)
                    mutations.setBoardError(roomId, generation, error.message);
                }).pipe(
                  Effect.andThen(Effect.logWarning("sync: board resync failed", error.message)),
                ),
              ),
            );
          }

          return;
        }

        mutations.setPageReplacing(roomId);

        const newest = yield* messages(roomId, null);

        mutations.applyPage(roomId, newest, "resync");

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
        mutations.setThreadPageReplacing(threadId);

        const [detail, newest] = yield* Effect.all(
          [
            Effect.result(
              // Installed as soon as it's settled, not after the replies (see `loadPane`).
              settled(
                thread(threadId),
                () => refetchThread(threadId),
                (answer) => [answer.thread.id],
                (answer, since) => mutations.loadThreadDetail(answer, since),
              ),
            ),
            threadMessages(threadId, null),
          ],
          { concurrency: 2 },
        );

        if (Result.isSuccess(detail) && detail.success.outcome === "gone") {
          mutations.setThreadPaneError(threadId, THREAD_DELETED);
        }

        mutations.applyThreadPage(threadId, newest, "resync");

        const anchor = middleOf(store.getState().threadTimelines[threadId]);

        if (anchor === null) {
          return;
        }

        const page = yield* threadMessages(threadId, { around: anchor });

        if (readingHistory(store.getState().threadTimelines[threadId])) {
          mutations.applyThreadPage(threadId, page, "refresh");
        }
      });

      /** The badge from the server; a failure keeps the count shown. */
      const refreshUnreadCount = activityUnreadCount().pipe(
        Effect.tap(({ unreadCount }) =>
          Effect.sync(() => mutations.setActivityUnreadCount(unreadCount)),
        ),
        Effect.catch((error) =>
          Effect.logWarning("sync: activity count refresh failed", error.message),
        ),
        Effect.provideContext(api),
      );

      /** REST refetch for topics the server can't replay: the sidebar, a room or a thread. */
      const resync = Effect.fnUntraced(function* (topicList: readonly string[]) {
        for (const topic of topicList) {
          const roomId = roomIdOf(topic);
          const threadId = threadIdOf(topic);

          if (topic === "user") {
            yield* sidebar().pipe(
              Effect.tap((data) => Effect.sync(() => mutations.loadSidebar(data))),
              Effect.catch((error) =>
                Effect.logWarning("sync: sidebar resync failed", error.message),
              ),
              Effect.provideContext(api),
            );
            // The inbox, saved and scheduled lists can't be replayed either: they reload when
            // next shown, and the badge refreshes now.
            mutations.markInboxStale();
            yield* refreshUnreadCount;
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

      const refreshingWork = new Set<number>();

      /** Applies batch events past the cursor in one store commit, then advances the cursor. */
      const applyEvents = Effect.fnUntraced(function* (
        events: readonly SyncEvent[],
        scope: Scope.Scope,
      ) {
        const point = yield* cursor.get;
        const covered = yield* Ref.get(snapshotThrough);
        const fresh: SyncEvent[] = [];
        const start = point?.seq ?? Number.NEGATIVE_INFINITY;
        let seq = start;

        for (const event of events) {
          if (event.seq > seq) {
            seq = event.seq;

            if (!(event.seq <= covered && SNAPSHOT_EVENTS.has(event.type))) {
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
        const workIds = changedWorkPanes(store.getState(), fresh, yield* topics.subscribed);

        for (const threadId of workIds) {
          if (refreshingWork.has(threadId)) continue;
          refreshingWork.add(threadId);
          yield* Effect.forkIn(
            refreshWorkPane(threadId).pipe(
              Effect.catch((error) =>
                Effect.logWarning("sync: work refresh failed", error.message),
              ),
              Effect.ensuring(Effect.sync(() => refreshingWork.delete(threadId))),
              Effect.provideContext(api),
            ),
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

        if (frame.resumed && point !== null) {
          yield* cursor.set({ epoch: frame.epoch, seq: point.seq });

          // After a reload the sidebar was fetched before this socket, while the replay starts
          // at the stored cursor: some replayed unreads are already counted, others (those since
          // the fetch) aren't. Refetch now, after every replayed event happened, and skip the
          // replayed sidebar events the new snapshot covers.
          if (afterReload && frame.seq > point.seq) {
            yield* Ref.set(snapshotThrough, frame.seq);
            yield* resync(["user"]);
          }

          return;
        }

        yield* cursor.set({ epoch: frame.epoch, seq: frame.seq });
        yield* resync(["user", ...(yield* topics.subscribed)]);
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
          yield* refreshUnreadCount;
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
