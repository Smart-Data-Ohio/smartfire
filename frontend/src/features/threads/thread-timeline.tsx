import { useLayoutEffect, useRef, useState } from "react";
import { VList, type VListHandle } from "virtua";
import { toMillis } from "../../lib/time.ts";
import type { MessageDTO, PendingMessage } from "../../store/model.ts";
import { emptyTimeline } from "../../store/state.ts";
import { useMessagesIn, useStore } from "../../store/store.ts";
import { actions } from "../../sync/runtime.ts";
import { Spinner } from "../../ui/button.tsx";
import { Icon } from "../../ui/icons/icon.tsx";
import { Skeleton, SkeletonReveal } from "../../ui/skeleton.tsx";
import { useListEdges } from "../messages/list-edges.ts";
import { DayDivider } from "../room/dividers.tsx";
import { MessageRow, PendingRow } from "../room/message-row.tsx";
import {
  type CommittedEdges,
  firstMessageKey,
  prepended,
  type TimelineItem,
  timelineItems,
} from "../room/timeline-items.ts";
import { replyCountLabel } from "./thread-format.ts";

/** Within this many px of the end counts as "at the bottom": new replies keep it pinned. */
const BOTTOM_SLOP = 40;

/** Fetch the next page when this close to an edge. */
const PAGE_AHEAD = 600;

/** A reply that arrived less than this ago, after the pane opened, rises in. */
const LIVE_WINDOW_MS = 8000;

const NO_PENDING: readonly string[] = [];

function mentionsViewer(bodyHtml: string, viewerId: number | null): boolean {
  return viewerId !== null && bodyHtml.includes(`data-user-id="${viewerId}"`);
}

/** The root message on top of the thread, then the "N replies" rule. */
function ThreadParent({
  parent,
  replyCount,
  viewerId,
}: {
  readonly parent: MessageDTO | null;
  readonly replyCount: number;
  readonly viewerId: number | null;
}) {
  return (
    <div className="thread-parent">
      {parent === null ? (
        <p className="thread-parent-gone">
          <Icon name="trash" size={14} />
          The original message was deleted.
        </p>
      ) : (
        <MessageRow
          message={parent}
          groupStart
          mentionsMe={mentionsViewer(parent.bodyHtml, viewerId)}
          focused={false}
          live={false}
          inThread
        />
      )}
      <div className="thread-replies-rule">
        <span className="thread-replies-label tabular">
          {replyCount === 0 ? "No replies yet" : replyCountLabel(replyCount)}
        </span>
      </div>
    </div>
  );
}

/** Placeholder for the parent and a few replies while the thread loads. */
export function ThreadSkeleton() {
  return (
    <div className="thread-skeleton">
      {[82, 46, 64, 38].map((width, index) => (
        // The widths are distinct, so each one names its row.
        <div key={width} className="thread-skeleton-row">
          <Skeleton width={32} height={32} radius="md" />
          <div className="thread-skeleton-lines">
            <Skeleton width={index === 0 ? 132 : 96} height={11} />
            <Skeleton width={`${width}%`} height={11} />
            {index === 0 ? <Skeleton width="58%" height={11} /> : null}
          </div>
        </div>
      ))}
    </div>
  );
}

interface ThreadTimelineProps {
  readonly threadId: number;
  readonly parent: MessageDTO | null;
  readonly replyCount: number;
  /** The header has loaded; until then the skeleton shows. */
  readonly ready: boolean;
  /** A reply's permalink: placed in view and highlighted instead of opening at the newest. */
  readonly focusMessageId: number | null;
}

/**
 * A thread's conversation on virtua, the room timeline's approach in a narrower column: the
 * parent message at the start of history with the reply count under it, then the replies grouped
 * by author with day dividers, pending replies after them. It opens at the newest reply, pages at
 * either edge without jumping, and keeps new replies in view while you're at the bottom.
 */
export function ThreadTimeline({
  threadId,
  parent,
  replyCount,
  ready,
  focusMessageId,
}: ThreadTimelineProps) {
  const timeline = useStore((state) => state.threadTimelines[threadId] ?? emptyTimeline);
  const messages = useMessagesIn(timeline.ids);
  const pendingIds = useStore((state) => state.pendingByThread[threadId] ?? NO_PENDING);
  const pendingById = useStore((state) => state.pending);
  const viewerId = useStore((state) => state.me?.user.id ?? state.boot?.user.id ?? null);
  const [openedAt] = useState(() => Date.now());
  const listRef = useRef<VListHandle | null>(null);
  const containerRef = useRef<HTMLDivElement | null>(null);
  const atBottomRef = useRef(true);
  const placedRef = useRef<string | null>(null);

  const committedRef = useRef<CommittedEdges & { readonly last: string | null }>({
    first: null,
    firstMessage: null,
    last: null,
  });

  const pending = pendingIds.flatMap((id): PendingMessage[] => {
    const entry = pendingById[id];

    return entry === undefined ? [] : [entry];
  });

  const now = Date.now();
  const loaded = ready && timeline.status === "ready";
  const items = loaded ? timelineItems({ timeline, messages, pending, now }) : [];

  useListEdges(containerRef, listRef, items);
  const firstKey = items[0]?.key ?? null;
  const lastKey = items.at(-1)?.key ?? null;
  const shift = prepended(items, committedRef.current);

  // Place the view once per loaded window: on the permalinked reply, else at the newest.
  useLayoutEffect(() => {
    const list = listRef.current;
    const placement = `${timeline.generation}:${focusMessageId ?? ""}`;

    if (!loaded || list === null || items.length === 0 || placedRef.current === placement) {
      return;
    }

    placedRef.current = placement;

    const focusIndex =
      focusMessageId === null
        ? -1
        : items.findIndex((item) => item.kind === "message" && item.message.id === focusMessageId);

    if (focusIndex >= 0) {
      list.scrollToIndex(focusIndex, { align: "center" });
    } else {
      list.scrollToIndex(items.length - 1, { align: "end" });
    }
  });

  // Follow new replies at the bottom (always your own).
  useLayoutEffect(() => {
    const previous = committedRef.current;
    const appended = previous.last !== null && lastKey !== previous.last && !shift;

    committedRef.current = {
      first: firstKey,
      firstMessage: firstMessageKey(items),
      last: lastKey,
    };

    if (appended && (atBottomRef.current || items.at(-1)?.kind === "pending")) {
      listRef.current?.scrollToIndex(items.length - 1, { align: "end" });
    }
  });

  const onScroll = (offset: number) => {
    const list = listRef.current;

    if (list === null) {
      return;
    }

    const distance = list.scrollSize - offset - list.viewportSize;

    atBottomRef.current = distance < BOTTOM_SLOP;

    if (offset < PAGE_AHEAD && timeline.before !== null && !timeline.loadingOlder) {
      void actions.threads.loadOlder(threadId);
    }

    if (distance < PAGE_AHEAD && timeline.after !== null && !timeline.loadingNewer) {
      void actions.threads.loadNewer(threadId);
    }
  };

  const renderItem = (item: TimelineItem) => {
    switch (item.kind) {
      case "intro":
        return (
          <ThreadParent
            key={item.key}
            parent={parent}
            replyCount={replyCount}
            viewerId={viewerId}
          />
        );
      case "loading":
        return (
          <div key={item.key} className="timeline-loading" role="status">
            <Spinner label="Loading replies" />
          </div>
        );
      case "day":
        return <DayDivider key={item.key} label={item.label} />;
      case "unread":
        // Threads keep no read position, so this never comes; an empty row keeps the switch whole.
        return <div key={item.key} />;
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
            inThread
          />
        );
      }
    }
  };

  return (
    <div className="thread-timeline" ref={containerRef}>
      <SkeletonReveal loading={!loaded} skeleton={<ThreadSkeleton />}>
        {loaded ? (
          <VList
            ref={listRef}
            className="thread-timeline-list"
            shift={shift}
            bufferSize={400}
            onScroll={onScroll}
            data={items}
            aria-label="Replies"
            role="log"
          >
            {renderItem}
          </VList>
        ) : (
          <div className="thread-timeline-list" />
        )}
      </SkeletonReveal>
    </div>
  );
}
