import {
  type RefObject,
  useCallback,
  useEffectEvent,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import type { VListHandle } from "virtua";
import type { TimelineItem } from "../room/timeline-items.ts";
import { rowCommand } from "./keyboard.ts";

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
  | { readonly kind: "cancelled"; readonly allowEnd: boolean }
);

function wrapperOf(row: HTMLElement): HTMLElement | null {
  return row.closest(".thread-parent")?.parentElement ?? row.parentElement;
}

/** Keep a visible message through the cards reveal, and a bottom reader through later growth. */
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
  const finishCardsRef = useRef<(() => void) | null>(null);
  const correctedOffsetRef = useRef<number | null>(null);
  const scrollOffsetRef = useRef<number | null>(null);
  const followingFromRef = useRef<number | null>(null);
  const placementRef = useRef<PlacementState | null>(null);
  const [retainedId, setRetainedId] = useState<number | null>(null);

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

  const interacting = () =>
    Boolean(viewport()?.querySelector("[data-editing], .message-popup-anchor"));

  const isPlacing = useCallback(() => {
    const state = placementRef.current;

    return (
      state?.placement === placement &&
      (state.kind === "placing-at-target" || state.kind === "placing-at-end")
    );
  }, [placement]);

  const observeScroll = () => {
    const element = viewport();

    if (!element) return;

    const previous = scrollOffsetRef.current;
    const offset = element.scrollTop;
    const distance = element.scrollHeight - element.clientHeight - offset;
    const followingFrom = followingFromRef.current;

    scrollOffsetRef.current = offset;

    // Corrections and placement write a known offset. Following can also animate toward
    // the end; recognize that motion until it arrives, while allowing an away scroll.
    if (isPlacing()) return;

    if (correctedOffsetRef.current !== null && Math.abs(offset - correctedOffsetRef.current) <= 1) {
      if (distance <= 1) followingFromRef.current = null;

      return;
    }

    if (
      followingFrom !== null &&
      offset >= Math.max(followingFrom, previous ?? followingFrom) - 1
    ) {
      if (distance <= 1) followingFromRef.current = null;

      return;
    }

    if (previous === null || Math.abs(offset - previous) <= 1) return;

    followingFromRef.current = null;
    correctedOffsetRef.current = null;
    const state = placementRef.current;

    if (state?.placement !== placement) return;

    if (distance <= 1 && (state.kind !== "settled" || state.messageId === null)) {
      placementRef.current = { kind: "settled", placement, messageId: null };
      anchorRef.current = { kind: "end", placement };

      return;
    }

    // Ordinary row anchors already disable follow. Keep their pre-growth geometry
    // through Virtua's partial compensation so correction can supply the remainder.
    if (anchorRef.current?.placement === placement && anchorRef.current.kind === "message") return;

    // Find-in-page and native focus scrolling have no input event we can rely on.
    // A changed offset is reader movement; height growth at an unchanged offset is not.
    placementRef.current = {
      kind: "cancelled",
      placement,
      allowEnd: offset > previous || distance <= 1,
    };
    anchorRef.current = null;
  };

  const canFollow = () => {
    observeScroll();

    if (!placed || isPlacing() || interacting()) return false;

    const state = placementRef.current;

    if (
      state?.placement === placement &&
      state.kind === "settled" &&
      state.messageId !== null &&
      indices.has(state.messageId)
    )
      return false;

    // Layout growth can move the end without a scroll event. Follow the reader's captured
    // intent, rather than interpreting the enlarged previous end as reader movement.
    return anchorRef.current?.placement === placement && anchorRef.current.kind === "end";
  };

  const followEnd = () => {
    // Sending from a permalink explicitly takes the reader to the newest message.
    placementRef.current = { kind: "settled", placement, messageId: null };
    anchorRef.current = { kind: "end", placement };
    correctedOffsetRef.current = null;

    const element = viewport();

    if (element) element.dataset.placementSettled = "true";

    scrollOffsetRef.current = element?.scrollTop ?? null;
    followingFromRef.current = element?.scrollTop ?? null;
  };

  const capture = (preferredId: number | null = null) => {
    observeScroll();
    const state = placementRef.current;

    if (!placed || state?.placement !== placement || isPlacing()) return;

    let targetId = state.kind === "settled" ? state.messageId : null;

    if (targetId !== null && !indices.has(targetId)) {
      placementRef.current = { kind: "settled", placement, messageId: null };
      anchorRef.current = null;
      targetId = null;
    }

    const element = viewport();
    const corrected = correctedOffsetRef.current;

    if (corrected !== null && element && Math.abs(element.scrollTop - corrected) < 1) return;

    correctedOffsetRef.current = null;

    if (anchorRef.current?.kind === "message" && !indices.has(anchorRef.current.id))
      anchorRef.current = null;

    // Scroll and scroll-end callbacks also come from layout changes. Only reader input
    // takes over end or permalink intent; ordinary row anchors track continued scrolling.
    if (
      anchorRef.current?.placement === placement &&
      (anchorRef.current.kind === "end" ||
        (state.kind === "settled" &&
          anchorRef.current.kind === "message" &&
          anchorRef.current.id === targetId))
    )
      return;

    // A native scroll can arrive while Virtua's cached total still describes an earlier
    // layout. Capture from the viewport so layout changes do not discard end intent.
    const distance = element
      ? element.scrollHeight - element.scrollTop - element.clientHeight
      : Number.POSITIVE_INFINITY;

    // Small native keyboard scrolls start inside the follow threshold. Once input has
    // cancelled placement, keep their row rather than pulling them back to the end.
    const atEnd = state.kind === "cancelled" ? distance <= 1 : distance < 40;
    const allowEnd = state.kind !== "cancelled" || state.allowEnd;

    if (allowEnd && atEnd && targetId === null) {
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

      if (id === preferredId) {
        visible = candidate;
        break;
      }

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
      capture();
    }
  });

  const settle = (preferredId: number | null = null) => {
    if (!placed || isPlacing()) return;

    if (placementRef.current?.placement !== placement)
      placementRef.current = { kind: "cancelled", placement, allowEnd: true };

    correctedOffsetRef.current = null;
    const element = viewport();

    if (element) {
      element.dataset.placementSettled = "true";
      capture(preferredId);
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

  const cancelPlacement = useEffectEvent(
    (readerInput = false, allowEnd = true, preferredId: number | null = null) => {
      placementRef.current = readerInput
        ? { kind: "cancelled", placement, allowEnd }
        : { kind: "settled", placement, messageId: null };
      anchorRef.current = null;
      followingFromRef.current = null;
      scrollOffsetRef.current = viewport()?.scrollTop ?? null;
      settle(preferredId);
    },
  );

  const takeControl = () => cancelPlacement(true, false);

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
      anchorRef.current = { kind: "end", placement };
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
    scrollOffsetRef.current = element.scrollTop;
    correctedOffsetRef.current = element.scrollTop;

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
      ((state.kind === "placing-at-target" && !itemIndices.has(state.key)) ||
        (state.kind === "settled" && state.messageId !== null && !indices.has(state.messageId)))
    )
      cancelPlacement();
  }, [placed, placement, itemIndices, indices]);

  useLayoutEffect(() => {
    if (retainedId !== null && !indices.has(retainedId)) {
      setRetainedId(null);
      cancelPlacement();
    }
  }, [retainedId, indices]);

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

    let touchY: number | null = null;
    let middlePressed = false;

    let pointer: {
      readonly id: number;
      readonly y: number;
      readonly offset: number;
      allowEnd: boolean;
    } | null = null;

    const retainRow = (target: EventTarget | null) => {
      const row =
        target instanceof Element ? target.closest<HTMLElement>("[data-message-row]") : null;

      const id = Number(row?.dataset.messageId);

      if (!row || !Number.isFinite(id)) return false;

      // A prepend can briefly exclude this row from Virtua's visible range. Keep the same
      // DOM node while it owns focus, a popup, or an editor, including a right-click release.
      setRetainedId(id);
      const rows = element.querySelectorAll<HTMLElement>("[data-message-row][data-message-id]");

      // Focusing the newest row at the bottom (including End navigation) keeps follow.
      // Older rows surrender it before the focus scroll's event is delivered.
      if (
        row === rows[rows.length - 1] &&
        element.scrollHeight - element.clientHeight - element.scrollTop <= 1
      )
        return true;

      cancelAnimationFrame(frame);
      cancelPlacement(true, false, id);

      return true;
    };

    const onFocus = (event: Event) => {
      retainRow(event.target);
    };

    const onInput = (event: Event) => {
      let away = false;

      if (event instanceof KeyboardEvent) {
        if (!(event.target instanceof HTMLElement)) return;

        const command = event.target.matches("[data-message-row]") ? rowCommand(event) : null;
        const navigates = command !== null && ["up", "down", "first", "last"].includes(command);

        if (
          (!["ArrowUp", "ArrowDown", "PageUp", "PageDown", "Home", "End", " "].includes(
            event.key,
          ) &&
            !navigates) ||
          (event.target !== element && event.target.closest("[data-message-row]") === null) ||
          event.target.isContentEditable ||
          event.target.closest('input, textarea, select, video, audio, [role="textbox"]') !==
            null ||
          (event.key === " " && event.target.closest('button, [role="button"], summary') !== null)
        )
          return;

        away =
          command === "up" ||
          command === "first" ||
          ["ArrowUp", "PageUp", "Home"].includes(event.key) ||
          (event.key === " " && event.shiftKey);
      } else if (typeof WheelEvent !== "undefined" && event instanceof WheelEvent) {
        if (event.deltaY === 0 || event.ctrlKey) return;

        away = event.deltaY < 0;
      } else if (typeof TouchEvent !== "undefined" && event instanceof TouchEvent) {
        touchY = event.touches[0]?.clientY ?? null;
      } else if (event instanceof MouseEvent && event.button === 1) {
        if (event.type === "auxclick" && middlePressed) {
          middlePressed = false;

          return;
        }

        if (event.type === "mousedown" && middlePressed) return;

        middlePressed = event.type !== "auxclick";
        away = true;
      } else if (typeof PointerEvent !== "undefined" && event instanceof PointerEvent) {
        if (retainRow(event.target)) return;

        const bounds = element.getBoundingClientRect();
        const gutter = Math.max(16, element.offsetWidth - element.clientWidth);

        if (
          event.button !== 0 ||
          event.target !== element ||
          event.clientX < bounds.right - gutter ||
          element.scrollHeight <= element.clientHeight
        )
          return;

        const thumb = Math.max(20, element.clientHeight ** 2 / element.scrollHeight);

        const thumbTop =
          ((element.clientHeight - thumb) * element.scrollTop) /
          (element.scrollHeight - element.clientHeight);

        pointer =
          event.clientY >= bounds.top + thumbTop && event.clientY <= bounds.top + thumbTop + thumb
            ? { id: event.pointerId, y: event.clientY, offset: element.scrollTop, allowEnd: false }
            : null;

        // Native scrollbar drags suppress DOM pointermove. Relinquish end intent on
        // the thumb, then retarget from the release coordinates once the drag finishes.
        away = event.clientY <= bounds.top + thumbTop + thumb;
      } else return;

      // Cancel our placement before the input scrolls. No Virtua imperative target remains
      // to reassert the initial position when another row measurement arrives.
      cancelAnimationFrame(frame);
      // An away gesture must relinquish end intent before its first scroll event.
      cancelPlacement(true, !away);
    };

    const onMove = (event: Event) => {
      if (typeof TouchEvent !== "undefined" && event instanceof TouchEvent && touchY !== null) {
        const y = event.touches[0]?.clientY;

        if (y === undefined || y === touchY) return;

        cancelPlacement(true, y < touchY);
        touchY = y;
      } else if (
        typeof PointerEvent !== "undefined" &&
        event instanceof PointerEvent &&
        pointer !== null &&
        event.pointerId === pointer.id
      ) {
        if (event.buttons === 0) {
          pointer = null;

          return;
        }

        const y = event.clientY;

        if (y === pointer.y) return;

        const allowEnd = y > pointer.y;

        if (allowEnd !== pointer.allowEnd) {
          pointer.allowEnd = allowEnd;
          cancelPlacement(true, allowEnd);
        }
      }
    };

    const onRelease = (event: Event) => {
      if (typeof TouchEvent !== "undefined" && event instanceof TouchEvent) {
        touchY = null;

        return;
      }

      if (
        typeof PointerEvent === "undefined" ||
        !(event instanceof PointerEvent) ||
        pointer === null ||
        event.pointerId !== pointer.id
      )
        return;

      if (event.type === "pointerup") {
        const allowEnd =
          event.clientY > pointer.y ||
          (event.clientY === pointer.y &&
            (element.scrollTop > pointer.offset ||
              (element.scrollTop === pointer.offset &&
                element.scrollHeight - element.clientHeight - element.scrollTop <= 1)));

        if (allowEnd !== pointer.allowEnd) cancelPlacement(true, allowEnd);
      }

      pointer = null;
    };

    const inputs = ["wheel", "touchstart", "keydown", "pointerdown", "mousedown", "auxclick"];

    for (const input of inputs)
      element.addEventListener(input, onInput, { capture: true, passive: true });

    element.addEventListener("focusin", onFocus, true);
    element.addEventListener("contextmenu", onFocus, true);

    element.addEventListener("touchmove", onMove, { capture: true, passive: true });
    element.ownerDocument.addEventListener("pointermove", onMove, { capture: true, passive: true });

    for (const release of ["touchend", "touchcancel"])
      element.addEventListener(release, onRelease, { capture: true, passive: true });

    for (const release of ["pointerup", "pointercancel"])
      element.ownerDocument.addEventListener(release, onRelease, { capture: true, passive: true });

    frame = requestAnimationFrame(tick);

    return () => {
      cancelAnimationFrame(frame);

      for (const input of inputs) element.removeEventListener(input, onInput, true);

      element.removeEventListener("focusin", onFocus, true);
      element.removeEventListener("contextmenu", onFocus, true);

      element.removeEventListener("touchmove", onMove, true);
      element.ownerDocument.removeEventListener("pointermove", onMove, true);

      for (const release of ["touchend", "touchcancel"])
        element.removeEventListener(release, onRelease, true);

      for (const release of ["pointerup", "pointercancel"])
        element.ownerDocument.removeEventListener(release, onRelease, true);

      if (placementRef.current?.placement === placement) placementRef.current = null;
    };
  }, [placed, placement, containerRef, isPlacing]);

  const chunkLoaded = useEffectEvent(() => cardsLoaded);

  const correct = useEffectEvent(() => {
    observeScroll();
    const anchor = anchorRef.current;
    const list = listRef.current;
    const element = viewport();

    if (
      anchor === null ||
      anchor.placement !== placement ||
      list === null ||
      !element ||
      interacting()
    )
      return;

    if (anchor.kind === "end") {
      const offset = element.scrollHeight - element.clientHeight;

      if (Math.abs(offset - element.scrollTop) > 1) {
        element.scrollTop = offset;
        correctedOffsetRef.current = element.scrollTop;
        scrollOffsetRef.current = element.scrollTop;
        followingFromRef.current = null;
      }

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

    if (Math.abs(delta) > 1) {
      const before = element.scrollTop;

      element.scrollTop += delta;

      if (element.scrollTop !== before) {
        correctedOffsetRef.current = element.scrollTop;
        scrollOffsetRef.current = element.scrollTop;
      }
    }
  });

  useLayoutEffect(() => {
    if (!placed || anchorRef.current?.placement !== placement) anchorRef.current = null;

    if (!placed || placementRef.current?.placement !== placement) placementRef.current = null;

    correctedOffsetRef.current = null;
    scrollOffsetRef.current = null;
    followingFromRef.current = null;

    if (!placed) return;

    const element = containerRef.current?.querySelector<HTMLElement>('[role="log"]');
    const content = element?.firstElementChild;

    if (!element || !content) return;

    const heights = new Map<Element, number>();
    let correctingCards = !chunkLoaded();

    const resizes = new ResizeObserver((entries) => {
      seed();

      let changed = false;

      for (const entry of entries) {
        const previous = heights.get(entry.target);

        heights.set(entry.target, entry.contentRect.height);

        // Remounting a cached row during reader scrolling is not a height change.
        // Only the cards reveal can make a first delivery a change worth correcting.
        if (
          previous !== entry.contentRect.height &&
          (previous !== undefined || (correctingCards && chunkLoaded())) &&
          (entry.target !== content || anchorRef.current?.kind === "end")
        )
          changed = true;
      }

      // Wrapper and content measurements can arrive separately. Correct the remaining
      // distance after Virtua updates the content height as well as when a row grows.
      if (changed && (correctingCards || anchorRef.current?.kind === "end")) correct();

      if (correctingCards && chunkLoaded()) {
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

        if (measured) finishCardsRef.current?.();
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

    const finishCards = () => {
      // A delayed image or font can still grow a bottom reader's rows after the chunk.
      // Keep observing for that intent; message-position correction ends with the reveal.
      correctingCards = false;
      finishCardsRef.current = null;
      mutations.disconnect();
      mutations.observe(content, { childList: true });
    };

    const stop = () => {
      resizes.disconnect();
      mutations.disconnect();
      finishCardsRef.current = null;
    };

    finishCardsRef.current = finishCards;
    resizes.observe(element);
    resizes.observe(content);
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
      finishCardsRef.current?.();
  });

  const retainedIndex = retainedId === null ? undefined : indices.get(retainedId);
  const keepMounted = retainedIndex === undefined ? [] : [retainedIndex];

  return { capture, settle, place, isPlacing, canFollow, followEnd, takeControl, keepMounted };
}
