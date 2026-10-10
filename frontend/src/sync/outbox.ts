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

const roomSendRevisions = new Map<number, number>();

/** Local root-timeline send attempts, including retries, even after their pending rows disappear. */
export function roomSendRevision(roomId: number): number {
  return roomSendRevisions.get(roomId) ?? 0;
}

function noteSend(pending: PendingMessage): void {
  if (pending.threadId === null) {
    roomSendRevisions.set(pending.roomId, roomSendRevision(pending.roomId) + 1);
  }
}

/** Where a message goes and what it carries besides its Markdown. */
export interface SendOptions {
  /** Reply in this thread instead of on the room's root timeline. */
  readonly threadId?: number | null;
  /** A finished direct upload, attached as the message's file. */
  readonly attachmentSignedId?: string | null;
  /** What the pending row shows for that file. */
  readonly attachment?: PendingAttachment | null;
  /** Drive files pinned on this message. Left out when there are none. */
  readonly driveFileIds?: readonly string[];
  /** An inline reply: the message it answers and whether that author is notified. */
  readonly reply?: { readonly messageId: number; readonly notify: boolean } | null;
  /** The `clientMessageId` to post under, when the caller follows the send; made here otherwise. */
  readonly clientMessageId?: string;
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

        const driveFileIds = pending.driveFileIds;

        const message = {
          clientMessageId: pending.clientMessageId,
          markdownSource: pending.markdownSource,
          replyToMessageId: pending.replyToMessageId,
          replyNotifyAuthor: pending.replyNotifyAuthor,
          attachmentSignedId: pending.attachmentSignedId,
        };

        const body =
          driveFileIds !== undefined && driveFileIds.length > 0
            ? { ...message, driveFileIds: [...driveFileIds] }
            : message;

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
        const clientMessageId = options.clientMessageId ?? uuid7(now);
        const state = store.getState();

        const driveFileIds = options.driveFileIds;

        const pendingBase = {
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
          state: "sending" as const,
          error: null,
        };

        const pending: PendingMessage =
          driveFileIds !== undefined && driveFileIds.length > 0
            ? { ...pendingBase, driveFileIds }
            : pendingBase;

        noteSend(pending);
        mutations.addPending(pending);

        yield* FiberMap.run(inFlight, clientMessageId, deliver(pending));

        return clientMessageId;
      });

      const retry = Effect.fnUntraced(function* (clientMessageId: string) {
        const pending = store.getState().pending[clientMessageId];

        if (pending === undefined || pending.state !== "failed") {
          return;
        }

        noteSend(pending);
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
