import { useNavigate } from "@tanstack/react-router";
import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { VList, type VListHandle } from "virtua";
import { toMillis } from "../../lib/time.ts";
import type { PendingMessage } from "../../store/model.ts";
import { emptyTimeline } from "../../store/state.ts";
import { store, useMessagesIn, useStore } from "../../store/store.ts";
import { actions } from "../../sync/runtime.ts";
import { Button, Spinner } from "../../ui/button.tsx";
import { Skeleton, SkeletonReveal } from "../../ui/skeleton.tsx";
import { hasCards, useCardsChunkLoaded, useCardsChunkSettled } from "../cards/card-slot.tsx";
import { useEditingId } from "../messages/editing-store.ts";
import { useListEdges } from "../messages/list-edges.ts";
import { isUnreadHeld, releaseUnread } from "../messages/unread-hold.ts";
import { useViewportAnchor } from "../messages/viewport-anchor.ts";
import { DayDivider, RoomIntro, UnreadDivider } from "./dividers.tsx";
import { useFollowPosted } from "./follow-posted.ts";
import { MessageRow, PendingRow } from "./message-row.tsx";
import {
  type CommittedEdges,
  firstMessageKey,
  prepended,
  type TimelineItem,
  timelineItems,
} from "./timeline-items.ts";

/** Within this many px of the end, clear unread and new-message indicators. */
const BOTTOM_SLOP = 40;

/** Start fetching the next page when this close to an edge. */
const PAGE_AHEAD = 800;

/** A message that arrived less than this ago, after the room opened, rises in. */
const LIVE_WINDOW_MS = 8000;

const NO_PENDING: readonly string[] = [];

const LIST_STYLE = { display: "flex", flexDirection: "column" } as const;

function mentionsViewer(bodyHtml: string, viewerId: number | null): boolean {
  return viewerId !== null && bodyHtml.includes(`data-user-id="${viewerId}"`);
}

/** The label of the nearest day divider at or above `index`. */
function dayAt(items: readonly TimelineItem[], index: number): string | null {
  for (let cursor = Math.min(index, items.length - 1); cursor >= 0; cursor -= 1) {
    const item = items[cursor];

    if (item?.kind === "day") {
      return item.label;
    }
  }

  return null;
}

function TimelineSkeleton() {
  return (
    <div className="timeline-skeleton">
      {[68, 42, 84, 55, 30, 76].map((width, index) => (
        // The widths are distinct, so each one names its row.
        <div key={width} className="timeline-skeleton-row">
          <Skeleton width={36} height={36} radius="md" />
          <div className="timeline-skeleton-lines">
            <Skeleton width={index % 2 === 0 ? 120 : 96} height={11} />
            <Skeleton width={`${width}%`} height={11} />
          </div>
        </div>
      ))}
    </div>
  );
}

interface TimelineProps {
  readonly roomId: number;
  readonly focusMessageId: number | null;
}

/**
 * A room's timeline on virtua: only the rows near the viewport are mounted, history pages in at
 * either edge without the view jumping (`shift` keeps the anchor while older rows prepend), live
 * messages keep it pinned to the bottom, and a pill offers the way back to the present.
 */
export function Timeline({ roomId, focusMessageId }: TimelineProps) {
  const navigate = useNavigate();
  const timeline = useStore((state) => state.timelines[roomId] ?? emptyTimeline);
  const messages = useMessagesIn(timeline.ids);
  const pendingIds = useStore((state) => state.pendingByRoom[roomId] ?? NO_PENDING);
  const pendingById = useStore((state) => state.pending);
  const viewerId = useStore((state) => state.me?.user.id ?? state.boot?.user.id ?? null);
  const [openedAt] = useState(() => Date.now());
  const listRef = useRef<VListHandle | null>(null);
  const containerRef = useRef<HTMLDivElement | null>(null);
  const atBottomRef = useRef(true);

  const committedRef = useRef<
    CommittedEdges & { readonly last: string | null; readonly count: number }
  >({
    first: null,
    firstMessage: null,
    last: null,
    count: 0,
  });

  // The window last placed (`room:generation:focus`).
  const [placed, setPlaced] = useState<string | null>(null);
  const [farFromPresent, setFarFromPresent] = useState(false);
  const [newBelow, setNewBelow] = useState(0);
  const [floatingDay, setFloatingDay] = useState<string | null>(null);
  const dayTimerRef = useRef<number | undefined>(undefined);

  const pending = pendingIds.flatMap((id): PendingMessage[] => {
    const entry = pendingById[id];

    return entry === undefined ? [] : [entry];
  });

  const now = Date.now();
  const items = timelineItems({ timeline, messages, pending, now });

  useListEdges(containerRef, listRef, items);

  const ready = timeline.status === "ready";
  const placement = `${roomId}:${timeline.generation}:${focusMessageId ?? ""}`;
  const cardsSettled = useCardsChunkSettled();
  const cardsLoaded = useCardsChunkLoaded();

  // A loaded window that holds cards stays under the skeleton, unplaced, until their chunk
  // settles: placed earlier, the cards growing in above a permalinked row would push it out of
  // view. Only before its first placement: once placed, the list stays as it is (a card arriving
  // live or in an older page grows in place, and following and anchoring carry on).
  const awaitingCards =
    ready &&
    placed !== placement &&
    !cardsSettled &&
    items.some((item) => item.kind === "message" && hasCards(item.message));

  const firstKey = items[0]?.key ?? null;
  // The loading row disappears when a newer page lands; follow from the previous content.
  const lastKey = items.findLast((item) => item.kind !== "loading")?.key ?? null;

  // Older rows went in above the previous first row: keep the view anchored from the end.
  const shift = prepended(items, committedRef.current);

  const focusIndex =
    focusMessageId === null
      ? -1
      : items.findIndex((item) => item.kind === "message" && item.message.id === focusMessageId);

  const unreadIndex = items.findIndex((item) => item.kind === "unread");

  const {
    capture: captureAnchor,
    settle: settleAnchor,
    place: placeAnchor,
    isPlacing,
    canFollow,
    followEnd,
    takeControl,
    keepMounted,
  } = useViewportAnchor({
    containerRef,
    listRef,
    items,
    placement,
    placed: ready && placed === placement,
    cardsLoaded,
  });

  // Place the view once per loaded window: on the permalinked message, on the unread divider
  // (clamped, so a short unread run just lands at the bottom), or at the bottom.
  useLayoutEffect(() => {
    const list = listRef.current;

    if (!ready || awaitingCards || list === null || placed === placement || items.length === 0) {
      return;
    }

    setPlaced(placement);
    const viewport = containerRef.current?.querySelector<HTMLElement>("[data-message-list]");

    if (viewport) viewport.dataset.scrollSettled = "false";

    if (focusIndex >= 0) {
      placeAnchor(focusIndex, { align: "center" });
    } else if (unreadIndex >= 0) {
      placeAnchor(unreadIndex, { align: "start", offset: -8 });
    } else {
      placeAnchor(items.length - 1, { align: "end" });
    }
  });

  // Follow new rows at the bottom; count them when scrolled up.
  useLayoutEffect(() => {
    const previous = committedRef.current;
    const appended = previous.last !== null && lastKey !== previous.last && !shift;

    committedRef.current = {
      first: firstKey,
      firstMessage: firstMessageKey(items),
      last: lastKey,
      count: items.length,
    };

    if (!appended || !ready || placed !== placement || isPlacing()) {
      return;
    }

    const last = items.at(-1);
    const mine = last?.kind === "pending";

    if (canFollow() || mine) {
      if (mine) followEnd();

      listRef.current?.scrollToIndex(items.length - 1, { align: "end" });
    } else if (timeline.after === null) {
      setNewBelow((count) => count + Math.max(1, items.length - previous.count));
    }
  });

  useFollowPosted(`room:${roomId}`, items, listRef, followEnd);

  // A resync can leave the window short of the present with the reader at its end (more was
  // posted than a page holds while they were away): page on and offer the jump without a scroll.
  // biome-ignore lint/correctness/useExhaustiveDependencies: when the window's end moves, not on every render
  useEffect(() => {
    const list = listRef.current;

    if (!ready || list === null || timeline.after === null) {
      return;
    }

    setFarFromPresent(true);
    loadNewerNear(list.scrollSize - list.scrollOffset - list.viewportSize);
  }, [ready, timeline.after]);

  // An edit opened from elsewhere (the composer's ↑) brings its row into view.
  const editingId = useEditingId();

  // biome-ignore lint/correctness/useExhaustiveDependencies: only when a new edit starts, not on every new message
  useEffect(() => {
    if (editingId === null) {
      return;
    }

    const index = items.findIndex(
      (item) => item.kind === "message" && item.message.id === editingId,
    );

    if (index >= 0) {
      takeControl();
      listRef.current?.scrollToIndex(index, { align: "nearest" });
    }
  }, [editingId]);

  // "Mark unread" holds the room unread until it's left; let go when this timeline goes.
  useEffect(() => () => releaseUnread(roomId), [roomId]);

  const markReadIfDue = () => {
    const row = store.getState().sidebar.rows[roomId];

    if (
      !isUnreadHeld(roomId) &&
      atBottomRef.current &&
      timeline.after === null &&
      document.visibilityState === "visible" &&
      row !== undefined &&
      row.membership.unreadAt !== null
    ) {
      void actions.markRead(roomId);
    }
  };

  useEffect(markReadIfDue);

  useEffect(() => {
    const onVisible = () => markReadIfDue();

    document.addEventListener("visibilitychange", onVisible);
    window.addEventListener("focus", onVisible);

    return () => {
      document.removeEventListener("visibilitychange", onVisible);
      window.removeEventListener("focus", onVisible);
      window.clearTimeout(dayTimerRef.current);
    };
  });

  /** The next newer page, when the view is within `PAGE_AHEAD` of a window short of the present. */
  const loadNewerNear = (distance: number) => {
    if (distance < PAGE_AHEAD && timeline.after !== null && !timeline.loadingNewer) {
      void actions.loadNewer(roomId);
    }
  };

  const onScroll = (offset: number) => {
    const list = listRef.current;

    if (list === null) {
      return;
    }

    const distance = list.scrollSize - offset - list.viewportSize;

    atBottomRef.current = distance < BOTTOM_SLOP;
    captureAnchor();

    if (atBottomRef.current) {
      setNewBelow(0);
      markReadIfDue();
    }

    setFarFromPresent(timeline.after !== null || distance > list.viewportSize * 1.5);

    if (offset < PAGE_AHEAD && timeline.before !== null && !timeline.loadingOlder) {
      void actions.loadOlder(roomId);
    }

    loadNewerNear(distance);
    setFloatingDay(offset > 24 ? dayAt(items, list.findItemIndex(offset + 8)) : null);
    window.clearTimeout(dayTimerRef.current);
    dayTimerRef.current = window.setTimeout(() => setFloatingDay(null), 1200);
  };

  const jumpToPresent = () => {
    setNewBelow(0);

    if (focusMessageId !== null) {
      void navigate({ to: "/r/$roomId", params: { roomId }, replace: true });
    }

    if (timeline.after !== null) {
      void actions.jumpToPresent(roomId);

      return;
    }

    followEnd();
    listRef.current?.scrollToIndex(items.length - 1, { align: "end", smooth: true });
  };

  const renderItem = (item: TimelineItem) => {
    switch (item.kind) {
      case "intro":
        return <RoomIntro key={item.key} roomId={roomId} />;
      case "loading":
        return (
          <div key={item.key} className="timeline-loading" role="status">
            <Spinner label="Loading messages" />
          </div>
        );
      case "day":
        return <DayDivider key={item.key} label={item.label} />;
      case "unread":
        return <UnreadDivider key={item.key} count={item.count} />;
      case "pending":
        return <PendingRow key={item.key} pending={item.pending} groupStart={item.groupStart} />;
      case "message": {
        const created = toMillis(item.message.createdAt);

        return (
          <MessageRow
            key={item.key}
            message={item.message}
            groupStart={item.groupStart}
            mentionsMe={mentionsViewer(item.message.bodyHtml, viewerId)}
            focused={item.message.id === focusMessageId}
            live={created > openedAt && now - created < LIVE_WINDOW_MS}
          />
        );
      }
    }
  };

  return (
    <div className="timeline" ref={containerRef}>
      <SkeletonReveal loading={!ready || awaitingCards} skeleton={<TimelineSkeleton />}>
        {ready ? (
          <VList
            ref={listRef}
            className="timeline-list"
            // A column, so the content can sit at the bottom of a short room (CSS margin-top: auto).
            style={LIST_STYLE}
            shift={shift}
            bufferSize={600}
            keepMounted={keepMounted}
            onScroll={onScroll}
            onScrollCapture={(event) => {
              if (event.target === event.currentTarget)
                event.currentTarget.dataset.scrollSettled = "false";
            }}
            onScrollEnd={() => {
              const viewport =
                containerRef.current?.querySelector<HTMLElement>("[data-message-list]");

              if (viewport) viewport.dataset.scrollSettled = "true";

              settleAnchor();
            }}
            data={items}
            aria-label="Messages"
            role="log"
            // Focusable from script only: Home/End hold focus here while the edge row is drawn.
            tabIndex={-1}
            data-message-list
            data-scroll-settled="false"
            data-placement-settled="false"
          >
            {renderItem}
          </VList>
        ) : (
          <div className="timeline-list" />
        )}
      </SkeletonReveal>
      <div className="timeline-day-float" data-open={floatingDay !== null} aria-hidden="true">
        {floatingDay}
      </div>
      <div className="timeline-jump" data-open={ready && (farFromPresent || newBelow > 0)}>
        <Button
          variant="secondary"
          size="sm"
          icon="arrow-down"
          onClick={jumpToPresent}
          tabIndex={farFromPresent || newBelow > 0 ? 0 : -1}
        >
          {newBelow > 0
            ? `${newBelow} new ${newBelow === 1 ? "message" : "messages"}`
            : "Jump to present"}
        </Button>
      </div>
    </div>
  );
}
