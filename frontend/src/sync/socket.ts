import { Context, Effect, Layer, Schema, type Scope, Stream } from "effect";
import * as Socket from "effect/socket/Socket";
import type { ClientFrame } from "../gen/ClientFrame.ts";
import type { ServerFrame } from "../gen/ServerFrame.ts";
import { decodeFrames, encodeClientFrame } from "./protocol.ts";

/** The sync socket couldn't open, dropped, or went silent. The engine reconnects. */
export class SyncSocketError extends Schema.TaggedError<SyncSocketError>()("SyncSocketError", {
  reason: Schema.String,
}) {}

/** One open connection to `/api/v1/sync`. */
export interface SyncConnection {
  /** Frames from the server, decoded; fails when the connection drops. */
  readonly incoming: Stream.Stream<ServerFrame, SyncSocketError>;
  /** Sends one frame. A frame written to a dying connection is lost, never an error. */
  readonly send: (frame: ClientFrame) => Effect.Effect<void>;
}

/**
 * The sync socket. `connect` dials `/api/v1/sync` once and succeeds when the connection is open;
 * it lasts until its scope closes or the server drops it. Reconnecting is the engine's job.
 */
export class SyncSocket extends Context.Service<
  SyncSocket,
  { readonly connect: Effect.Effect<SyncConnection, SyncSocketError, Scope.Scope> }
>()("smartfire/sync/SyncSocket") {
  /** The browser's WebSocket, on the page's own origin (`wss:` under `https:`). */
  static readonly layerWebSocket = Layer.effect(
    SyncSocket,
    Effect.gen(function* () {
      const webSocket = yield* Socket.WebSocketConstructor;

      const connect = Effect.gen(function* () {
        const socket = yield* Socket.makeWebSocket(Effect.sync(syncUrl)).pipe(
          Effect.provideService(Socket.WebSocketConstructor, webSocket),
        );

        const pull = yield* Socket.readerString(socket);
        const writer = yield* socket.writer;
        // Effect's writer waits for the next open once its socket closes; ours never reopens, so a
        // late frame is dropped instead (this finalizer runs before the socket's own).
        let open = true;

        yield* Effect.addFinalizer(() =>
          Effect.sync(() => {
            open = false;
          }),
        );

        const texts = Stream.fromPull(Effect.succeed(pull)).pipe(
          Stream.mapError((error) => new SyncSocketError({ reason: error.message })),
        );

        return {
          incoming: decodeFrames(texts),
          send: (frame: ClientFrame) =>
            Effect.suspend(() =>
              open
                ? writer
                    .write(encodeClientFrame(frame))
                    .pipe(
                      Effect.catchTag("SocketError", (error) =>
                        Effect.logDebug("sync: a frame was lost", error.message),
                      ),
                    )
                : Effect.void,
            ),
        } satisfies SyncConnection;
      }).pipe(
        Effect.catchTag("SocketError", (error) =>
          Effect.fail(new SyncSocketError({ reason: error.message })),
        ),
      );

      return SyncSocket.of({ connect });
    }),
  ).pipe(Layer.provide(Socket.layerWebSocketConstructorGlobal));
}

/** `ws(s)://<this host>/api/v1/sync`. */
function syncUrl(): string {
  const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";

  return `${protocol}//${window.location.host}/api/v1/sync`;
}
