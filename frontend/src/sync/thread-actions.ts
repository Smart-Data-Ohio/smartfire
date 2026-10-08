/**
 * Threads as Effect programs: open one in the pane (subscribe, header, newest replies), page
 * through its replies, start one, rename/close/lock it, follow it and mark it read, and list a
 * room's threads.
 */
import { Effect, Result } from "effect";
import * as api from "../api/thread-endpoints.ts";
import type { ThreadFilter } from "../gen/ThreadFilter.ts";
import type { ThreadInvolvement } from "../gen/ThreadInvolvement.ts";
import type { UpdateThread } from "../gen/UpdateThread.ts";
import { uuid7 } from "../lib/uuid7.ts";
import { mutations, store } from "../store/store.ts";
import { setThreadUnread } from "../store/threads.ts";
import { Topics } from "./topics.ts";
import { Typing } from "./typing.ts";

export const threadTopic = (threadId: number) => `thread:${threadId}`;

/**
 * Loads the thread's header and its newest replies (or those around `focusMessageId`, a reply's
 * permalink) into the pane. Errors land in the store.
 */
const loadPane = Effect.fnUntraced(function* (threadId: number, focusMessageId: number | null) {
  mutations.setThreadPaneLoading(threadId);
  mutations.setThreadPageLoading(threadId, "newer");
  mutations.setThreadPageReplacing(threadId);

  const since = store.getState().removalCount;

  const [detail, page] = yield* Effect.all(
    [
      Effect.result(api.thread(threadId)),
      Effect.result(
        api.threadMessages(threadId, focusMessageId === null ? null : { around: focusMessageId }),
      ),
    ],
    { concurrency: 2 },
  );

  if (Result.isFailure(detail)) {
    mutations.setThreadPaneError(threadId, detail.failure.message);
    mutations.setThreadPageFailed(threadId);

    return;
  }

  mutations.loadThreadDetail(detail.success, since);

  if (Result.isFailure(page)) {
    mutations.setThreadPageFailed(threadId);
  } else {
    mutations.applyThreadPage(threadId, page.success, "replace");
  }
});

/** Subscribes to the thread, then loads its header and newest replies (or those around a reply). */
export const open = Effect.fn("threads.open")(function* (
  threadId: number,
  focusMessageId: number | null = null,
) {
  const topics = yield* Topics;

  yield* topics.acquire(threadTopic(threadId));
  yield* loadPane(threadId, focusMessageId);
});

/** Loads an open pane again after an error; the subscription `open` took still holds. */
export const reload = Effect.fn("threads.reload")(function* (
  threadId: number,
  focusMessageId: number | null = null,
) {
  yield* loadPane(threadId, focusMessageId);
});

/** Leaves the pane: releases the topic and stops typing there. */
export const close = Effect.fn("threads.close")(function* (threadId: number) {
  const topics = yield* Topics;
  const typing = yield* Typing;

  yield* typing.set(threadTopic(threadId), false);
  yield* topics.release(threadTopic(threadId));
});

const loadPage = Effect.fnUntraced(function* (threadId: number, direction: "older" | "newer") {
  const timeline = store.getState().threadTimelines[threadId];

  if (timeline === undefined) {
    return;
  }

  const from = direction === "older" ? timeline.before : timeline.after;
  const busy = direction === "older" ? timeline.loadingOlder : timeline.loadingNewer;

  if (from === null || busy) {
    return;
  }

  mutations.setThreadPageLoading(threadId, direction);

  yield* api
    .threadMessages(threadId, direction === "older" ? { before: from } : { after: from })
    .pipe(
      Effect.tap((page) => Effect.sync(() => mutations.applyThreadPage(threadId, page, direction))),
      Effect.catch(() => Effect.sync(() => mutations.setThreadPageFailed(threadId))),
    );
});

/** The replies before the loaded window. */
export const loadOlder = Effect.fn("threads.loadOlder")(function* (threadId: number) {
  yield* loadPage(threadId, "older");
});

/** The replies after the loaded window. */
export const loadNewer = Effect.fn("threads.loadNewer")(function* (threadId: number) {
  yield* loadPage(threadId, "newer");
});

/**
 * Starts a thread on `parentMessageId` with its first reply; answers the new thread's id so the
 * pane can move to it. Pass the same `clientMessageId` when retrying: the server answers the
 * thread the first attempt made instead of starting a second one.
 */
export const create = Effect.fn("threads.create")(function* (
  roomId: number,
  parentMessageId: number,
  markdown: string,
  options: {
    readonly name?: string | null;
    readonly attachmentSignedId?: string | null;
    readonly clientMessageId?: string;
  } = {},
) {
  const since = store.getState().removalCount;

  const created = yield* api.createThread(roomId, {
    parentMessageId,
    name: options.name ?? null,
    message: {
      clientMessageId: options.clientMessageId ?? uuid7(Date.now()),
      markdownSource: markdown,
      replyToMessageId: null,
      replyNotifyAuthor: null,
      attachmentSignedId: options.attachmentSignedId ?? null,
    },
  });

  mutations.threadCreated(created, since);

  return created.detail.thread.id;
});

/** Renames, closes, reopens, locks or unlocks it. */
export const update = Effect.fn("threads.update")(function* (threadId: number, body: UpdateThread) {
  const since = store.getState().removalCount;

  mutations.loadThreadDetail(yield* api.updateThread(threadId, body), since);
});

/** Deletes it on the server; its `thread.removed` takes it out of the store and the board. */
export const remove = Effect.fn("threads.remove")(function* (threadId: number) {
  yield* api.deleteThread(threadId);
});

/** Follows (`everything`), mentions-only, or leaves (`null`). */
export const follow = Effect.fn("threads.follow")(function* (
  threadId: number,
  involvement: ThreadInvolvement | null,
) {
  const reply = yield* api.joinThread(threadId, involvement);

  mutations.setThreadMembership(threadId, reply.membership);
});

/** Marks it read here at once, then on the server. */
export const markRead = Effect.fn("threads.markRead")(function* (threadId: number) {
  const membership = store.getState().threadMemberships[threadId];

  if (membership == null || membership.unreadAt === null) {
    return;
  }

  store.setState((state) => setThreadUnread(state, threadId, null), true);

  yield* api.markThreadRead(threadId).pipe(
    Effect.tap((reply) =>
      Effect.sync(() => mutations.setThreadMembership(threadId, reply.membership)),
    ),
    Effect.catch((error) => Effect.logWarning("thread mark read failed", error.message)),
  );
});

/** The room's threads for one filter (the Threads pane). */
export const list = Effect.fn("threads.list")(function* (roomId: number, filter: ThreadFilter) {
  mutations.setThreadListLoading(roomId, filter);
  const since = store.getState().removalCount;

  yield* api.threads(roomId, filter).pipe(
    Effect.tap((threads) =>
      Effect.sync(() => mutations.loadThreadList(roomId, filter, threads, since)),
    ),
    Effect.catch(() => Effect.sync(() => mutations.setThreadListFailed(roomId, filter))),
  );
});
