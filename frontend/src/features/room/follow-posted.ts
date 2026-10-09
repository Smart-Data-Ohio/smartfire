/**
 * A slash command's post: the server writes the message, so there's no pending row for the list
 * to follow down as there is for a send. The composer notes the id the command answers with; the
 * conversation's list goes to that message once it's in the window, then forgets it.
 */
import { type RefObject, useLayoutEffect } from "react";
import type { VListHandle } from "virtua";
import { useStore as useZustand } from "zustand";
import { createStore } from "zustand/vanilla";
import type { TimelineItem } from "./timeline-items.ts";

/** Per conversation topic (`room:12`, `thread:88`), the message a command from here posted. */
const postedStore = createStore<Readonly<Record<string, number>>>()(() => ({}));

/** Notes that a command from this tab's composer posted `messageId` in `topic`. */
export function notePosted(topic: string, messageId: number): void {
  postedStore.setState({ [topic]: messageId });
}

function forgetPosted(topic: string, messageId: number): void {
  if (postedStore.getState()[topic] !== messageId) {
    return;
  }

  postedStore.setState(({ [topic]: _gone, ...rest }) => rest, true);
}

/** Scrolls `listRef` to the message noted for `topic` once it's among `items`. */
export function useFollowPosted(
  topic: string,
  items: readonly TimelineItem[],
  listRef: RefObject<Pick<VListHandle, "scrollToIndex"> | null>,
  beforeFollow?: () => void,
): void {
  const postedId = useZustand(postedStore, (state) => state[topic] ?? null);

  const index =
    postedId === null
      ? -1
      : items.findIndex((item) => item.kind === "message" && item.message.id === postedId);

  useLayoutEffect(() => {
    if (postedId === null || index < 0) {
      return;
    }

    beforeFollow?.();
    listRef.current?.scrollToIndex(index, { align: "end" });
    forgetPosted(topic, postedId);
  }, [topic, postedId, index, listRef, beforeFollow]);
}
