import { useStore as useZustand } from "zustand";
import { createStore } from "zustand/vanilla";
import type { MessageDTO } from "../../store/model.ts";
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

/** Forgets every reply target (for tests). */
export function resetReplies(): void {
  replyStore.setState({ targets: {} });
}
