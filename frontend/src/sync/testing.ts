/**
 * In-memory stand-ins for the sync engine's platform pieces, driven by the test: a socket whose
 * server side the test plays, and browser lifecycle events the test fires. Only tests import this.
 */
import { Clock, Context, Effect, Layer, PubSub, Queue, Ref, Stream } from "effect";
import type { ClientFrame } from "../gen/ClientFrame.ts";
import type { ServerFrame } from "../gen/ServerFrame.ts";
import { Lifecycle } from "./lifecycle.ts";
import { decodeFrames } from "./protocol.ts";
import { type SyncConnection, SyncSocket, SyncSocketError } from "./socket.ts";

interface SocketState {
  /** The open connection's server-to-client queue, if one is open. */
  readonly open: Queue.Queue<string, SyncSocketError> | null;
  readonly reachable: boolean;
}

/**
 * The server side of an in-memory `SyncSocket`. Each dial is recorded in `dials` (Clock time)
 * and fails while the server is unreachable; otherwise it opens a connection the test feeds with
 * `push`/`pushText` and ends with `drop`. Frames from the client collect in `sent`.
 */
export class MemorySocket extends Context.Service<
  MemorySocket,
  {
    /** Sends a frame to the client on the open connection (lost if none is open). */
    readonly push: (frame: ServerFrame) => Effect.Effect<void>;
    /** Sends raw text, to exercise decoding (unknown event types, garbage). */
    readonly pushText: (text: string) => Effect.Effect<void>;
    /** The server closes the open connection. */
    readonly drop: Effect.Effect<void>;
    /** While unreachable, every dial fails. */
    readonly setReachable: (reachable: boolean) => Effect.Effect<void>;
    readonly isOpen: Effect.Effect<boolean>;
    /** Clock time (ms) of every dial, successful or not. */
    readonly dials: Effect.Effect<readonly number[]>;
    /** Every frame the client sent, oldest first, across connections. */
    readonly sent: Effect.Effect<readonly ClientFrame[]>;
    readonly clearSent: Effect.Effect<void>;
  }
>()("smartfire/sync/MemorySocket") {
  /** `MemorySocket` plus the `SyncSocket` it serves. */
  static readonly layer = Layer.effectContext(
    Effect.gen(function* () {
      const state = yield* Ref.make<SocketState>({ open: null, reachable: true });
      const dials = yield* Ref.make<readonly number[]>([]);
      const sent = yield* Ref.make<readonly ClientFrame[]>([]);

      const pushText = (text: string) =>
        Effect.flatMap(Ref.get(state), ({ open }) =>
          open === null ? Effect.void : Effect.asVoid(Queue.offer(open, text)),
        );

      const drop = Effect.flatMap(
        Ref.modify(state, (current) => [current.open, { ...current, open: null }] as const),
        (open) =>
          open === null
            ? Effect.void
            : Effect.asVoid(Queue.fail(open, new SyncSocketError({ reason: "dropped" }))),
      );

      const connect = Effect.gen(function* () {
        const now = yield* Clock.currentTimeMillis;

        yield* Ref.update(dials, (list) => [...list, now]);

        const { reachable } = yield* Ref.get(state);

        if (!reachable) {
          return yield* new SyncSocketError({ reason: "unreachable" });
        }

        const queue = yield* Queue.unbounded<string, SyncSocketError>();

        yield* Ref.update(state, (current) => ({ ...current, open: queue }));
        yield* Effect.addFinalizer(() =>
          Ref.update(state, (current) =>
            current.open === queue ? { ...current, open: null } : current,
          ),
        );

        const connection: SyncConnection = {
          incoming: decodeFrames(Stream.fromQueue(queue)),
          send: (frame) => Ref.update(sent, (list) => [...list, frame]),
        };

        return connection;
      });

      const memory = MemorySocket.of({
        push: (frame) => pushText(JSON.stringify(frame)),
        pushText,
        drop,
        setReachable: (reachable) => Ref.update(state, (current) => ({ ...current, reachable })),
        isOpen: Effect.map(Ref.get(state), ({ open }) => open !== null),
        dials: Ref.get(dials),
        sent: Ref.get(sent),
        clearSent: Ref.set(sent, []),
      });

      return Context.make(MemorySocket, memory).pipe(
        Context.add(SyncSocket, SyncSocket.of({ connect })),
      );
    }),
  );
}

/** A browser lifecycle event: back online, tab shown, tab hidden. */
export type LifecycleEvent = "online" | "visible" | "hidden";

/** Browser lifecycle events the test fires. */
export class TestLifecycle extends Context.Service<
  TestLifecycle,
  { readonly fire: (event: LifecycleEvent) => Effect.Effect<void> }
>()("smartfire/sync/TestLifecycle") {
  /** `TestLifecycle` plus the `Lifecycle` it drives. The tab starts shown. */
  static readonly layer = Layer.effectContext(
    Effect.gen(function* () {
      const events = yield* PubSub.unbounded<LifecycleEvent>();
      const visible = yield* Ref.make(true);
      const stream = Stream.fromPubSub(events);

      const fire = Effect.fnUntraced(function* (event: LifecycleEvent) {
        if (event !== "online") {
          yield* Ref.set(visible, event === "visible");
        }

        yield* PubSub.publish(events, event);
      });

      const lifecycle = Lifecycle.of({
        wakeups: stream.pipe(
          Stream.filter((event) => event !== "hidden"),
          Stream.as(undefined),
        ),
        visibility: stream.pipe(
          Stream.filter((event) => event !== "online"),
          Stream.map((event) => event === "visible"),
        ),
        isVisible: Ref.get(visible),
      });

      return Context.make(TestLifecycle, TestLifecycle.of({ fire })).pipe(
        Context.add(Lifecycle, lifecycle),
      );
    }),
  );
}
