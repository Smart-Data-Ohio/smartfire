import { Context, Effect, Layer, Ref, Stream } from "effect";
import type { ClientFrame } from "../gen/ClientFrame.ts";
import { Lifecycle } from "./lifecycle.ts";
import { SyncLink } from "./link.ts";

/** Workspace presence heartbeat, matching `WorkspacePresenceChannel`. */
export const HEARTBEAT_MS = 25_000;

/**
 * This person's presence: `hb{active}` every `HEARTBEAT_MS` (active when there was input since
 * the last one), `present`/`absent` as rooms open and close and as the tab is shown or hidden.
 */
export class Presence extends Context.Service<
  Presence,
  {
    /** A room view opened. */
    readonly enter: (roomId: number) => Effect.Effect<void>;
    /** A room view closed. */
    readonly leave: (roomId: number) => Effect.Effect<void>;
    /** The person typed, clicked or scrolled. */
    readonly noteActivity: Effect.Effect<void>;
    /** What a new connection needs after `hello`: `present` for each open room, when shown. */
    readonly greeting: Effect.Effect<readonly ClientFrame[]>;
    /** Heartbeats and tab visibility; runs until interrupted. */
    readonly run: Effect.Effect<void>;
  }
>()("smartfire/sync/Presence") {
  static readonly layer = Layer.effect(
    Presence,
    Effect.gen(function* () {
      const link = yield* SyncLink;
      const lifecycle = yield* Lifecycle;
      const openRooms = yield* Ref.make<ReadonlySet<number>>(new Set());
      const active = yield* Ref.make(false);

      const framesFor = (rooms: ReadonlySet<number>, t: "present" | "absent"): ClientFrame[] =>
        Array.from(rooms, (room) => ({ t, room }));

      const sendAll = Effect.fnUntraced(function* (frames: readonly ClientFrame[]) {
        for (const frame of frames) {
          yield* link.send(frame);
        }
      });

      const enter = Effect.fnUntraced(function* (roomId: number) {
        yield* Ref.update(openRooms, (rooms) => new Set(rooms).add(roomId));

        if (yield* lifecycle.isVisible) {
          yield* link.send({ t: "present", room: roomId });
        }
      });

      const leave = Effect.fnUntraced(function* (roomId: number) {
        const wasOpen = yield* Ref.modify(openRooms, (rooms) => {
          const next = new Set(rooms);

          return [next.delete(roomId), next] as const;
        });

        if (wasOpen && (yield* lifecycle.isVisible)) {
          yield* link.send({ t: "absent", room: roomId });
        }
      });

      const greeting = Effect.gen(function* () {
        return (yield* lifecycle.isVisible) ? framesFor(yield* Ref.get(openRooms), "present") : [];
      });

      const heartbeat = Ref.getAndSet(active, false).pipe(
        Effect.flatMap((wasActive) => link.send({ t: "hb", active: wasActive })),
        Effect.delay(HEARTBEAT_MS),
        Effect.forever,
      );

      const visibility = lifecycle.visibility.pipe(
        Stream.runForEach((visible) =>
          Effect.flatMap(Ref.get(openRooms), (rooms) =>
            sendAll(framesFor(rooms, visible ? "present" : "absent")),
          ),
        ),
      );

      return Presence.of({
        enter,
        leave,
        noteActivity: Ref.set(active, true),
        greeting,
        run: Effect.all([heartbeat, visibility], { concurrency: "unbounded", discard: true }),
      });
    }),
  );
}
