import { Context, Effect, Layer, Ref, Semaphore } from "effect";
import type { ClientFrame } from "../gen/ClientFrame.ts";
import type { SyncConnection } from "./socket.ts";

/**
 * The current connection, if any, as the rest of the engine sees it. Frames sent while there is
 * none are dropped: the next `hello` carries the topics, and typing or presence would be stale.
 * Attaching sends the greeting (`hello` first) before any other frame can go out.
 */
export class SyncLink extends Context.Service<
  SyncLink,
  {
    readonly send: (frame: ClientFrame) => Effect.Effect<void>;
    /** Makes `connection` current, after sending the frames `greeting` computes. */
    readonly attach: (
      connection: SyncConnection,
      greeting: Effect.Effect<readonly ClientFrame[]>,
    ) => Effect.Effect<void>;
    readonly detach: Effect.Effect<void>;
    readonly isConnected: Effect.Effect<boolean>;
  }
>()("smartfire/sync/SyncLink") {
  static readonly layer = Layer.effect(
    SyncLink,
    Effect.gen(function* () {
      const current = yield* Ref.make<SyncConnection | null>(null);
      const lock = yield* Semaphore.make(1);

      const send = (frame: ClientFrame) =>
        lock.withPermit(
          Effect.flatMap(Ref.get(current), (connection) =>
            connection === null ? Effect.void : connection.send(frame),
          ),
        );

      const attach = (
        connection: SyncConnection,
        greeting: Effect.Effect<readonly ClientFrame[]>,
      ) =>
        lock.withPermit(
          Effect.gen(function* () {
            for (const frame of yield* greeting) {
              yield* connection.send(frame);
            }

            yield* Ref.set(current, connection);
          }),
        );

      return SyncLink.of({
        send,
        attach,
        detach: lock.withPermit(Ref.set(current, null)),
        isConnected: Effect.map(Ref.get(current), (connection) => connection !== null),
      });
    }),
  );
}
