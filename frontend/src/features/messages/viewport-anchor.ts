import {
  type RefObject,
  useCallback,
  useEffectEvent,
  useLayoutEffect,
  useMemo,
  useRef,
} from "react";
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

type PlacementState = { readonly placement: string } & (
  | { readonly kind: "placing-at-end" }
  | {
      readonly kind: "placing-at-target";
      readonly key: string;
      readonly align: "start" | "center";
      readonly offset: number;
    }
  | { readonly kind: "settled"; readonly messageId: number | null }
  | { readonly kind: "cancelled" }
);

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
  const placementRef = useRef<PlacementState | null>(null);

  const { indices, itemIndices } = useMemo(() => {
    const indices = new Map<number, number>();
    const itemIndices = new Map<string, number>();

    items.forEach((item, index) => {
      itemIndices.set(item.key, index);

      if (item.kind === "message") indices.set(item.message.id, index);
      else if (item.kind === "intro" && parentId !== null) indices.set(parentId, index);
    });

    return { indices, itemIndices };
  }, [items, parentId]);

  const viewport = () => containerRef.current?.querySelector<HTMLElement>('[role="log"]');

  const isPlacing = useCallback(() => {
    const state = placementRef.current;

    return (
      state?.placement === placement &&
      (state.kind === "placing-at-target" || state.kind === "placing-at-end")
    );
  }, [placement]);

  const capture = (atBottom: boolean) => {
    const state = placementRef.current;

    if (!placed || state?.placement !== placement || isPlacing()) return;

    let targetId = state.kind === "settled" ? state.messageId : null;

    if (targetId !== null && !indices.has(targetId)) {
      placementRef.current = { kind: "cancelled", placement };
      targetId = null;
    }

    const element = viewport();
    const corrected = correctedOffsetRef.current;

    if (corrected !== null && element && Math.abs(element.scrollTop - corrected) < 1) return;

    correctedOffsetRef.current = null;

    // A fitting placeholder window says nothing about permalink intent. Keep its placed
    // offset until reader input takes over, including scroll events caused by the reveal.
    if (
      targetId !== null &&
      anchorRef.current?.kind === "message" &&
      anchorRef.current.placement === placement &&
      anchorRef.current.id === targetId
    )
      return;

    if (atBottom && targetId === null) {
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

      if (!indices.has(id) || (targetId !== null && id !== targetId)) continue;

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
    if (placementRef.current?.placement !== placement || isPlacing()) return;

    const list = listRef.current;

    if (list !== null && list.viewportSize > 0 && anchorRef.current?.placement !== placement) {
      capture(list.scrollSize - list.scrollOffset - list.viewportSize < 40);
    }
  });

  const settle = () => {
    if (!placed || isPlacing()) return;

    if (placementRef.current?.placement !== placement)
      placementRef.current = { kind: "cancelled", placement };

    correctedOffsetRef.current = null;
    const element = viewport();

    if (element) {
      element.dataset.placementSettled = "true";
      capture(element.scrollHeight - element.scrollTop - element.clientHeight < 40);
    }
  };

  const place = (
    index: number,
    { align, offset = 0 }: { readonly align: "start" | "center" | "end"; readonly offset?: number },
  ) => {
    const item = items[index];

    if (item === undefined) return;

    placementRef.current =
      align === "end"
        ? { kind: "placing-at-end", placement }
        : { kind: "placing-at-target", placement, key: item.key, align, offset };
    anchorRef.current = null;

    const element = viewport();

    if (element) element.dataset.placementSettled = "false";
  };

  const cancelPlacement = useEffectEvent(() => {
    placementRef.current = { kind: "cancelled", placement };
    anchorRef.current = null;
    settle();
  });

  const finishPlacement = useEffectEvent(() => {
    const state = placementRef.current;

    if (state?.placement !== placement) return;

    if (state.kind === "placing-at-target") {
      const index = itemIndices.get(state.key);
      const item = index === undefined ? undefined : items[index];

      placementRef.current = {
        kind: "settled",
        placement,
        messageId: item?.kind === "message" ? item.message.id : null,
      };
    } else if (state.kind === "placing-at-end") {
      placementRef.current = { kind: "settled", placement, messageId: null };
    }

    settle();
  });

  const measurePlacement = useEffectEvent(() => {
    const target = placementRef.current;
    const list = listRef.current;
    const element = viewport();

    if (
      target?.placement !== placement ||
      (target.kind !== "placing-at-target" && target.kind !== "placing-at-end") ||
      list === null ||
      !element
    )
      return null;

    const index = target.kind === "placing-at-end" ? items.length - 1 : itemIndices.get(target.key);

    if (index === undefined || index < 0) {
      cancelPlacement();

      return null;
    }

    if (list.viewportSize === 0) return null;

    const size = list.getItemSize(index);

    const offset =
      target.kind === "placing-at-end"
        ? element.scrollHeight - element.clientHeight
        : list.getItemOffset(index) +
          (target.align === "center" ? (size - list.viewportSize) / 2 : 0) +
          target.offset;

    element.scrollTop = offset;

    const item = items[index];
    let selector: string | null = null;

    if (item?.kind === "message")
      selector = `[data-message-row][data-message-id="${item.message.id}"]`;
    else if (item?.kind === "unread") selector = ".unread-divider";

    const row = selector === null ? null : element.querySelector<HTMLElement>(selector);
    const wrapper = row && wrapperOf(row);

    const targetMeasured =
      selector === null ||
      (wrapper !== null && Math.abs(wrapper.getBoundingClientRect().height - size) <= 1);

    // Virtua hides unmeasured wrappers. Wait for their measurements and the native scroll
    // event, then for the target geometry to agree across frames, including a no-scroll list.
    const measured = Array.from(element.firstElementChild?.children ?? []).every(
      (wrapper) => wrapper instanceof HTMLElement && wrapper.style.visibility !== "hidden",
    );

    return targetMeasured && measured && Math.abs(list.scrollOffset - element.scrollTop) <= 1
      ? { offset: element.scrollTop, size, total: list.scrollSize, viewport: list.viewportSize }
      : null;
  });

  useLayoutEffect(() => {
    const state = placementRef.current;

    if (
      placed &&
      state?.placement === placement &&
      state.kind === "placing-at-target" &&
      !itemIndices.has(state.key)
    )
      cancelPlacement();
  }, [placed, placement, itemIndices]);

  useLayoutEffect(() => {
    if (!placed) return;

    const element = containerRef.current?.querySelector<HTMLElement>('[role="log"]');

    if (!element) return;

    let frame = 0;
    let previous: ReturnType<typeof measurePlacement> = null;

    const tick = () => {
      if (!isPlacing()) return;

      const current = measurePlacement();

      if (!isPlacing()) return;

      if (
        current !== null &&
        previous !== null &&
        current.offset === previous.offset &&
        current.size === previous.size &&
        current.total === previous.total &&
        current.viewport === previous.viewport
      ) {
        finishPlacement();

        return;
      }

      previous = current;
      frame = requestAnimationFrame(tick);
    };

    const takeControl = (event: Event) => {
      if (
        event instanceof KeyboardEvent &&
        !["ArrowUp", "ArrowDown", "PageUp", "PageDown", "Home", "End", " "].includes(event.key)
      )
        return;

      // Cancel our placement before the input scrolls. No Virtua imperative target remains
      // to reassert the initial position when another row measurement arrives.
      cancelAnimationFrame(frame);
      cancelPlacement();
    };

    const inputs = ["wheel", "touchstart", "keydown", "pointerdown"];

    for (const input of inputs)
      element.addEventListener(input, takeControl, { capture: true, passive: true });

    frame = requestAnimationFrame(tick);

    return () => {
      cancelAnimationFrame(frame);

      for (const input of inputs) element.removeEventListener(input, takeControl, true);

      if (placementRef.current?.placement === placement) placementRef.current = null;
    };
  }, [placed, placement, containerRef, isPlacing]);

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

    if (!placed || placementRef.current?.placement !== placement) placementRef.current = null;

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

        if (previous !== entry.contentRect.height) changed = true;
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

  return { capture, settle, place, isPlacing };
}
