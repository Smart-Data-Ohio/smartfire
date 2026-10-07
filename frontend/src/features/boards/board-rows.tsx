import { Link } from "@tanstack/react-router";
import { parseBoardSearch } from "../../lib/board-search.ts";
import { formatFull } from "../../lib/time.ts";
import { useStore } from "../../store/store.ts";
import { Icon } from "../../ui/icons/icon.tsx";
import {
  replyCountLabel,
  THREAD_STATUS_LABEL,
  threadTitle,
  timeAgo,
} from "../threads/thread-format.ts";
import { useNow } from "../threads/use-now.ts";
import { linkedLabel } from "./board-format.ts";
import { OwnerLine, StatusChip, TagList } from "./board-parts.tsx";

interface PostProps {
  readonly roomId: number;
  readonly threadId: number;
  /** The post is open in the right pane. */
  readonly open: boolean;
  /** The tag filter, so the matching pill stands out. */
  readonly activeTag: string;
}

/** Locked or closed, beside the title (an active post says nothing). */
function Lifecycle({ threadId }: { readonly threadId: number }) {
  const status = useStore((state) => state.threads[threadId]?.status ?? "active");

  if (status === "active") {
    return null;
  }

  return (
    <span className="board-lifecycle" data-status={status}>
      <Icon name={status === "locked" ? "lock" : "archive"} size={12} />
      {THREAD_STATUS_LABEL[status]}
    </span>
  );
}

function useUnread(threadId: number): boolean {
  return useStore((state) => (state.threadMemberships[threadId]?.unreadAt ?? null) !== null);
}

/**
 * One post in the list, the forum row: title with its lifecycle and tags, then the owner, reply
 * and link counts and when it last moved; the status sits at the end. Opens in the right pane.
 */
export function PostRow({ roomId, threadId, open, activeTag }: PostProps) {
  const thread = useStore((state) => state.threads[threadId]);
  const unread = useUnread(threadId);
  const now = useNow();

  if (thread === undefined || thread.work === null) {
    return null;
  }

  const work = thread.work;

  return (
    <li className="board-row-item" data-flip={threadId}>
      <Link
        to="/r/$roomId/t/$threadId"
        params={{ roomId, threadId }}
        search={parseBoardSearch}
        className="board-row"
        data-open={open || undefined}
        data-unread={unread || undefined}
        aria-current={open ? "true" : undefined}
      >
        <span className="board-row-main">
          <span className="board-row-head">
            {unread ? <span className="board-unread-dot" role="img" aria-label="Unread" /> : null}
            <span className="board-row-title">{threadTitle(thread)}</span>
            <Lifecycle threadId={threadId} />
            <TagList tags={work.tags} active={activeTag} />
          </span>
          <span className="board-row-meta">
            <OwnerLine work={work} />
            <span className="board-meta-item tabular">{replyCountLabel(thread.replyCount)}</span>
            {work.links.length > 0 ? (
              <span className="board-meta-item tabular">{linkedLabel(work.links.length)}</span>
            ) : null}
          </span>
        </span>
        <span className="board-row-side">
          <StatusChip status={work.status} />
          <span className="board-row-time" title={formatFull(thread.lastActivityAt)}>
            Updated {timeAgo(thread.lastActivityAt, now)}
          </span>
        </span>
      </Link>
    </li>
  );
}

/** One post in a status column: a compact card with the title, tags, owner and counts. */
export function PostCard({ roomId, threadId, open, activeTag }: PostProps) {
  const thread = useStore((state) => state.threads[threadId]);
  const unread = useUnread(threadId);
  const now = useNow();

  if (thread === undefined || thread.work === null) {
    return null;
  }

  const work = thread.work;

  return (
    <li className="board-card-item" data-flip={threadId}>
      <Link
        to="/r/$roomId/t/$threadId"
        params={{ roomId, threadId }}
        search={parseBoardSearch}
        className="board-card"
        data-open={open || undefined}
        data-unread={unread || undefined}
        aria-current={open ? "true" : undefined}
      >
        <span className="board-card-head">
          {unread ? <span className="board-unread-dot" role="img" aria-label="Unread" /> : null}
          <span className="board-card-title">{threadTitle(thread)}</span>
        </span>
        <Lifecycle threadId={threadId} />
        <TagList tags={work.tags} active={activeTag} />
        <span className="board-card-foot">
          <OwnerLine work={work} />
          <span className="board-card-counts">
            <span className="board-card-count tabular" title={replyCountLabel(thread.replyCount)}>
              <Icon name="message-circle" size={12} />
              {thread.replyCount}
            </span>
            {work.links.length > 0 ? (
              <span className="board-card-count tabular" title={linkedLabel(work.links.length)}>
                <Icon name="link" size={12} />
                {work.links.length}
              </span>
            ) : null}
            <span className="board-card-count">{timeAgo(thread.lastActivityAt, now)}</span>
          </span>
        </span>
      </Link>
    </li>
  );
}
