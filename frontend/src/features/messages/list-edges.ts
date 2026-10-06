/**
 * Home and End on a message row: the list is virtualised, so its first or last message may not be
 * rendered. The row asks its list (a bubbling event); the list scrolls its virtualiser there and
 * focuses that message's row once it's drawn.
 */
import { type RefObject, useEffect, useRef } from "react";
import type { VListHandle } from "virtua";
import type { TimelineItem } from "../room/timeline-items.ts";

const EDGE_EVENT = "message-list-edge";

export type ListEdge = "first" | "last";

/** Asks `row`'s list to bring its first or last message into view; answers whether one did. */
export function requestListEdge(row: HTMLElement, edge: ListEdge): boolean {
  const event = new CustomEvent<ListEdge>(EDGE_EVENT, {
    bubbles: true,
    cancelable: true,
    detail: edge,
  });

  return !row.dispatchEvent(event);
}

function focusRow(container: HTMLElement, messageId: number, triesLeft: number): void {
  const row = container.querySelector<HTMLElement>(
    `[data-message-row][data-message-id="${messageId}"]`,
  );

  // A row the virtualiser hasn't measured yet is drawn hidden and can't take focus: try again.
  row?.focus({ preventScroll: true });

  if (row !== null && document.activeElement === row) {
    row.scrollIntoView({ block: "nearest" });

    return;
  }

  if (triesLeft > 0) {
    requestAnimationFrame(() => focusRow(container, messageId, triesLeft - 1));
  }
}

/** Answers Home/End requests from the rows inside `containerRef` by scrolling `listRef`. */
export function useListEdges(
  containerRef: RefObject<HTMLElement | null>,
  listRef: RefObject<VListHandle | null>,
  items: readonly TimelineItem[],
): void {
  const itemsRef = useRef(items);

  useEffect(() => {
    itemsRef.current = items;
  });

  useEffect(() => {
    const container = containerRef.current;

    if (container === null) {
      return;
    }

    const onEdge = (event: Event) => {
      const list = listRef.current;
      const current = itemsRef.current;
      const edge = event instanceof CustomEvent && event.detail === "first" ? "first" : "last";

      const index =
        edge === "first"
          ? current.findIndex((item) => item.kind === "message")
          : current.findLastIndex((item) => item.kind === "message");

      const item = current[index];

      if (list === null || item === undefined || item.kind !== "message") {
        return;
      }

      event.preventDefault();
      list.scrollToIndex(index, { align: edge === "first" ? "start" : "end" });
      requestAnimationFrame(() => focusRow(container, item.message.id, 30));
    };

    container.addEventListener(EDGE_EVENT, onEdge);

    return () => container.removeEventListener(EDGE_EVENT, onEdge);
  }, [containerRef, listRef]);
}
