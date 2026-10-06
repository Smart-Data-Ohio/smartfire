import { Clock, Context, Duration, Effect, Layer, Ref, Schedule, type Scope, Stream } from "effect";
import type { ApiClient } from "../api/client.ts";
import { messages, sidebar, users } from "../api/endpoints.ts";
import type { ClientFrame } from "../gen/ClientFrame.ts";
import type { ServerFrame } from "../gen/ServerFrame.ts";
import type { SyncEvent } from "../gen/SyncEvent.ts";
import type { ConnectionStatus } from "../store/model.ts";
import { mutations, store } from "../store/store.ts";
import { Cursor } from "./cursor.ts";
import { Lifecycle } from "./lifecycle.ts";
import { SyncLink } from "./link.ts";
import { Presence } from "./presence.ts";
import { SyncSocket, SyncSocketError } from "./socket.ts";
import { Topics } from "./topics.ts";

/** The server pings after 15 s idle; this long without any frame means the socket is dead. */
export const SILENCE_LIMIT_MS = 30_000;

/** A connection that lasted this long resets the reconnect backoff. */
export const STABLE_AFTER_MS = 60_000;

/** How often typing entries and tombstones are swept. */
export const PRUNE_EVERY_MS = 1000;

/** Reconnect delays: exponential from 250 ms, factor 2, jittered, capped at 30 s. */
export const reconnectSchedule = Schedule.min([
  Schedule.exponential("250 millis", 2).pipe(Schedule.jittered),
  Schedule.spaced("30 seconds"),
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

/** Authors of new messages the store has no user record for. */
function unknownAuthors(events: readonly SyncEvent[]): readonly number[] {
  const known = store.getState().users;
  const missing = new Set<number>();

  for (const event of events) {
    if (event.type === "message.created" && known[event.data.creatorId] === undefined) {
      missing.add(event.data.creatorId);
    }
  }

  return [...missing];
}

/**
 * The sync engine's main fiber: connect, `hello` (resume point and topics), then apply what the
 * server sends, reconnecting with backoff whenever the socket drops, goes silent, or the server
 * says `bye{reconnect:true}`. Also sweeps the store's typing entries and tombstones every second
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

      /** REST refetch for topics the server can't replay: the newest page, or the sidebar. */
      const resync = Effect.fnUntraced(function* (topicList: readonly string[]) {
        for (const topic of topicList) {
          const roomId = roomIdOf(topic);

          if (topic === "user") {
            yield* sidebar().pipe(
              Effect.tap((data) => Effect.sync(() => mutations.loadSidebar(data))),
              Effect.catch((error) =>
                Effect.logWarning("sync: sidebar resync failed", error.message),
              ),
              Effect.provideContext(api),
            );
          } else if (roomId !== null) {
            yield* messages(roomId, null).pipe(
              Effect.tap((page) => Effect.sync(() => mutations.applyPage(roomId, page, "replace"))),
              Effect.catch((error) =>
                Effect.logWarning(`sync: ${topic} resync failed`, error.message),
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

      /** Applies batch events past the cursor in one store commit, then advances the cursor. */
      const applyEvents = Effect.fnUntraced(function* (
        events: readonly SyncEvent[],
        scope: Scope.Scope,
      ) {
        const point = yield* cursor.get;
        const fresh: SyncEvent[] = [];
        let seq = point?.seq ?? Number.NEGATIVE_INFINITY;

        for (const event of events) {
          if (event.seq > seq) {
            fresh.push(event);
            seq = event.seq;
          }
        }

        if (fresh.length === 0) {
          return;
        }

        const now = yield* Clock.currentTimeMillis;

        mutations.applyEvents(fresh, now);

        yield* cursor.set({ epoch: point?.epoch ?? "", seq });

        const missing = unknownAuthors(fresh);

        if (missing.length > 0) {
          yield* Effect.forkIn(fetchUsers(missing), scope);
        }
      });

      const welcome = Effect.fnUntraced(function* (frame: Extract<ServerFrame, { t: "welcome" }>) {
        const point = yield* cursor.get;

        if (frame.resumed && point !== null) {
          yield* cursor.set({ epoch: frame.epoch, seq: point.seq });

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

      const prune = Clock.currentTimeMillis.pipe(
        Effect.tap((now) => Effect.sync(() => mutations.prune(now))),
        Effect.repeat(Schedule.spaced(Duration.millis(PRUNE_EVERY_MS))),
      );

      const run = Effect.scoped(
        Effect.gen(function* () {
          const scope = yield* Effect.scope;

          yield* Effect.forkIn(prune, scope);
          yield* Effect.forkIn(presence.run, scope);
          yield* connectLoop(scope);
        }),
      );

      return Engine.of({ run });
    }),
  );
}
