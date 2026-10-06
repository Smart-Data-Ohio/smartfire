import { Layer, ManagedRuntime } from "effect";
import { ApiClient, ApiConfig, endpointUrl } from "../api/client.ts";
import { Engine } from "./engine.ts";
import { SyncServices } from "./layers.ts";
import { Lifecycle } from "./lifecycle.ts";
import { Outbox } from "./outbox.ts";
import { Presence } from "./presence.ts";
import * as session from "./session.ts";
import { SyncSocket } from "./socket.ts";
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

/** What React calls. Nothing here throws synchronously; failures land in the store. */
export const actions = {
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

  /** Sends optimistically: the pending row shows at once. */
  send(roomId: number, markdown: string): void {
    runtime.runFork(session.send(roomId, markdown));
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

  /** Call on each keystroke (`true`) and on blur/clear (`false`); throttled to 1 per 3 s. */
  setTyping(roomId: number, on: boolean): void {
    runtime.runFork(Typing.use((typing) => typing.set(roomId, on)));
  },

  /** The person did something (typed, clicked, scrolled): the next heartbeat says active. */
  noteActivity(): void {
    runtime.runFork(Presence.use((presence) => presence.noteActivity));
  },
};
