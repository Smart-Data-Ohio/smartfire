/**
 * Fresh loads of a thread's replies (opening or retrying the pane, and a sync resync) aren't
 * serialized with each other, so each takes a ticket and only the latest one lands: an older
 * snapshot answering late mustn't replace the replies a newer load installed. The latest load
 * then owns everything the superseded ones would have written: the pane's error, and the reply a
 * permalink asked to open at.
 */
import { Effect, Predicate, Result } from "effect";
import { thread, threadMessages } from "../api/thread-endpoints.ts";
import { mutations } from "../store/store.ts";
import { paneProblem, refetchThread, settled } from "./settle.ts";

interface Load {
  readonly ticket: number;
  /** The reply this load opens the replies around (a permalink's), until that page is in. */
  readonly focus: number | null;
}

const latest = new Map<number, Load>();

let issued = 0;

/** Starts a fresh load of `threadId`'s replies, around `focus` if set; any earlier one is superseded. */
export function beginThreadLoad(threadId: number, focus: number | null): number {
  issued += 1;
  latest.set(threadId, { ticket: issued, focus });

  return issued;
}

/** No fresh load of `threadId` started after the one holding `ticket`. */
export function isLatestThreadLoad(threadId: number, ticket: number): boolean {
  return latest.get(threadId)?.ticket === ticket;
}

/** The reply the latest load of `threadId` still has to open at, which a load superseding it keeps. */
export function pendingThreadFocus(threadId: number): number | null {
  return latest.get(threadId)?.focus ?? null;
}

/**
 * The load holding `ticket` installed its page (its reply is in view, or the newest replies once
 * the reply proved gone): no later load keeps the focus.
 */
export function finishThreadLoad(threadId: number, ticket: number): void {
  const load = latest.get(threadId);

  if (load?.ticket === ticket && load.focus !== null) {
    latest.set(threadId, { ticket, focus: null });
  }
}

/** A failed `around` page that means the permalink's reply is gone, not a passing failure. */
export const isGoneFocus = Predicate.isTagged("NotFound");

/**
 * The thread's header for the load holding `ticket`, installed as soon as it's settled (not after
 * the replies: a removal landing while they load would otherwise make it uncertain again with
 * nobody left to ask), unless a newer load started meanwhile: an older header answering late
 * mustn't replace the permissions, status or error a newer one installed.
 */
export const loadThreadHeader = (threadId: number, ticket: number) =>
  Effect.result(
    settled(
      thread(threadId),
      () => refetchThread(threadId),
      (answer) => [answer.thread.id],
      (answer, since) => {
        if (isLatestThreadLoad(threadId, ticket)) {
          mutations.loadThreadDetail(answer, since);
        }
      },
    ),
  );

/**
 * The permalink's `around` page was a 404 while the thread's header resolved. That may mean the
 * reply is gone, or that the viewer lost access between the two requests (a lost membership is a
 * 404 too), so the newest replies and the header are asked for again. The focus is dropped (no
 * later load asks for the reply again) only once both answer: the thread is still there for the
 * viewer, so it was the reply that went. Otherwise the pane says why and the focus waits for the
 * next load. Only the latest load writes the pane.
 */
export const openAtNewest = Effect.fnUntraced(function* (threadId: number, ticket: number) {
  const [detail, page] = yield* Effect.all(
    [loadThreadHeader(threadId, ticket), Effect.result(threadMessages(threadId, null))],
    { concurrency: 2 },
  );

  if (!isLatestThreadLoad(threadId, ticket)) {
    return;
  }

  const problem = paneProblem(detail);

  if (problem !== null) {
    mutations.setThreadPaneError(threadId, problem);
    mutations.setThreadPageFailed(threadId);

    return;
  }

  if (Result.isSuccess(page)) {
    mutations.applyThreadPage(threadId, page.success, "replace");
    finishThreadLoad(threadId, ticket);
  } else {
    mutations.setThreadPageFailed(threadId);
  }
});
