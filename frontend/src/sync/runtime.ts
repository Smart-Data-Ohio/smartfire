import { type Effect, Layer, ManagedRuntime } from "effect";
import { ApiClient, ApiConfig, endpointUrl } from "../api/client.ts";
import type { CreateUpload } from "../gen/CreateUpload.ts";
import type { DirectUpload } from "../gen/DirectUpload.ts";
import type { ForwardDestinationList } from "../gen/ForwardDestinationList.ts";
import type { ForwardTarget } from "../gen/ForwardTarget.ts";
import type { MessageDTO } from "../gen/MessageDTO.ts";
import type { PinList } from "../gen/PinList.ts";
import type { SavedItem } from "../gen/SavedItem.ts";
import type { ThreadFilter } from "../gen/ThreadFilter.ts";
import type { ThreadInvolvement } from "../gen/ThreadInvolvement.ts";
import type { UpdateThread } from "../gen/UpdateThread.ts";
import { Engine } from "./engine.ts";
import { SyncServices } from "./layers.ts";
import { Lifecycle } from "./lifecycle.ts";
import * as messageActions from "./message-actions.ts";
import { Outbox, type SendOptions } from "./outbox.ts";
import { Presence } from "./presence.ts";
import { asAction } from "./run.ts";
import * as session from "./session.ts";
import { SyncSocket } from "./socket.ts";
import * as threadActions from "./thread-actions.ts";
import { Typing } from "./typing.ts";

const API_BASE = "/api/v1";

/**
 * The one Effect runtime the app shares. Code outside src/api and src/sync never imports
 * `effect`: it calls `actions`, plain async functions that run Effect programs here. Building it
 * opens nothing; the socket connects when `actions.start()` starts the engine.
 */
export const runtime = ManagedRuntime.make(
  SyncServices.pipe(
    Layer.provideMerge(
      Layer.mergeAll(
        ApiConfig.layer(API_BASE),
        ApiClient.layerBrowser(API_BASE),
        SyncSocket.layerWebSocket,
        Lifecycle.layerBrowser,
      ),
    ),
  ),
);

const runEngine = Engine.use((engine) => engine.run);

let started: Promise<void> | null = null;

type AppServices = ManagedRuntime.ManagedRuntime.Services<typeof runtime>;

/** Runs an action; failures reject with an `ActionError` whose message is fit to show. */
export const runAction = <A, E extends { readonly _tag: string; readonly message: string }>(
  effect: Effect.Effect<A, E, AppServices>,
): Promise<A> => runtime.runPromise(asAction(effect));

/** Message actions (S2). Each lands the server's reply in the store; failures reject. */
const messages = {
  edit: (messageId: number, markdown: string): Promise<MessageDTO> =>
    runAction(messageActions.edit(messageId, markdown)),
  source: (messageId: number): Promise<string> => runAction(messageActions.source(messageId)),
  remove: (messageId: number): Promise<void> => runAction(messageActions.remove(messageId)),
  toggleReaction: (
    messageId: number,
    content: string,
    shown: { readonly title: string; readonly imageUrl: string | null },
  ): Promise<void> => runAction(messageActions.toggleReaction(messageId, content, shown)),
  boost: (messageId: number, text: string): Promise<void> =>
    runAction(messageActions.boost(messageId, text)),
  removeBoost: (messageId: number, boostId: number): Promise<void> =>
    runAction(messageActions.removeBoost(messageId, boostId)),
  setPinned: (messageId: number, pinned: boolean): Promise<void> =>
    runAction(messageActions.setPinned(messageId, pinned)),
  pins: (roomId: number): Promise<PinList> => runAction(messageActions.pins(roomId)),
  save: (messageId: number, remindAt: string | null = null): Promise<SavedItem> =>
    runAction(messageActions.save(messageId, remindAt)),
  unsave: (messageId: number): Promise<void> => runAction(messageActions.unsave(messageId)),
  forwardDestinations: (): Promise<ForwardDestinationList> =>
    runAction(messageActions.forwardDestinations()),
  forward: (
    messageId: number,
    note: string | null,
    destinations: readonly ForwardTarget[],
  ): Promise<readonly MessageDTO[]> =>
    runAction(messageActions.forward(messageId, note, destinations)),
  startUpload: (body: CreateUpload): Promise<DirectUpload> =>
    runAction(messageActions.startUpload(body)),
};

/** Thread actions (S2). Loads land in the store (errors too); writes reject on failure. */
const threads = {
  open: (threadId: number): Promise<void> => runAction(threadActions.open(threadId)),
  close(threadId: number): void {
    runtime.runFork(threadActions.close(threadId));
  },
  loadOlder: (threadId: number): Promise<void> => runAction(threadActions.loadOlder(threadId)),
  loadNewer: (threadId: number): Promise<void> => runAction(threadActions.loadNewer(threadId)),
  create: (
    roomId: number,
    parentMessageId: number,
    markdown: string,
    options?: { readonly name?: string | null; readonly attachmentSignedId?: string | null },
  ): Promise<number> => runAction(threadActions.create(roomId, parentMessageId, markdown, options)),
  update: (threadId: number, body: UpdateThread): Promise<void> =>
    runAction(threadActions.update(threadId, body)),
  follow: (threadId: number, involvement: ThreadInvolvement | null): Promise<void> =>
    runAction(threadActions.follow(threadId, involvement)),
  markRead: (threadId: number): Promise<void> => runAction(threadActions.markRead(threadId)),
  list: (roomId: number, filter: ThreadFilter): Promise<void> =>
    runAction(threadActions.list(roomId, filter)),
};

/** What React calls. Nothing here throws synchronously; failures land in the store or reject. */
export const actions = {
  messages,
  threads,

  endpointUrl: (path: string): Promise<string> => runtime.runPromise(endpointUrl(path)),

  /**
   * Loads boot data, me, the sidebar and presence, then starts the sync engine. Idempotent; the
   * promise rejects (and a later call tries again) only when boot data or `me` can't be had.
   */
  start(): Promise<void> {
    started ??= runtime.runPromise(session.start()).then(
      () => {
        runtime.runFork(runEngine);
      },
      (error: Error) => {
        started = null;

        throw error;
      },
    );

    return started;
  },

  /** Subscribes, says present, and loads the room's detail and first page. */
  openRoom: (roomId: number, focusMessageId: number | null): Promise<void> =>
    runtime.runPromise(session.openRoom(roomId, focusMessageId)),

  /** Releases the room's topic, says absent, clears the unread divider. */
  closeRoom(roomId: number): void {
    runtime.runFork(session.closeRoom(roomId));
  },

  loadOlder: (roomId: number): Promise<void> => runtime.runPromise(session.loadOlder(roomId)),

  loadNewer: (roomId: number): Promise<void> => runtime.runPromise(session.loadNewer(roomId)),

  jumpToPresent: (roomId: number): Promise<void> =>
    runtime.runPromise(session.jumpToPresent(roomId)),

  /** Sends optimistically (to a thread with `options.threadId`): the pending row shows at once. */
  send(roomId: number, markdown: string, options?: SendOptions): void {
    runtime.runFork(session.send(roomId, markdown, options));
  },

  /** Sends a failed message again with the same client id. */
  retry(clientMessageId: string): void {
    runtime.runFork(Outbox.use((outbox) => outbox.retry(clientMessageId)));
  },

  /** Drops a pending or failed message. */
  discard(clientMessageId: string): void {
    runtime.runFork(Outbox.use((outbox) => outbox.discard(clientMessageId)));
  },

  /** Clears the room's unread state here at once, then on the server. */
  markRead: (roomId: number): Promise<void> => runtime.runPromise(session.markRead(roomId)),

  /**
   * Call on each keystroke (`true`) and on blur/clear (`false`); throttled to 1 per 3 s. In a
   * thread's composer pass its id: the indicator is per thread.
   */
  setTyping(roomId: number, on: boolean, threadId: number | null = null): void {
    const conv = threadId === null ? `room:${roomId}` : `thread:${threadId}`;

    runtime.runFork(Typing.use((typing) => typing.set(conv, on)));
  },

  /** The person did something (typed, clicked, scrolled): the next heartbeat says active. */
  noteActivity(): void {
    runtime.runFork(Presence.use((presence) => presence.noteActivity));
  },
};
