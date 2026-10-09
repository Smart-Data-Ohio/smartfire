import {
  type RefObject,
  useCallback,
  useEffectEvent,
  useInsertionEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import type { VListHandle } from "virtua";
import type { TimelineItem } from "../room/timeline-items.ts";

type Anchor =
  | {
      readonly placement: string;
      readonly kind: "intro";
      readonly scroll: number;
    }
  | {
      readonly placement: string;
      readonly kind: "end";
      readonly scroll: number;
      readonly row: {
        readonly id: number;
        readonly offset: number;
      } | null;
    }
  | {
      readonly placement: string;
      readonly kind: "message";
      readonly id: number;
      readonly top: number;
      readonly inset: number;
    };

type PlacementState = { readonly placement: string } & (
  | { readonly kind: "placing-at-end"; readonly follow: boolean }
  | {
      readonly kind: "placing-at-target";
      readonly key: string;
      readonly align: "start" | "center";
      readonly offset: number;
      readonly follow: boolean;
    }
  | { readonly kind: "settled"; readonly messageId: number | null }
  | {
      readonly kind: "cancelled";
      readonly allowEnd: boolean;
      readonly awaitEndInput: boolean;
    }
);

function wrapperOf(row: HTMLElement): HTMLElement | null {
  return row.closest(".thread-parent")?.parentElement ?? row.parentElement;
}

function endOffset(element: HTMLElement): number {
  return Math.max(0, element.scrollHeight - element.clientHeight);
}

function followsMotion(scroll: number, end: number, reference: number): boolean {
  // Virtua premeasures the destination before native smooth scrolling starts.
  // Follow toward the live end, while rejecting reversals and overshoot.
  return scroll >= Math.max(0, Math.min(end, reference)) - 1 && scroll <= end + 1;
}

function hasInteraction(element: HTMLElement | null | undefined): boolean {
  return Boolean(
    element?.querySelector("[data-editing], [data-popup-pending], .message-popup-anchor"),
  );
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
  onTakeControl,
}: {
  readonly containerRef: RefObject<HTMLElement | null>;
  readonly listRef: RefObject<VListHandle | null>;
  readonly items: readonly TimelineItem[];
  readonly placement: string;
  readonly placed: boolean;
  readonly cardsLoaded: boolean;
  readonly parentId?: number | null;
  /** Real reader input. Layout scrolls do not call this. */
  readonly onTakeControl?: () => void;
}) {
  const anchorRef = useRef<Anchor | null>(null);
  const finishCardsRef = useRef<(() => void) | null>(null);
  const correctedOffsetRef = useRef<number | null>(null);

  const endMotionRef = useRef(false);

  const correctionPendingRef = useRef(false);
  const retainedIdRef = useRef<number | null>(null);
  const retentionPendingRef = useRef<number | null>(null);
  const placementRef = useRef<PlacementState | null>(null);
  const beforeResizesRef = useRef<ResizeObserver | null>(null);

  const committedItemsRef = useRef<{
    readonly placement: string;
    readonly indices: ReadonlyMap<number, number>;
    readonly itemIndices: ReadonlyMap<string, number>;
  } | null>(null);

  const measurementRef = useRef<{
    readonly anchor: Extract<Anchor, { kind: "end" }>;
    readonly before: number;
    readonly expected: number;
  } | null>(null);

  const settlementFrameRef = useRef(0);
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

  // Render still sees the committed DOM. A deletion's native clamp can erase
  // eventless reader movement before any layout effect gets to inspect it.
  const removalSnapshot = useMemo(() => {
    const previous = committedItemsRef.current;
    const anchor = anchorRef.current;

    if (
      !placed ||
      previous?.placement !== placement ||
      anchor?.placement !== placement ||
      anchor.kind !== "end" ||
      (!Array.from(previous.itemIndices.keys()).some((key) => !itemIndices.has(key)) &&
        !Array.from(previous.indices.keys()).some((id) => !indices.has(id)))
    )
      return null;

    const element = containerRef.current?.querySelector<HTMLElement>('[role="log"]');

    if (!element) return { anchor, follows: null, allowEnd: null };

    const { scrollTop } = element;
    const end = endOffset(element);
    const paused = hasInteraction(element) || correctionPendingRef.current;
    const index = anchor.row ? previous.indices.get(anchor.row.id) : undefined;
    const offset = index !== undefined ? listRef.current?.getItemOffset(index) : undefined;
    const compensation = anchor.row && offset !== undefined ? offset - anchor.row.offset : 0;

    const atEnd = end - scrollTop <= 1;
    const reference = anchor.scroll + compensation;
    const expected = paused ? Math.max(0, Math.min(end, reference)) : reference;
    const continuous = (!anchor.row || offset !== undefined) && Math.abs(scrollTop - expected) <= 1;
    const motion = endMotionRef.current;

    return {
      anchor,
      follows: motion
        ? (!anchor.row || offset !== undefined) && followsMotion(scrollTop, end, reference)
        : continuous || (!paused && atEnd),
      allowEnd: atEnd || (Math.abs(compensation) > 1 && continuous),
    };
  }, [indices, itemIndices, placed, placement, containerRef, listRef]);

  const interacting = useEffectEvent(() => hasInteraction(viewport()));

  const isPlacing = useCallback(() => {
    const state = placementRef.current;

    return (
      state?.placement === placement &&
      (state.kind === "placing-at-target" || state.kind === "placing-at-end")
    );
  }, [placement]);

  const endRow = (offset: number) => {
    const element = viewport();

    if (!element) return null;

    const bounds = element.getBoundingClientRect();

    for (const [id, index] of indices) {
      const row = element.querySelector<HTMLElement>(`[data-message-id="${id}"]`);
      const wrapper = row && wrapperOf(row);

      if (wrapper?.style.visibility === "hidden") continue;

      const rect = row?.getBoundingClientRect();
      const inset = rect && wrapper ? rect.top - wrapper.getBoundingClientRect().top : 0;

      const top = rect
        ? rect.top - bounds.top + element.scrollTop - offset
        : (listRef.current?.getItemOffset(index) ?? 0) + inset - offset;

      const height = rect?.height ?? listRef.current?.getItemSize(index) ?? 0;

      if (top + height > 0 && top < element.clientHeight)
        return { id, offset: listRef.current?.getItemOffset(index) ?? 0 };
    }

    return null;
  };

  const pinEnd = (offset?: number) => {
    const element = viewport();

    if (!element) return;

    endMotionRef.current = false;
    placementRef.current = { kind: "settled", placement, messageId: null };
    const scroll = offset ?? element.scrollTop;

    anchorRef.current = { kind: "end", placement, scroll, row: endRow(scroll) };
  };

  const checkFollow = useEffectEvent(() => {
    if (anchorRef.current?.placement !== placement || anchorRef.current.kind !== "end")
      return false;

    const element = viewport();

    if (!element) return false;

    if (settlementFrameRef.current !== 0) return false;

    const atEnd = endOffset(element) - element.scrollTop <= 1;
    const paused = interacting() || correctionPendingRef.current;
    const motion = endMotionRef.current;

    if (motion) {
      const anchor = anchorRef.current;
      const index = anchor.row ? indices.get(anchor.row.id) : undefined;
      const offset = index !== undefined ? listRef.current?.getItemOffset(index) : undefined;
      const compensation = anchor.row && offset !== undefined ? offset - anchor.row.offset : 0;

      if (
        (anchor.row && offset === undefined) ||
        !followsMotion(element.scrollTop, endOffset(element), anchor.scroll + compensation)
      ) {
        cancelPlacement(true, false, null, true);

        return false;
      }

      if (atEnd) pinEnd();
      else {
        anchorRef.current = {
          ...anchor,
          scroll: element.scrollTop,
          row: endRow(element.scrollTop),
        };
      }

      return true;
    }

    if (atEnd && !paused) {
      pinEnd();

      return true;
    }

    const witness = anchorRef.current.row;
    const index = witness === null ? undefined : indices.get(witness.id);

    if (witness !== null && (index === undefined || listRef.current === null)) {
      cancelPlacement(true, !paused);

      return false;
    }

    // Rows above the first visible witness are fully hidden and compensated by Virtua.
    // Its own compensation is recorded across each measurement batch below.
    // An empty list still has an offset reference while its first row is being measured.
    const reference =
      anchorRef.current.scroll +
      (witness && index !== undefined
        ? (listRef.current?.getItemOffset(index) ?? witness.offset) - witness.offset
        : 0);

    // A paused pin can be clamped by a native layout shrink, but cannot adopt movement.
    const expected = paused ? Math.max(0, Math.min(endOffset(element), reference)) : reference;

    if (Math.abs(element.scrollTop - expected) > 1) {
      cancelPlacement(true, !paused);

      return false;
    }

    if (atEnd) pinEnd(expected);

    return true;
  });

  const beforeMeasure = useEffectEvent(() => {
    measurementRef.current = null;

    if (!checkFollow()) return;

    const anchor = anchorRef.current;
    const element = viewport();
    const list = listRef.current;
    const index = anchor?.kind === "end" && anchor.row ? indices.get(anchor.row.id) : undefined;

    if (anchor?.kind !== "end" || !element || !list) return;

    measurementRef.current = {
      anchor,
      before: element.scrollTop,
      expected:
        anchor.scroll +
        (anchor.row && index !== undefined ? list.getItemOffset(index) - anchor.row.offset : 0),
    };
  });

  const afterMeasure = useEffectEvent(() => {
    const measurement = measurementRef.current;
    measurementRef.current = null;
    const anchor = anchorRef.current;
    const list = listRef.current;
    const element = viewport();
    const index = anchor?.kind === "end" && anchor.row ? indices.get(anchor.row.id) : undefined;

    if (measurement?.anchor !== anchor || anchor?.kind !== "end" || !list || !element) return;

    if (anchor.row && index === undefined) {
      cancelPlacement(true, false);

      return;
    }

    // No browser input runs between these observer callbacks. Include every actual
    // Virtua scroll change, including compensation of a partly visible witness.
    anchorRef.current = {
      ...anchor,
      scroll: measurement.expected + element.scrollTop - measurement.before,
      row:
        anchor.row && index !== undefined
          ? { ...anchor.row, offset: list.getItemOffset(index) }
          : null,
    };
  });

  // Virtua creates its observer in child layout effects. Construct this one earlier,
  // then attach both observers in our layout effect to bracket its synchronous batch.
  // Keep its construction order across placement changes in the same mounted list.
  useInsertionEffect(() => {
    const observer = new ResizeObserver(beforeMeasure);

    beforeResizesRef.current = observer;

    return () => {
      observer.disconnect();
      beforeResizesRef.current = null;
    };
  }, []);

  const validateRemoval = useEffectEvent(() => {
    const anchor = anchorRef.current;

    if (!placed || anchor?.placement !== placement || anchor.kind !== "end") return;

    const paused = interacting() || correctionPendingRef.current;

    if (
      removalSnapshot?.anchor === anchor &&
      (removalSnapshot.follows === false || (removalSnapshot.follows === null && paused))
    ) {
      // Delayed native scroll and settlement callbacks cannot rearm this new end.
      cancelPlacement(true, false, null, true);

      return;
    }

    if (removalSnapshot?.anchor === anchor && endMotionRef.current) {
      const element = viewport();

      if (element && removalSnapshot.follows === true) {
        anchorRef.current = {
          ...anchor,
          scroll: element.scrollTop,
          row: endRow(element.scrollTop),
        };

        return;
      }
    }

    // Without a surviving witness, a pause still cannot acquire a replacement pin.
    if (anchor.row && !indices.has(anchor.row.id)) {
      const allowEnd =
        !paused && (removalSnapshot?.anchor !== anchor || removalSnapshot.allowEnd !== false);

      cancelPlacement(true, allowEnd, null, !allowEnd);
    }
  });

  useLayoutEffect(() => {
    validateRemoval();

    // A popup can mount after native arrival but before its scroll callback. Finish
    // that motion against the live end before later growth freezes the pause geometry.
    if (endMotionRef.current && interacting()) checkFollow();

    committedItemsRef.current = { placement, indices, itemIndices };
  });

  const canFollow = () => {
    if (!placed || isPlacing() || interacting()) return false;

    const element = viewport();

    if (element && endOffset(element) - element.scrollTop <= 1) capture(null, true);

    const state = placementRef.current;

    if (
      state?.placement === placement &&
      state.kind === "settled" &&
      state.messageId !== null &&
      indices.has(state.messageId)
    )
      return false;

    return checkFollow();
  };

  const followEnd = () => {
    // Sending from a permalink explicitly takes the reader to the newest message.
    const element = viewport();

    pinEnd();
    correctedOffsetRef.current = null;

    if (element) {
      element.dataset.placementSettled = "true";

      const end = endOffset(element);

      if (end - element.scrollTop > 1) endMotionRef.current = true;
    }
  };

  const capture = (preferredId: number | null = null, readerInput = false) => {
    const state = placementRef.current;

    if (!placed || state?.placement !== placement || isPlacing()) return;

    // Layout scrolls cannot replace the post's work with a visible reply anchor.
    // Reader takeover and sending clear this hold before normal capture resumes.
    if (anchorRef.current?.placement === placement && anchorRef.current.kind === "intro") return;

    let targetId = state.kind === "settled" ? state.messageId : null;

    if (targetId !== null && !indices.has(targetId)) {
      placementRef.current = { kind: "settled", placement, messageId: null };
      anchorRef.current = null;
      targetId = null;
    }

    const element = viewport();

    if (endMotionRef.current && !checkFollow()) return;

    // Focus restoration and layout scrolls during a popup/editor preserve its pause geometry.
    // Explicit reader input clears the anchor in cancelPlacement before capturing again.
    if (interacting() && anchorRef.current?.placement === placement) return;

    if (
      element &&
      targetId === null &&
      endOffset(element) - element.scrollTop <= 1 &&
      (state.kind !== "cancelled" || state.allowEnd || (!readerInput && !state.awaitEndInput))
    ) {
      if (anchorRef.current?.kind === "end" && correctionPendingRef.current && !checkFollow())
        return;

      pinEnd();

      return;
    }

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

  const settle = (preferredId: number | null = null, readerInput = false, afterCommit = false) => {
    if (!placed || isPlacing()) return;

    const element = viewport();
    const anchor = anchorRef.current;
    const list = listRef.current;
    const index = anchor?.kind === "end" && anchor.row ? indices.get(anchor.row.id) : undefined;

    if (
      !readerInput &&
      !afterCommit &&
      correctionPendingRef.current &&
      anchor?.kind === "end" &&
      anchor.row &&
      element &&
      list &&
      index !== undefined &&
      endOffset(element) - element.scrollTop > 1 &&
      Math.abs(element.scrollTop - anchor.scroll) <= 1 &&
      Math.abs(list.getItemOffset(index) - anchor.row.offset) > 1
    ) {
      // Virtua exposes a queued jump in its offsets before the scroll-end layout
      // commit applies it (iOS momentum and smooth scrolling). Check after that commit.
      cancelAnimationFrame(settlementFrameRef.current);
      settlementFrameRef.current = requestAnimationFrame(() => {
        settlementFrameRef.current = 0;
        settleAfterCommit(preferredId);
      });

      return;
    }

    const endMotion = endMotionRef.current;

    if (endMotion && !checkFollow()) return;

    endMotionRef.current = false;

    // Virtua can premeasure a taller destination after motion is issued. Only arrival
    // at the live end keeps follow; a stop short relinquishes it in either direction.
    if (!readerInput && endMotion && element && endOffset(element) - element.scrollTop > 1) {
      cancelPlacement(true);

      return;
    }

    if (placementRef.current?.placement !== placement)
      placementRef.current = { kind: "cancelled", placement, allowEnd: true, awaitEndInput: false };

    if (!readerInput && !interacting()) {
      checkFollow();

      if (correctionPendingRef.current) correct();
    }

    correctedOffsetRef.current = null;

    if (element) {
      element.dataset.placementSettled = "true";
      capture(preferredId, readerInput);
    }
  };

  const settleAfterCommit = useEffectEvent((preferredId: number | null) =>
    settle(preferredId, false, true),
  );

  const place = (
    index: number,
    {
      align,
      offset = 0,
      follow = true,
    }: {
      readonly align: "start" | "center" | "end";
      readonly offset?: number;
      readonly follow?: boolean;
    },
  ) => {
    const item = items[index];

    if (item === undefined) return;

    placementRef.current =
      align === "end"
        ? { kind: "placing-at-end", placement, follow }
        : { kind: "placing-at-target", placement, key: item.key, align, offset, follow };
    anchorRef.current = null;

    const element = viewport();

    if (element) element.dataset.placementSettled = "false";
  };

  const cancelPlacement = useEffectEvent(
    (
      readerInput = false,
      allowEnd = true,
      preferredId: number | null = null,
      awaitEndInput = false,
    ) => {
      cancelAnimationFrame(settlementFrameRef.current);
      settlementFrameRef.current = 0;
      placementRef.current = readerInput
        ? { kind: "cancelled", placement, allowEnd, awaitEndInput }
        : { kind: "settled", placement, messageId: null };
      anchorRef.current = null;
      endMotionRef.current = false;
      correctedOffsetRef.current = null;
      settle(preferredId, readerInput);
    },
  );

  const noteReaderInput = useEffectEvent(() => {
    onTakeControl?.();
  });

  const takeControl = (allowEnd = false) => {
    noteReaderInput();
    cancelPlacement(true, allowEnd);
  };

  const finishPlacement = useEffectEvent(() => {
    const state = placementRef.current;

    if (state?.placement !== placement) return;

    if ((state.kind === "placing-at-target" || state.kind === "placing-at-end") && !state.follow) {
      // A post's intro stays put even when it fits; only reader input or a send can follow.
      placementRef.current = {
        kind: "cancelled",
        placement,
        allowEnd: false,
        awaitEndInput: true,
      };

      const element = viewport();

      if (
        state.kind === "placing-at-target" &&
        state.align === "start" &&
        items[itemIndices.get(state.key) ?? -1]?.kind === "intro" &&
        element
      )
        anchorRef.current = { kind: "intro", placement, scroll: element.scrollTop };
    } else if (state.kind === "placing-at-target") {
      const index = itemIndices.get(state.key);
      const item = index === undefined ? undefined : items[index];

      placementRef.current = {
        kind: "settled",
        placement,
        messageId: item?.kind === "message" ? item.message.id : null,
      };
    } else if (state.kind === "placing-at-end") {
      pinEnd();
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

  const isNewest = useEffectEvent(
    (id: number) => indices.get(id) === Math.max(...indices.values()),
  );

  const onNativeScrollEnd = useEffectEvent(() => settle());

  const releaseRetention = useEffectEvent(
    (focused: EventTarget | null = document.activeElement) => {
      const id = retainedIdRef.current;

      if (id === null) return;

      const row = viewport()?.querySelector<HTMLElement>(`[data-message-id="${id}"]`);

      if (row?.matches("[data-popup-pending]")) return;

      if (row && (row.matches("[data-editing]") || row.querySelector(".message-popup-anchor"))) {
        retentionPendingRef.current = null;

        return;
      }

      if (
        retentionPendingRef.current === id ||
        (row && focused instanceof Node && row.contains(focused))
      )
        return;

      retainedIdRef.current = null;
      setRetainedId(null);
    },
  );

  useLayoutEffect(() => {
    if (retainedId !== null && !indices.has(retainedId)) {
      retainedIdRef.current = null;
      retentionPendingRef.current = null;
      setRetainedId(null);
    }
  }, [retainedId, indices]);

  useLayoutEffect(() => {
    if (!placed) return;

    const element = containerRef.current?.querySelector<HTMLElement>('[role="log"]');

    if (!element) return;

    let frame = 0;
    let retentionFrame = 0;
    let retentionTask: number | undefined;
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
      const restoring =
        retainedIdRef.current === id && (interacting() || correctionPendingRef.current);

      retainedIdRef.current = id;
      setRetainedId(id);

      if (restoring) return true;

      // Focusing the newest row at the bottom (including End navigation) keeps follow.
      // Older rows surrender it before the focus scroll's event is delivered.
      if (
        isNewest(id) &&
        (endOffset(element) - element.scrollTop <= 1 ||
          (anchorRef.current?.kind === "end" && checkFollow()))
      )
        return true;

      cancelAnimationFrame(frame);
      cancelPlacement(true, false, id);

      return true;
    };

    const onFocus = (event: Event) => {
      retainRow(event.target);

      if (event.type === "contextmenu" && event instanceof MouseEvent && event.buttons !== 0)
        retentionPendingRef.current = retainedIdRef.current;
    };

    const onBlur = (event: FocusEvent) => {
      releaseRetention(event.relatedTarget);
    };

    const onInput = (event: Event) => {
      let away = false;

      if (event instanceof KeyboardEvent) {
        if (
          event.defaultPrevented ||
          !(event.target instanceof HTMLElement) ||
          !element.contains(event.target)
        )
          return;

        if (
          !["ArrowUp", "ArrowDown", "PageUp", "PageDown", "Home", "End", " "].includes(event.key) ||
          event.target.closest('dialog, [role="menu"], [role="listbox"], [role="dialog"]') !==
            null ||
          (["ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight"].includes(event.key) &&
            event.target.closest('[aria-haspopup]:not([aria-haspopup="false"])') !== null) ||
          event.target.isContentEditable ||
          event.target.closest('input, textarea, select, video, audio, [role="textbox"]') !==
            null ||
          (event.key === " " && event.target.closest('button, [role="button"], summary') !== null)
        )
          return;

        away =
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
        const onScroller =
          event.button === 0 &&
          (event.target === element ||
            (event.target instanceof Node && element.contains(event.target)));

        if (retainRow(event.target)) {
          retentionPendingRef.current = retainedIdRef.current;
          noteReaderInput();

          return;
        }

        const bounds = element.getBoundingClientRect();
        const gutter = Math.max(16, element.offsetWidth - element.clientWidth);

        if (
          event.button !== 0 ||
          event.target !== element ||
          event.clientX < bounds.right - gutter ||
          element.scrollHeight <= element.clientHeight
        ) {
          if (onScroller) noteReaderInput();

          return;
        }

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
      noteReaderInput();
      cancelPlacement(true, !away);
    };

    const onMove = (event: Event) => {
      if (typeof TouchEvent !== "undefined" && event instanceof TouchEvent && touchY !== null) {
        const y = event.touches[0]?.clientY;

        if (y === undefined || y === touchY) return;

        noteReaderInput();
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
      if (event.type === "pointerup" || event.type === "pointercancel") {
        const releasedId = retainedIdRef.current;

        // Pressed context menus open in a task after release. Let their marker mount
        // before checking ownership, and never release a newly retained row.
        window.clearTimeout(retentionTask);
        cancelAnimationFrame(retentionFrame);
        retentionTask = window.setTimeout(() => {
          retentionTask = undefined;
          retentionFrame = requestAnimationFrame(() => {
            retentionFrame = 0;

            if (retentionPendingRef.current === releasedId) retentionPendingRef.current = null;

            if (retainedIdRef.current === releasedId) releaseRetention();
          });
        }, 0);
      }

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

    const inputs = ["wheel", "touchstart", "pointerdown", "mousedown", "auxclick"];

    for (const input of inputs)
      element.addEventListener(input, onInput, { capture: true, passive: true });

    // React delegates control handlers at its root. Check consumed keys after they run,
    // while the target containment check keeps composer and sibling logs isolated.
    element.ownerDocument.addEventListener("keydown", onInput);

    element.addEventListener("focusin", onFocus, true);
    element.addEventListener("focusout", onBlur, true);
    element.addEventListener("contextmenu", onFocus, true);
    element.addEventListener("scrollend", onNativeScrollEnd);

    element.addEventListener("touchmove", onMove, { capture: true, passive: true });
    element.ownerDocument.addEventListener("pointermove", onMove, { capture: true, passive: true });

    for (const release of ["touchend", "touchcancel"])
      element.addEventListener(release, onRelease, { capture: true, passive: true });

    for (const release of ["pointerup", "pointercancel"])
      element.ownerDocument.addEventListener(release, onRelease, { capture: true, passive: true });

    frame = requestAnimationFrame(tick);

    return () => {
      cancelAnimationFrame(frame);
      cancelAnimationFrame(retentionFrame);
      window.clearTimeout(retentionTask);
      retentionPendingRef.current = null;
      cancelAnimationFrame(settlementFrameRef.current);
      settlementFrameRef.current = 0;

      for (const input of inputs) element.removeEventListener(input, onInput, true);

      element.ownerDocument.removeEventListener("keydown", onInput);

      element.removeEventListener("focusin", onFocus, true);
      element.removeEventListener("focusout", onBlur, true);
      element.removeEventListener("contextmenu", onFocus, true);
      element.removeEventListener("scrollend", onNativeScrollEnd);

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
    const anchor = anchorRef.current;
    const list = listRef.current;
    const element = viewport();

    if (anchor === null || anchor.placement !== placement || list === null || !element) return;

    if (interacting() || endMotionRef.current || settlementFrameRef.current !== 0) {
      correctionPendingRef.current = true;

      return;
    }

    if (anchor.kind === "end") {
      const follows = checkFollow();
      correctionPendingRef.current = false;

      if (!follows) return;
      const offset = endOffset(element);

      if (Math.abs(offset - element.scrollTop) > 1) {
        element.scrollTop = offset;
        correctedOffsetRef.current = element.scrollTop;
      }

      pinEnd();

      return;
    }

    correctionPendingRef.current = false;

    if (anchor.kind === "intro") {
      if (Math.abs(element.scrollTop - anchor.scroll) > 1) {
        element.scrollTop = anchor.scroll;
        correctedOffsetRef.current = element.scrollTop;
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
      }
    }
  });

  useLayoutEffect(() => {
    if (!placed || anchorRef.current?.placement !== placement) anchorRef.current = null;

    if (!placed || placementRef.current?.placement !== placement) placementRef.current = null;

    correctedOffsetRef.current = null;
    correctionPendingRef.current = false;
    endMotionRef.current = false;

    if (!placed) return;

    const element = containerRef.current?.querySelector<HTMLElement>('[role="log"]');
    const content = element?.firstElementChild;

    if (!element || !content) return;

    const heights = new Map<Element, number>();
    const beforeResizes = beforeResizesRef.current;
    let correctingCards = !chunkLoaded();

    const resizes = new ResizeObserver((entries) => {
      afterMeasure();
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
          (entry.target !== content ||
            anchorRef.current?.kind === "end" ||
            anchorRef.current?.kind === "intro")
        )
          changed = true;
      }

      // Wrapper and content measurements can arrive separately. Correct the remaining
      // distance after Virtua updates the content height as well as when a row grows.
      if (
        changed &&
        (correctingCards ||
          anchorRef.current?.kind === "end" ||
          anchorRef.current?.kind === "intro")
      )
        correct();

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
          beforeResizes?.unobserve(row);
          observed.delete(row);
          heights.delete(row);
        }
      }

      for (const row of content.children) {
        if (!observed.has(row)) {
          observed.add(row);

          resizes.observe(row);
          beforeResizes?.observe(row);
        }
      }

      seed();

      if (!interacting()) {
        if (correctionPendingRef.current) correct();
      }

      releaseRetention();
    };

    const mutations = new MutationObserver(observeRows);

    const finishCards = () => {
      // Delayed growth can still affect a bottom reader or a post's intro after the chunk.
      // Keep observing those intents; message-position correction ends with the reveal.
      correctingCards = false;
      finishCardsRef.current = null;
      mutations.disconnect();
      mutations.observe(content, {
        childList: true,
        subtree: true,
        attributes: true,
        attributeFilter: ["data-editing", "data-popup-pending"],
      });
    };

    const stop = () => {
      resizes.disconnect();
      beforeResizes?.disconnect();
      measurementRef.current = null;
      mutations.disconnect();
      finishCardsRef.current = null;
    };

    finishCardsRef.current = finishCards;
    resizes.observe(element);
    resizes.observe(content);
    beforeResizes?.observe(element);
    beforeResizes?.observe(content);
    mutations.observe(content, {
      childList: true,
      subtree: true,
      attributes: true,
      attributeFilter: ["style", "data-editing", "data-popup-pending"],
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
