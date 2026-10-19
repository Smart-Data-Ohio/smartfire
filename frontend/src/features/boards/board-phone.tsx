import { Link } from "@tanstack/react-router";
import { type KeyboardEvent, useId, useLayoutEffect, useRef, useState } from "react";
import type { WorkStatus } from "../../gen/WorkStatus.ts";
import { parseBoardSearch } from "../../lib/board-search.ts";
import { useLongPress } from "../../lib/menu-long-press.ts";
import { AnimatedNumber } from "../../motion/animated-number.tsx";
import { prefersReducedMotion } from "../../motion/reduced-motion.ts";
import { useStore } from "../../store/store.ts";
import { Button } from "../../ui/button.tsx";
import { IconButton } from "../../ui/icon-button.tsx";
import { Icon } from "../../ui/icons/icon.tsx";
import { replyCountLabel, threadTitle, timeAgo } from "../threads/thread-format.ts";
import { useNow } from "../threads/use-now.ts";
import {
  BoardEmpty,
  type BodyProps,
  Digest,
  LoadMore,
  useColumns,
  useOpenPostId,
} from "./board-body.tsx";
import { linkedLabel, WORK_STATUS_LABEL, WORK_STATUSES } from "./board-format.ts";
import { OwnerLine, StatusChip, TagList } from "./board-parts.tsx";
import { PostMenu, useCanMovePost } from "./board-post-menu.tsx";
import { Lifecycle, useCatalogTags, useUnread } from "./board-rows.tsx";
import { useLiveFlip } from "./use-live-flip.ts";
import "./board-phone.css";

interface PhonePostProps {
  readonly roomId: number;
  readonly threadId: number;
  readonly open: boolean;
  readonly activeTag: string;
  /** The list mixes statuses, so its cards say theirs; a column's cards don't need to. */
  readonly showStatus?: boolean;
}

/**
 * One post on a phone: a full-width card that opens the post. For those who may move it, its "⋯"
 * or a finger resting on the card opens its menu, an action sheet on a touch screen.
 */
export function PhonePostCard({
  roomId,
  threadId,
  open,
  activeTag,
  showStatus = false,
}: PhonePostProps) {
  const thread = useStore((state) => state.threads[threadId]);
  const unread = useUnread(threadId);
  const catalog = useCatalogTags(roomId);
  const movable = useCanMovePost(roomId, threadId);
  const now = useNow();
  const [menuOpen, setMenuOpen] = useState(false);
  const longPress = useLongPress(() => setMenuOpen(true));

  if (thread === undefined || thread.work === null) {
    return null;
  }

  const work = thread.work;
  const title = threadTitle(thread);

  const gestures = movable
    ? {
        onPointerDown: longPress.start,
        onPointerMove: longPress.move,
        onPointerUp: longPress.cancel,
        onPointerCancel: longPress.cancel,
      }
    : {};

  return (
    <li className="phone-card-item" data-flip={threadId} data-movable={movable || undefined}>
      <Link
        to="/r/$roomId/t/$threadId"
        params={{ roomId, threadId }}
        search={parseBoardSearch}
        className="phone-card"
        data-open={open || undefined}
        data-unread={unread || undefined}
        data-pressed={menuOpen || undefined}
        aria-current={open ? "true" : undefined}
        draggable={false}
        {...gestures}
        onClick={(event) => {
          if (longPress.fired()) {
            event.preventDefault();
          }
        }}
        onContextMenu={(event) => {
          if (longPress.claim() || longPress.fired()) {
            event.preventDefault();
          }
        }}
      >
        {showStatus ? <StatusChip status={work.status} /> : null}
        <span className="phone-card-head">
          {unread ? <span className="board-unread-dot" role="img" aria-label="Unread" /> : null}
          <span className="phone-card-title">{title}</span>
        </span>
        <Lifecycle threadId={threadId} />
        <TagList tags={work.tags} active={activeTag} catalog={catalog} />
        <span className="phone-card-foot">
          <OwnerLine work={work} size={20} />
          <span className="phone-card-counts">
            <span className="phone-card-count tabular" title={replyCountLabel(work.messageCount)}>
              <Icon name="message-circle" size={14} />
              {work.messageCount}
            </span>
            {work.links.length > 0 ? (
              <span className="phone-card-count tabular" title={linkedLabel(work.links.length)}>
                <Icon name="link" size={14} />
                {work.links.length}
              </span>
            ) : null}
            <span className="phone-card-count">{timeAgo(thread.lastActivityAt, now)}</span>
          </span>
        </span>
      </Link>
      {movable ? (
        <PostMenu
          threadId={threadId}
          open={menuOpen}
          onOpenChange={setMenuOpen}
          trigger={(props) => (
            <IconButton
              {...props}
              icon="more"
              label={`${title} actions`}
              tooltipPlacement="left"
              className="phone-card-more"
            />
          )}
        />
      ) : null}
    </li>
  );
}

const NOTHING_HERE = {
  planned: "Nothing planned.",
  in_progress: "Nothing in progress.",
  blocked: "Nothing blocked.",
  done: "Nothing done yet.",
} as const satisfies Record<WorkStatus, string>;

function ColumnEmpty({
  status,
  filtered,
  onClear,
}: {
  readonly status: WorkStatus;
  readonly filtered: boolean;
  readonly onClear: () => void;
}) {
  return (
    <div className="phone-column-empty enter-fade" data-status={status}>
      <span className="phone-column-empty-glyph" aria-hidden="true">
        <span className="board-status-dot" />
      </span>
      {filtered ? (
        <>
          <p>No {WORK_STATUS_LABEL[status].toLowerCase()} posts match these filters.</p>
          <Button variant="secondary" size="sm" onClick={onClear}>
            Clear filters
          </Button>
        </>
      ) : (
        <p>{NOTHING_HERE[status]}</p>
      )}
    </div>
  );
}

/** The column each board last showed this session, so coming back lands where you left. */
const lastColumn = new Map<number, WorkStatus>();

function columnIndex(status: WorkStatus): number {
  return WORK_STATUSES.indexOf(status);
}

/**
 * The column view on a phone: one status at a time, the full width, under a switcher that shows
 * every status with its count. Swiping sideways pages between the statuses (scroll snap), and the
 * switcher's pill follows the finger; tapping a status slides there. Each column scrolls on its
 * own. Opens on the column this board last showed, else the first with posts.
 */
export function PhoneBoardColumns({ roomId, query, onNewPost, onClear }: BodyProps) {
  const columns = useColumns(roomId);
  const anyPosts = useStore((state) => state.boards[roomId]?.anyPosts ?? false);
  const openId = useOpenPostId();
  const pagerRef = useRef<HTMLDivElement | null>(null);
  const tabsRef = useRef<HTMLDivElement | null>(null);
  const frame = useRef(0);
  const id = useId();
  const filtered = query.owner !== "anyone" || query.tag !== "";

  const [active, setActive] = useState<WorkStatus>(
    () =>
      lastColumn.get(roomId) ??
      WORK_STATUSES.find((status) => columns[status].length > 0) ??
      "planned",
  );

  const opening = useRef(active);
  const total = WORK_STATUSES.reduce((sum, status) => sum + columns[status].length, 0);

  const signature = WORK_STATUSES.map((status) => `${status}:${columns[status].join(",")}`).join(
    "|",
  );

  useLiveFlip(pagerRef, signature, `${query.owner}|${query.tag}`);

  useLayoutEffect(() => {
    const pager = pagerRef.current;

    if (pager !== null) {
      pager.scrollLeft = columnIndex(opening.current) * pager.clientWidth;
      tabsRef.current?.style.setProperty("--pager-progress", `${columnIndex(opening.current)}`);
    }

    return () => cancelAnimationFrame(frame.current);
  }, []);

  const follow = () => {
    cancelAnimationFrame(frame.current);
    frame.current = requestAnimationFrame(() => {
      const pager = pagerRef.current;

      if (pager === null || pager.clientWidth === 0) {
        return;
      }

      const progress = pager.scrollLeft / pager.clientWidth;
      const showing = WORK_STATUSES[Math.round(progress)] ?? "planned";

      tabsRef.current?.style.setProperty("--pager-progress", `${progress}`);
      lastColumn.set(roomId, showing);
      setActive(showing);
    });
  };

  const show = (status: WorkStatus) => {
    const pager = pagerRef.current;

    pager?.scrollTo({
      left: columnIndex(status) * pager.clientWidth,
      behavior: prefersReducedMotion() ? "instant" : "smooth",
    });
  };

  const onTabKey = (event: KeyboardEvent<HTMLButtonElement>) => {
    const at = columnIndex(active);
    const last = WORK_STATUSES.length - 1;

    const next = {
      ArrowLeft: Math.max(0, at - 1),
      ArrowRight: Math.min(last, at + 1),
      Home: 0,
      End: last,
    }[event.key];

    const status = next === undefined ? undefined : WORK_STATUSES[next];

    if (status === undefined) {
      return;
    }

    event.preventDefault();
    show(status);
    document.getElementById(`${id}-${status}`)?.focus({ preventScroll: true });
  };

  if (total === 0 && !anyPosts) {
    return (
      <div className="board-scroll">
        <BoardEmpty anyPosts={false} onNewPost={onNewPost} onClear={onClear} />
      </div>
    );
  }

  return (
    <div className="phone-board">
      <Digest roomId={roomId} />
      <div className="phone-board-tabs" role="tablist" aria-label="Status" ref={tabsRef}>
        <span className="phone-board-pill" aria-hidden="true" />
        {WORK_STATUSES.map((status) => (
          <button
            key={status}
            id={`${id}-${status}`}
            type="button"
            role="tab"
            className="phone-board-tab"
            data-status={status}
            aria-selected={status === active}
            aria-controls={`${id}-${status}-panel`}
            tabIndex={status === active ? 0 : -1}
            onClick={() => show(status)}
            onKeyDown={onTabKey}
          >
            <span className="phone-board-tab-label">
              <span className="board-status-dot" aria-hidden="true" />
              {WORK_STATUS_LABEL[status]}
            </span>
            <AnimatedNumber value={columns[status].length} className="phone-board-tab-count" />
          </button>
        ))}
      </div>
      <div className="phone-board-pager" ref={pagerRef} onScroll={follow}>
        {WORK_STATUSES.map((status) => (
          <section
            key={status}
            id={`${id}-${status}-panel`}
            role="tabpanel"
            className="phone-board-page"
            data-status={status}
            data-flip-group={status}
            data-flip-scroll
            aria-labelledby={`${id}-${status}`}
          >
            {columns[status].length === 0 ? (
              <ColumnEmpty status={status} filtered={filtered} onClear={onClear} />
            ) : (
              <ul className="phone-board-list">
                {columns[status].map((post) => (
                  <PhonePostCard
                    key={post}
                    roomId={roomId}
                    threadId={post}
                    open={post === openId}
                    activeTag={query.tag}
                  />
                ))}
              </ul>
            )}
            <LoadMore roomId={roomId} />
          </section>
        ))}
      </div>
    </div>
  );
}

/** A phone's new-post button: bottom right, in thumb reach, clear of the home indicator. */
export function NewPostFab({ onNewPost }: { readonly onNewPost: () => void }) {
  return (
    <Button variant="primary" icon="plus" className="board-fab" onClick={onNewPost}>
      New post
    </Button>
  );
}
