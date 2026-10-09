import { useStore as useZustand } from "zustand";
import { createStore } from "zustand/vanilla";
import type { MessageDTO, PendingMessage } from "../../store/model.ts";
import { store } from "../../store/store.ts";
import { draftKey } from "./draft.ts";

/** The message a composer's next send replies to (the classic composer's reply mode). */
export interface ReplyTarget {
  readonly messageId: number;
  /** Whether the replied-to author is notified; on by default, as in classic. */
  readonly notify: boolean;
  /** Bumps on every pick, so picking the same message again still focuses the composer. */
  readonly seq: number;
}

interface ReplyState {
  /** Keyed by the conversation's draft key (`draft.ts`): a room's composer, or a thread's. */
  readonly targets: Readonly<Record<string, ReplyTarget>>;
}

const replyStore = createStore<ReplyState>()(() => ({ targets: {} }));

let picks = 0;

/** The composer key a message's reply goes to: its own timeline (the room, or its thread). */
export function replyKey(message: Pick<MessageDTO, "roomId" | "threadId">): string {
  return draftKey(message.roomId, message.threadId);
}

/** Makes `message` the reply target of its conversation's composer, notifying its author. */
export function startReply(message: Pick<MessageDTO, "id" | "roomId" | "threadId">): void {
  picks += 1;

  const key = replyKey(message);
  const target: ReplyTarget = { messageId: message.id, notify: true, seq: picks };

  replyStore.setState((state) => ({ targets: { ...state.targets, [key]: target } }));
}

/** Drops the composer's reply target (×, Esc, or a send that carried it). */
export function cancelReply(key: string): void {
  if (replyStore.getState().targets[key] === undefined) {
    return;
  }

  replyStore.setState((state) => {
    const { [key]: _dropped, ...rest } = state.targets;

    return { targets: rest };
  });
}

/** Flips "Notify author" for the composer's reply. */
export function setReplyNotify(key: string, notify: boolean): void {
  const current = replyStore.getState().targets[key];

  if (current === undefined || current.notify === notify) {
    return;
  }

  replyStore.setState((state) => ({
    targets: { ...state.targets, [key]: { ...current, notify } },
  }));
}

/** The composer's reply target, or `null`. */
export function replyTarget(key: string): ReplyTarget | null {
  return replyStore.getState().targets[key] ?? null;
}

export function useReplyTarget(key: string | null): ReplyTarget | null {
  return useZustand(replyStore, (state) => (key === null ? null : (state.targets[key] ?? null)));
}

/** One send's messages that carried a reply: its text, and a message per further file. */
interface SentReply {
  readonly key: string;
  readonly target: ReplyTarget;
  /** The messages not yet settled (landed or discarded), and whether each has shown as pending. */
  readonly open: Map<string, boolean>;
  restored: boolean;
}

const sent = new Map<string, SentReply>();

let unwatch: (() => void) | null = null;

/** Puts a failed send's reply back, then drops it once that send lands (Retry) or is discarded. */
function settle(pending: Readonly<Record<string, PendingMessage>>): void {
  for (const [id, group] of sent) {
    const entry = pending[id];

    if (entry === undefined) {
      if (group.open.get(id) === true) {
        sent.delete(id);
        group.open.delete(id);

        const current = replyStore.getState().targets[group.key];

        if (group.open.size === 0 && group.restored && current?.seq === group.target.seq) {
          cancelReply(group.key);
        }
      }

      continue;
    }

    group.open.set(id, true);

    if (entry.state === "failed" && !group.restored) {
      group.restored = true;

      // A reply picked since then is newer input: keep it.
      if (replyStore.getState().targets[group.key] === undefined) {
        replyStore.setState((state) => ({
          targets: { ...state.targets, [group.key]: group.target },
        }));
      }
    }
  }

  if (sent.size === 0 && unwatch !== null) {
    unwatch();
    unwatch = null;
  }
}

/**
 * Follows a send that carried `target` (the composer drops the chip as it sends). If any of its
 * messages fails, the reply comes back with its notify choice, as classic keeps reply mode until
 * the send succeeds; it keeps its pick number, so the composer doesn't take the focus again.
 */
export function trackSentReply(
  key: string,
  target: ReplyTarget,
  clientMessageIds: readonly string[],
): void {
  const pending = store.getState().pending;

  const group: SentReply = {
    key,
    target,
    open: new Map(clientMessageIds.map((id) => [id, pending[id] !== undefined])),
    restored: false,
  };

  for (const id of clientMessageIds) {
    sent.set(id, group);
  }

  unwatch ??= store.subscribe((state, previous) => {
    if (state.pending !== previous.pending) {
      settle(state.pending);
    }
  });
  settle(pending);
}

/** Forgets every reply target (for tests). */
export function resetReplies(): void {
  replyStore.setState({ targets: {} });
  sent.clear();
  unwatch?.();
  unwatch = null;
}
