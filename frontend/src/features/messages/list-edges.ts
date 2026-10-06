/**
 * Home and End on a message row: the list is virtualised, so its first or last message may not be
 * rendered. The row asks its list (a bubbling event); the list scrolls its virtualiser there and
 * focuses that message's row once it's drawn, holding focus itself meanwhile.
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

/**
 * How long Home/End waits for the edge row to be drawn: past a page landing mid-scroll, which
 * can take longer than any fixed number of frames.
 */
const FOCUS_WAIT_MS = 3000;

function rowOf(container: HTMLElement, messageId: number): HTMLElement | null {
  return container.querySelector<HTMLElement>(`[data-message-row][data-message-id="${messageId}"]`);
}

/** Focuses `row`, answering whether it took focus: one not yet measured is drawn hidden and can't. */
function takesFocus(row: HTMLElement): boolean {
  row.focus({ preventScroll: true });

  return document.activeElement === row;
}

/**
 * Holds focus on the list while the edge row is drawn (and if it never is): off the row the key
 * moved away from at once, and never dropped on `<body>` when the scroll unmounts that row.
 */
function holdFocusOnList(container: HTMLElement): void {
  const list = container.querySelector<HTMLElement>('[role="log"]');

  if (list !== null && document.activeElement !== list) {
    list.focus({ preventScroll: true });
  }
}

/** Answers Home/End requests from the rows inside `containerRef` by scrolling `listRef`. */
export function useListEdges(
  containerRef: RefObject<HTMLElement | null>,
  listRef: RefObject<Pick<VListHandle, "scrollToIndex"> | null>,
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

      const messageId = item.message.id;
      const align = edge === "first" ? "start" : "end";
      const deadline = performance.now() + FOCUS_WAIT_MS;
      let scrolledTo = index;

      // Now, then each frame until the row takes focus; a page that landed meanwhile moved it, so
      // scroll to where it is now.
      const focusEdge = () => {
        const row = rowOf(container, messageId);

        if (row !== null && takesFocus(row)) {
          row.scrollIntoView({ block: "nearest" });

          return;
        }

        holdFocusOnList(container);

        const at = itemsRef.current.findIndex(
          (entry) => entry.kind === "message" && entry.message.id === messageId,
        );

        if (at >= 0 && at !== scrolledTo) {
          scrolledTo = at;
          listRef.current?.scrollToIndex(at, { align });
        }

        if (at >= 0 && performance.now() < deadline) {
          requestAnimationFrame(focusEdge);
        }
      };

      event.preventDefault();
      list.scrollToIndex(index, { align });
      focusEdge();
    };

    container.addEventListener(EDGE_EVENT, onEdge);

    return () => container.removeEventListener(EDGE_EVENT, onEdge);
  }, [containerRef, listRef]);
}
