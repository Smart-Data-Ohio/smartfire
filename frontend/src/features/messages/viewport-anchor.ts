import { type RefObject, useLayoutEffect, useRef } from "react";
import type { VListHandle } from "virtua";
import type { TimelineItem } from "../room/timeline-items.ts";

type Anchor =
  | { readonly placement: string; readonly kind: "end" }
  | {
      readonly placement: string;
      readonly kind: "message";
      readonly id: number;
      readonly top: number;
      readonly inset: number;
    };

/** Keep a visible message in place through prepends and the cards chunk's measurements. */
export function useViewportAnchor({
  containerRef,
  listRef,
  items,
  placement,
  placed,
  shift,
  cardsSettled,
}: {
  readonly containerRef: RefObject<HTMLElement | null>;
  readonly listRef: RefObject<VListHandle | null>;
  readonly items: readonly TimelineItem[];
  readonly placement: string;
  readonly placed: boolean;
  readonly shift: boolean;
  readonly cardsSettled: boolean;
}) {
  const anchorRef = useRef<Anchor | null>(null);
  const settledRef = useRef(cardsSettled);

  const viewport = () => containerRef.current?.querySelector<HTMLElement>('[role="log"]');

  const capture = (atBottom: boolean) => {
    if (atBottom) {
      anchorRef.current = { kind: "end", placement };

      return;
    }

    const list = viewport();

    if (list === undefined || list === null) return;

    const bounds = list.getBoundingClientRect();
    let visible: Anchor | null = null;

    for (const row of list.querySelectorAll<HTMLElement>("[data-message-row][data-message-id]")) {
      const id = Number(row.dataset.messageId);

      if (!items.some((item) => item.kind === "message" && item.message.id === id)) continue;

      const rect = row.getBoundingClientRect();
      const wrapper = row.parentElement;

      if (wrapper === null || rect.bottom <= bounds.top || rect.top >= bounds.bottom) continue;

      const candidate: Anchor = {
        kind: "message",
        placement,
        id,
        top: rect.top - bounds.top,
        inset: rect.top - wrapper.getBoundingClientRect().top,
      };

      // A prepend can bring another row into view without the reader scrolling to it.
      if (
        anchorRef.current?.kind === "message" &&
        anchorRef.current.placement === placement &&
        anchorRef.current.id === id
      ) {
        visible = candidate;
        break;
      }

      // Prefer a row whose start is visible over the tail of the preceding message.
      if (rect.top >= bounds.top && (visible === null || visible.top < 0)) {
        visible = candidate;
      }

      visible ??= candidate;
    }

    if (visible !== null) anchorRef.current = visible;
  };

  useLayoutEffect(() => {
    const revealed = cardsSettled && !settledRef.current;

    settledRef.current = cardsSettled;

    const anchor = anchorRef.current;
    const list = listRef.current;

    if (!placed || (!shift && !revealed) || anchor === null || list === null) return;

    if (anchor.placement !== placement) return;

    if (anchor.kind === "end") {
      list.scrollToIndex(items.length - 1, { align: "end" });

      return;
    }

    const index = items.findIndex(
      (item) => item.kind === "message" && item.message.id === anchor.id,
    );

    if (index < 0) return;

    const row = viewport()?.querySelector<HTMLElement>(`[data-message-id="${anchor.id}"]`);
    const wrapper = row?.parentElement;

    const inset =
      row && wrapper
        ? row.getBoundingClientRect().top - wrapper.getBoundingClientRect().top
        : anchor.inset;

    // Virtua re-evaluates this target as ResizeObserver measures the growing rows. Its native
    // anchoring skips a partly visible preceding row once the prepend's scroll mode has ended.
    list.scrollToIndex(index, { align: "start", offset: inset - anchor.top });
  });

  return capture;
}
