import { type RefObject, useEffectEvent, useLayoutEffect, useMemo, useRef } from "react";
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

function wrapperOf(row: HTMLElement): HTMLElement | null {
  return row.closest(".thread-parent")?.parentElement ?? row.parentElement;
}

/** Keep a visible message in place when the cards chunk grows rows above it. */
export function useViewportAnchor({
  containerRef,
  listRef,
  items,
  placement,
  placed,
  cardsLoaded,
  parentId = null,
}: {
  readonly containerRef: RefObject<HTMLElement | null>;
  readonly listRef: RefObject<VListHandle | null>;
  readonly items: readonly TimelineItem[];
  readonly placement: string;
  readonly placed: boolean;
  readonly cardsLoaded: boolean;
  readonly parentId?: number | null;
}) {
  const anchorRef = useRef<Anchor | null>(null);
  const stopRef = useRef<(() => void) | null>(null);
  const correctedOffsetRef = useRef<number | null>(null);

  const indices = useMemo(() => {
    const map = new Map<number, number>();

    items.forEach((item, index) => {
      if (item.kind === "message") map.set(item.message.id, index);
      else if (item.kind === "intro" && parentId !== null) map.set(parentId, index);
    });

    return map;
  }, [items, parentId]);

  const viewport = () => containerRef.current?.querySelector<HTMLElement>('[role="log"]');

  const capture = (atBottom: boolean) => {
    const element = viewport();
    const corrected = correctedOffsetRef.current;

    if (corrected !== null && element && Math.abs(element.scrollTop - corrected) < 1) return;

    correctedOffsetRef.current = null;

    if (atBottom) {
      anchorRef.current = { kind: "end", placement };

      return;
    }

    const list = viewport();

    if (list === undefined || list === null) {
      anchorRef.current = null;

      return;
    }

    const bounds = list.getBoundingClientRect();
    let visible: Anchor | null = null;

    for (const row of list.querySelectorAll<HTMLElement>("[data-message-row][data-message-id]")) {
      const id = Number(row.dataset.messageId);

      if (!indices.has(id)) continue;

      const rect = row.getBoundingClientRect();
      const wrapper = wrapperOf(row);

      if (
        wrapper === null ||
        wrapper.style.visibility === "hidden" ||
        rect.bottom <= bounds.top ||
        rect.top >= bounds.bottom
      )
        continue;

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

    anchorRef.current = visible;
  };

  const seed = useEffectEvent(() => {
    const list = listRef.current;

    if (list !== null && list.viewportSize > 0 && anchorRef.current?.placement !== placement) {
      capture(list.scrollSize - list.scrollOffset - list.viewportSize < 40);
    }
  });

  const chunkLoaded = useEffectEvent(() => cardsLoaded);

  const correct = useEffectEvent(() => {
    const anchor = anchorRef.current;
    const list = listRef.current;
    const element = viewport();

    if (anchor === null || anchor.placement !== placement || list === null || !element) return;

    if (anchor.kind === "end") {
      element.scrollTop = element.scrollHeight - element.clientHeight;
      correctedOffsetRef.current = element.scrollTop;

      return;
    }

    const index = indices.get(anchor.id);

    if (index === undefined) return;

    const row = element.querySelector<HTMLElement>(`[data-message-id="${anchor.id}"]`);

    const top = row
      ? row.getBoundingClientRect().top - element.getBoundingClientRect().top
      : list.getItemOffset(index) + anchor.inset - element.scrollTop;

    const delta = top - anchor.top;

    // Virtua already compensates fully hidden rows and prepends. Apply only the remainder,
    // without starting a scrollToIndex operation that can later override the reader's scrolling.

    if (Math.abs(delta) > 1) element.scrollTop += delta;

    correctedOffsetRef.current = element.scrollTop;
  });

  useLayoutEffect(() => {
    if (!placed || anchorRef.current?.placement !== placement) anchorRef.current = null;

    correctedOffsetRef.current = null;

    if (!placed || chunkLoaded()) return;

    const element = containerRef.current?.querySelector<HTMLElement>('[role="log"]');
    const content = element?.firstElementChild;

    if (!element || !content) return;

    const heights = new Map<Element, number>();

    const resizes = new ResizeObserver((entries) => {
      seed();

      let changed = false;

      for (const entry of entries) {
        const previous = heights.get(entry.target);

        heights.set(entry.target, entry.contentRect.height);

        if (previous !== undefined && Math.abs(previous - entry.contentRect.height) > 1)
          changed = true;
      }

      // Virtua observes these wrappers first and applies its measurements and native jump
      // synchronously. Correct the remainder before its resulting scroll event captures it.
      if (changed) correct();

      if (chunkLoaded()) {
        // The reveal can span resize deliveries; wait until every rendered card wrapper has
        // delivered its current height before ending the correction.
        const cards = content.querySelectorAll<HTMLElement>(".message-cards:not(:empty)");

        const measured = Array.from(cards).every((card) => {
          const row = card.closest<HTMLElement>("[data-message-row]");
          const wrapper = row && wrapperOf(row);

          return (
            wrapper !== null &&
            wrapper !== undefined &&
            Math.abs((heights.get(wrapper) ?? -1) - wrapper.getBoundingClientRect().height) < 1
          );
        });

        if (measured) stopRef.current?.();
      }
    });

    const observed = new Set<Element>();

    const observeRows = () => {
      for (const row of observed) {
        if (row.parentElement !== content) {
          resizes.unobserve(row);
          observed.delete(row);
          heights.delete(row);
        }
      }

      for (const row of content.children) {
        if (!observed.has(row)) {
          observed.add(row);

          if (!chunkLoaded()) heights.set(row, row.getBoundingClientRect().height);

          resizes.observe(row);
        }
      }

      seed();
    };

    const mutations = new MutationObserver(observeRows);

    const stop = () => {
      resizes.disconnect();
      mutations.disconnect();
      stopRef.current = null;
    };

    stopRef.current = stop;
    resizes.observe(element);
    mutations.observe(content, {
      childList: true,
      subtree: true,
      attributes: true,
      attributeFilter: ["style"],
    });
    observeRows();

    return stop;
  }, [placed, placement, containerRef]);

  useLayoutEffect(() => {
    // A successful chunk can reveal nothing in this timeline, including suppressed cards.
    if (cardsLoaded && !viewport()?.querySelector(".message-cards:not(:empty)"))
      stopRef.current?.();
  });

  return capture;
}
