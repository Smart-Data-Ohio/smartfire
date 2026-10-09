import { useNavigate, useParams } from "@tanstack/react-router";
import type { MessageDTO } from "../../store/model.ts";
import { useStore } from "../../store/store.ts";
import { Icon } from "../../ui/icons/icon.tsx";
import { WorkLinks, WorkSummary } from "../work/work-facts.tsx";
import { AvatarGroup } from "./avatar-group.tsx";
import { useEnsureUsers } from "./ensure-users.ts";
import { lastReplyLabel, replyCountLabel, threadTitle } from "./thread-format.ts";
import { useNow } from "./use-now.ts";
import "./threads.css";

const NO_IDS: readonly number[] = [];

/**
 * Under a message that started a thread: the last repliers' faces, the reply count, "Last reply
 * 2 hours ago" and, on hover, "View thread"; a dot when the thread is unread for the viewer. It
 * follows `thread.indicator` events (the message's `thread` changes) and the viewer's membership,
 * and opens the thread in the right pane. A tracked thread's status, owner and links follow it.
 */
export function ThreadIndicator({ message }: { readonly message: MessageDTO }) {
  const navigate = useNavigate();
  const params = useParams({ strict: false });
  const now = useNow();
  const indicator = message.thread;
  const threadId = indicator?.threadId ?? null;

  useEnsureUsers(indicator?.replierIds ?? NO_IDS);

  const unread = useStore((state) =>
    threadId === null ? false : (state.threadMemberships[threadId]?.unreadAt ?? null) !== null,
  );

  // The indicator carries no work facts: they come with the thread (the room's active threads
  // are fetched when it opens, others once listed or opened).
  const thread = useStore((state) => (threadId === null ? undefined : state.threads[threadId]));
  const work = thread?.work ?? null;

  if (indicator === null || indicator.replyCount === 0) {
    return null;
  }

  const open = params.threadId === indicator.threadId;
  const count = replyCountLabel(indicator.replyCount);
  const last = lastReplyLabel(indicator.lastReplyAt, now);

  return (
    <div className="thread-indicator-row">
      <button
        type="button"
        className="thread-indicator"
        data-unread={unread || undefined}
        data-open={open || undefined}
        aria-label={`${count}${unread ? ", unread" : ""}. ${last}. View thread`}
        aria-current={open ? "true" : undefined}
        onClick={() =>
          void navigate({
            to: "/r/$roomId/t/$threadId",
            params: { roomId: message.roomId, threadId: indicator.threadId },
          })
        }
      >
        {indicator.replierIds.length === 0 ? null : (
          <AvatarGroup userIds={indicator.replierIds} size={20} max={3} />
        )}
        <span className="thread-indicator-count">{count}</span>
        {unread ? <span className="thread-indicator-dot" aria-hidden="true" /> : null}
        <time className="thread-indicator-last" dateTime={indicator.lastReplyAt} aria-hidden="true">
          {last}
        </time>
        <span className="thread-indicator-view" aria-hidden="true">
          View thread
          <Icon name="chevron-right" size={14} />
        </span>
      </button>
      {work === null ? null : (
        <span className="thread-indicator-work">
          <WorkSummary facts={work} />
          <WorkLinks
            links={work.links}
            runUrl={work.runUrl}
            label={`Links for ${threadTitle(thread)}`}
          />
        </span>
      )}
    </div>
  );
}
