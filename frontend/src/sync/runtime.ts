import { type Effect, Layer, ManagedRuntime } from "effect";
import { ApiClient, ApiConfig, endpointUrl } from "../api/client.ts";
import type { ActivityItem } from "../gen/ActivityItem.ts";
import type { ActivityState } from "../gen/ActivityState.ts";
import type { ActivityTab } from "../gen/ActivityTab.ts";
import type { CreateScheduledMessage } from "../gen/CreateScheduledMessage.ts";
import type { CreateUpload } from "../gen/CreateUpload.ts";
import type { DirectUpload } from "../gen/DirectUpload.ts";
import type { ForwardDestinationList } from "../gen/ForwardDestinationList.ts";
import type { ForwardTarget } from "../gen/ForwardTarget.ts";
import type { Icon } from "../gen/Icon.ts";
import type { MessageDTO } from "../gen/MessageDTO.ts";
import type { PinList } from "../gen/PinList.ts";
import type { SavedFilter } from "../gen/SavedFilter.ts";
import type { SavedItem } from "../gen/SavedItem.ts";
import type { SavedStatus } from "../gen/SavedStatus.ts";
import type { ScheduledMessage } from "../gen/ScheduledMessage.ts";
import type { ThreadFilter } from "../gen/ThreadFilter.ts";
import type { ThreadInvolvement } from "../gen/ThreadInvolvement.ts";
import type { UpdateScheduledMessage } from "../gen/UpdateScheduledMessage.ts";
import type { UpdateThread } from "../gen/UpdateThread.ts";
import type { ActivityAction } from "../store/activity.ts";
import type { ScheduledListKey } from "../store/scheduled.ts";
import * as activityActions from "./activity-actions.ts";
import * as agentActions from "./agent-actions.ts";
import { Engine } from "./engine.ts";
import { SyncServices } from "./layers.ts";
import { Lifecycle } from "./lifecycle.ts";
import * as messageActions from "./message-actions.ts";
import * as messageViewActions from "./message-view-actions.ts";
import { Outbox, type SendOptions } from "./outbox.ts";
import { Presence } from "./presence.ts";
import { ActionError, asAction } from "./run.ts";
import * as savedActions from "./saved-actions.ts";
import * as scheduledActions from "./scheduled-actions.ts";
import * as session from "./session.ts";
import { SyncSocket } from "./socket.ts";
import * as threadActions from "./thread-actions.ts";
import { prefetchMemberships } from "./thread-prefetch.ts";
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
  /**
   * "Mark unread from here": the divider moves and the sidebar counts from it. Answers the first
   * unread message's id.
   */
  markUnreadFrom: (roomId: number, messageId: number): Promise<number | null> =>
    runAction(messageViewActions.markUnreadFrom(roomId, messageId)),
  /** Every brand and workspace icon, for the emoji picker's Custom tab. */
  customIcons: (): Promise<readonly Icon[]> => runAction(messageViewActions.customIcons()),
};

/** Thread actions (S2). Loads land in the store (errors too); writes reject on failure. */
const threads = {
  /** Subscribes and loads the pane: the newest replies, or those around `focusMessageId`. */
  open: (threadId: number, focusMessageId: number | null = null): Promise<void> =>
    runAction(threadActions.open(threadId, focusMessageId)),
  /** Loads the open pane again (its Retry); doesn't subscribe a second time. */
  reload: (threadId: number, focusMessageId: number | null = null): Promise<void> =>
    runAction(threadActions.reload(threadId, focusMessageId)),
  close(threadId: number): void {
    runtime.runFork(threadActions.close(threadId));
  },
  loadOlder: (threadId: number): Promise<void> => runAction(threadActions.loadOlder(threadId)),
  loadNewer: (threadId: number): Promise<void> => runAction(threadActions.loadNewer(threadId)),
  create: (
    roomId: number,
    parentMessageId: number,
    markdown: string,
    options?: {
      readonly name?: string | null;
      readonly attachmentSignedId?: string | null;
      readonly clientMessageId?: string;
    },
  ): Promise<number> => runAction(threadActions.create(roomId, parentMessageId, markdown, options)),
  update: (threadId: number, body: UpdateThread): Promise<void> =>
    runAction(threadActions.update(threadId, body)),
  follow: (threadId: number, involvement: ThreadInvolvement | null): Promise<void> =>
    runAction(threadActions.follow(threadId, involvement)),
  markRead: (threadId: number): Promise<void> => runAction(threadActions.markRead(threadId)),
  list: (roomId: number, filter: ThreadFilter): Promise<void> =>
    runAction(threadActions.list(roomId, filter)),
  /** The viewer's memberships in the room's active threads, so reply indicators can show unread. */
  prefetchMemberships: (roomId: number): Promise<void> => runAction(prefetchMemberships(roomId)),
};

/**
 * The activity inbox (S3). Loads land in the store (a failure as the list's `error`) and never
 * reject; state changes show at once and reject (rolled back) on failure.
 */
const activity = {
  /** Loads (or reloads) a tab's first page in one state. */
  load: (tab: ActivityTab, status: ActivityState): Promise<void> =>
    runAction(activityActions.load(tab, status)),
  /** Loads the tab's next page in that state, if any. */
  loadMore: (tab: ActivityTab, status: ActivityState): Promise<void> =>
    runAction(activityActions.loadMore(tab, status)),
  /** Refreshes the badge; answers the count. */
  loadUnreadCount: (): Promise<number> => runAction(activityActions.loadUnreadCount()),
  /** Read, unread, handled, or unhandled ("Clear handled"). */
  setState: (activityItemId: number, action: ActivityAction): Promise<ActivityItem> =>
    runAction(activityActions.setState(activityItemId, action)),
  /** Marks it read and answers it; then go to `activityDestination(item)`. */
  open: (activityItemId: number): Promise<ActivityItem> =>
    runAction(activityActions.open(activityItemId)),
};

/** The Saved page (S3). Loads never reject; writes show at once and reject on failure. */
const saved = {
  load: (filter: SavedFilter): Promise<void> => runAction(savedActions.load(filter)),
  loadMore: (filter: SavedFilter): Promise<void> => runAction(savedActions.loadMore(filter)),
  /** Mark done (`done`) or reopen (`in_progress`). */
  setStatus: (savedItemId: number, status: SavedStatus): Promise<SavedItem> =>
    runAction(savedActions.setStatus(savedItemId, status)),
  /** Unsave. */
  remove: (savedItemId: number): Promise<void> => runAction(savedActions.remove(savedItemId)),
  /** Undoes a removal: saves the message again with its status and (still due) reminder. */
  restore: (removed: SavedItem): Promise<SavedItem> => runAction(savedActions.restore(removed)),
  /** Sets (RFC 3339, future) or clears (`null`) the reminder; saves the message if it wasn't. */
  setReminder: (messageId: number, remindAt: string | null): Promise<SavedItem> =>
    runAction(savedActions.setReminder(messageId, remindAt)),
};

/** Scheduled messages (S3, and the composer's). Loads never reject; writes reject on failure. */
const scheduled = {
  /** Loads (or reloads) `pending`, `past` or `room:<id>` (see `roomScheduledKey`). */
  load: (key: ScheduledListKey): Promise<void> => runAction(scheduledActions.load(key)),
  loadMore: (key: ScheduledListKey): Promise<void> => runAction(scheduledActions.loadMore(key)),
  create: (roomId: number, body: CreateScheduledMessage): Promise<ScheduledMessage> =>
    runAction(scheduledActions.create(roomId, body)),
  /** New text and/or time. */
  update: (id: number, body: UpdateScheduledMessage): Promise<ScheduledMessage> =>
    runAction(scheduledActions.update(id, body)),
  /**
   * `"sent"`, or `"held"` (202: it stays scheduled). Rejects when it was dropped instead, with an
   * error `isScheduledDropped` recognises (its message is the reason).
   */
  sendNow: (id: number): Promise<"sent" | "held"> => runAction(scheduledActions.sendNow(id)),
  /** Cancels it at once; it comes back if refused (409 while sending). */
  cancel: (id: number): Promise<void> => runAction(scheduledActions.cancel(id)),
};

/** The agent screens (S4). Loads land in the store (failures as its error) and never reject. */
const agents = {
  /** Loads (or reloads) the directory. */
  loadDirectory: (): Promise<void> => runAction(agentActions.loadDirectory()),
  /** Loads (or reloads) an agent's profile. */
  loadProfile: (agentId: number): Promise<void> => runAction(agentActions.loadProfile(agentId)),
};

/** True for `actions.scheduled.sendNow`'s rejection when the message was dropped, not sent. */
export function isScheduledDropped(error: Error): boolean {
  return error instanceof ActionError && error.tag === "ScheduledDropped";
}

/** What React calls. Nothing here throws synchronously; failures land in the store or reject. */
export const actions = {
  messages,
  threads,
  activity,
  saved,
  scheduled,
  agents,

  endpointUrl: (path: string): Promise<string> => runtime.runPromise(endpointUrl(path)),

  /** Loads the people among `ids` the store doesn't hold yet. */
  ensureUsers: (ids: readonly number[]): Promise<void> =>
    runAction(messageViewActions.ensureUsers(ids)),

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

  /** Loads an open room again (its Try again); doesn't subscribe or say present a second time. */
  reloadRoom: (roomId: number, focusMessageId: number | null): Promise<void> =>
    runtime.runPromise(session.reloadRoom(roomId, focusMessageId)),

  /** Releases the room's topic, says absent, clears the unread divider. */
  closeRoom(roomId: number): void {
    runtime.runFork(session.closeRoom(roomId));
  },

  loadOlder: (roomId: number): Promise<void> => runtime.runPromise(session.loadOlder(roomId)),

  loadNewer: (roomId: number): Promise<void> => runtime.runPromise(session.loadNewer(roomId)),

  /** Loads the window around a message the open room hasn't loaded. */
  loadAround: (roomId: number, messageId: number): Promise<void> =>
    runtime.runPromise(session.loadAround(roomId, messageId)),

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
