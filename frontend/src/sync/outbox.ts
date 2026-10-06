import { Clock, Context, Effect, FiberMap, Layer, Result, Schedule, Stream } from "effect";
import type { ApiClient } from "../api/client.ts";
import { createMessage } from "../api/endpoints.ts";
import { type ApiFailure, NetworkError, ServerError } from "../api/errors.ts";
import { uuid7 } from "../lib/uuid7.ts";
import { mutations, store } from "../store/store.ts";
import { SyncLink } from "./link.ts";

/** Network errors and 5xx are worth retrying; any 4xx is final. */
const isTransient = (error: ApiFailure) =>
  error instanceof NetworkError || (error instanceof ServerError && error.status >= 500);

/**
 * Retry backoff for a send: exponential from 1 s, capped at 30 s, until it lands or fails for good.
 * A reconnected socket cuts the current wait short (the network is evidently back).
 */
const resendSchedule = Schedule.min([
  Schedule.exponential("1 second", 2),
  Schedule.spaced("30 seconds"),
]);

/**
 * Optimistic sends. A message shows as pending at once and is posted with a UUID v7
 * `clientMessageId`; transient failures retry with backoff, a 4xx marks it failed (Retry keeps
 * the same id, so the server dedupes); a resend waiting out its backoff goes at once when the sync
 * socket reconnects. The store reconciles by `clientMessageId` whichever of the
 * reply and the `message.created` event arrives first.
 */
export class Outbox extends Context.Service<
  Outbox,
  {
    /** Queues `markdown` for `roomId`; returns its `clientMessageId`. */
    readonly send: (roomId: number, markdown: string) => Effect.Effect<string>;
    /** Sends a failed message again, with the same `clientMessageId`. */
    readonly retry: (clientMessageId: string) => Effect.Effect<void>;
    /** Drops a pending or failed message (and stops its retries). */
    readonly discard: (clientMessageId: string) => Effect.Effect<void>;
  }
>()("smartfire/sync/Outbox") {
  static readonly layer = Layer.effect(
    Outbox,
    Effect.gen(function* () {
      const inFlight = yield* FiberMap.make<string>();
      const context = yield* Effect.context<ApiClient>();
      const link = yield* SyncLink;
      const reconnected = Effect.asVoid(Stream.runHead(link.attached));

      /** Posts until it lands or fails for good, backing off between transient failures. */
      const post = Effect.fnUntraced(function* (
        clientMessageId: string,
        roomId: number,
        markdownSource: string,
      ) {
        const step = yield* Schedule.toStep(resendSchedule);

        const attempt = createMessage(roomId, {
          clientMessageId,
          markdownSource,
          replyToMessageId: null,
          replyNotifyAuthor: null,
          attachmentSignedId: null,
        });

        while (true) {
          const result = yield* Effect.result(attempt);

          if (Result.isSuccess(result)) {
            return result.success;
          }

          if (!isTransient(result.failure)) {
            return yield* Effect.fail(result.failure);
          }

          const [, delay] = yield* Effect.orDie(step(yield* Clock.currentTimeMillis, undefined));

          yield* Effect.raceFirst(Effect.sleep(delay), reconnected);
        }
      });

      const deliver = (clientMessageId: string, roomId: number, markdownSource: string) =>
        post(clientMessageId, roomId, markdownSource).pipe(
          Effect.matchEffect({
            onSuccess: (message) => Effect.sync(() => mutations.receiveMessage(message)),
            onFailure: (error) =>
              Effect.sync(() =>
                mutations.setPendingState(clientMessageId, "failed", error.message),
              ),
          }),
          Effect.provideContext(context),
        );

      const send = Effect.fnUntraced(function* (roomId: number, markdown: string) {
        const now = yield* Clock.currentTimeMillis;
        const clientMessageId = uuid7(now);
        const state = store.getState();

        mutations.addPending({
          clientMessageId,
          roomId,
          creatorId: state.me?.user.id ?? state.boot?.user.id ?? 0,
          markdownSource: markdown,
          createdAt: new Date(now).toISOString(),
          state: "sending",
          error: null,
        });

        yield* FiberMap.run(inFlight, clientMessageId, deliver(clientMessageId, roomId, markdown));

        return clientMessageId;
      });

      const retry = Effect.fnUntraced(function* (clientMessageId: string) {
        const pending = store.getState().pending[clientMessageId];

        if (pending === undefined || pending.state !== "failed") {
          return;
        }

        mutations.setPendingState(clientMessageId, "sending", null);

        yield* FiberMap.run(
          inFlight,
          clientMessageId,
          deliver(clientMessageId, pending.roomId, pending.markdownSource),
        );
      });

      const discard = Effect.fnUntraced(function* (clientMessageId: string) {
        yield* FiberMap.remove(inFlight, clientMessageId);

        mutations.discardPending(clientMessageId);
      });

      return Outbox.of({ send, retry, discard });
    }),
  );
}
