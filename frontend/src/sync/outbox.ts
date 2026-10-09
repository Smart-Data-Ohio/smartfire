import { Clock, Context, Effect, FiberMap, Layer, Result, Schedule, Stream } from "effect";
import type { ApiClient } from "../api/client.ts";
import { createMessage } from "../api/endpoints.ts";
import { type ApiFailure, NetworkError, ServerError } from "../api/errors.ts";
import { createThreadMessage } from "../api/thread-endpoints.ts";
import { uuid7 } from "../lib/uuid7.ts";
import type { PendingAttachment, PendingMessage } from "../store/model.ts";
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

/** Where a message goes and what it carries besides its Markdown. */
export interface SendOptions {
  /** Reply in this thread instead of on the room's root timeline. */
  readonly threadId?: number | null;
  /** A finished direct upload, attached as the message's file. */
  readonly attachmentSignedId?: string | null;
  /** What the pending row shows for that file. */
  readonly attachment?: PendingAttachment | null;
  /** An inline reply: the message it answers and whether that author is notified. */
  readonly reply?: { readonly messageId: number; readonly notify: boolean } | null;
}

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
    /** Queues `markdown` for `roomId` (or a thread in it); returns its `clientMessageId`. */
    readonly send: (
      roomId: number,
      markdown: string,
      options?: SendOptions,
    ) => Effect.Effect<string>;
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
      const post = Effect.fnUntraced(function* (pending: PendingMessage) {
        const step = yield* Schedule.toStep(resendSchedule);

        const body = {
          clientMessageId: pending.clientMessageId,
          markdownSource: pending.markdownSource,
          replyToMessageId: pending.replyToMessageId,
          replyNotifyAuthor: pending.replyNotifyAuthor,
          attachmentSignedId: pending.attachmentSignedId,
        };

        const attempt =
          pending.threadId === null
            ? createMessage(pending.roomId, body)
            : createThreadMessage(pending.threadId, body);

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

      const deliver = (pending: PendingMessage) =>
        post(pending).pipe(
          Effect.matchEffect({
            onSuccess: (message) => Effect.sync(() => mutations.receiveMessage(message)),
            onFailure: (error) =>
              Effect.sync(() =>
                mutations.setPendingState(pending.clientMessageId, "failed", error.message),
              ),
          }),
          Effect.provideContext(context),
        );

      const send = Effect.fnUntraced(function* (
        roomId: number,
        markdown: string,
        options: SendOptions = {},
      ) {
        const now = yield* Clock.currentTimeMillis;
        const clientMessageId = uuid7(now);
        const state = store.getState();

        const pending: PendingMessage = {
          clientMessageId,
          roomId,
          threadId: options.threadId ?? null,
          attachmentSignedId: options.attachmentSignedId ?? null,
          attachment: options.attachment ?? null,
          replyToMessageId: options.reply?.messageId ?? null,
          replyNotifyAuthor: options.reply?.notify ?? null,
          creatorId: state.me?.user.id ?? state.boot?.user.id ?? 0,
          markdownSource: markdown,
          createdAt: new Date(now).toISOString(),
          state: "sending",
          error: null,
        };

        mutations.addPending(pending);

        yield* FiberMap.run(inFlight, clientMessageId, deliver(pending));

        return clientMessageId;
      });

      const retry = Effect.fnUntraced(function* (clientMessageId: string) {
        const pending = store.getState().pending[clientMessageId];

        if (pending === undefined || pending.state !== "failed") {
          return;
        }

        mutations.setPendingState(clientMessageId, "sending", null);

        yield* FiberMap.run(inFlight, clientMessageId, deliver(pending));
      });

      const discard = Effect.fnUntraced(function* (clientMessageId: string) {
        yield* FiberMap.remove(inFlight, clientMessageId);

        mutations.discardPending(clientMessageId);
      });

      return Outbox.of({ send, retry, discard });
    }),
  );
}
