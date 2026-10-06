import { useEffect, useState } from "react";
import type { Thread } from "../../store/model.ts";
import { useStore } from "../../store/store.ts";
import { actions } from "../../sync/runtime.ts";
import { Icon } from "../../ui/icons/icon.tsx";
import { SkeletonReveal } from "../../ui/skeleton.tsx";
import { Tabs } from "../../ui/tabs.tsx";
import { PaneFrame, RoomName } from "../panes/pane-frame.tsx";
import { PaneEmpty, PaneError, PaneListSkeleton } from "../panes/pane-states.tsx";
import { usePaneNavigation } from "../panes/use-right-pane.ts";
import { UserAvatar } from "../people/user-avatar.tsx";
import {
  lastReplyLabel,
  replyCountLabel,
  THREAD_STATUS_LABEL,
  threadTitle,
} from "./thread-format.ts";
import {
  EMPTY_TAB_TEXT,
  isThreadTab,
  serverFilter,
  THREAD_TABS,
  type ThreadTab,
  visibleThreads,
} from "./thread-list.ts";
import { useNow } from "./use-now.ts";

/** The tab last picked, for the next time the pane opens in this tab. */
let lastTab: ThreadTab = "active";

function ThreadRow({ thread, onOpen }: { readonly thread: Thread; readonly onOpen: () => void }) {
  const now = useNow();
  const membership = useStore((state) => state.threadMemberships[thread.id] ?? null);
  const creator = useStore((state) => state.users[thread.creatorId]);
  const unread = membership?.unreadAt != null;
  const following = membership?.involvement === "everything";

  return (
    <li>
      <button
        type="button"
        className="thread-row"
        data-unread={unread || undefined}
        onClick={onOpen}
      >
        <UserAvatar userId={thread.creatorId} size={32} decorative />
        <span className="thread-row-main">
          <span className="thread-row-name">
            <span className="thread-row-title">{threadTitle(thread)}</span>
            {unread ? <span className="thread-row-dot" aria-label="Unread" role="img" /> : null}
          </span>
          <span className="thread-row-meta">
            <span>{creator?.name ?? "Someone"}</span>
            <span aria-hidden="true">·</span>
            <span className="tabular">{replyCountLabel(thread.replyCount)}</span>
            <span aria-hidden="true">·</span>
            <time dateTime={thread.lastActivityAt}>
              {lastReplyLabel(thread.lastActivityAt, now)}
            </time>
          </span>
        </span>
        <span className="thread-row-tags">
          {thread.status === "active" ? null : (
            <span className="thread-status" data-status={thread.status}>
              <Icon name={thread.status === "locked" ? "lock" : "archive"} size={12} />
              {THREAD_STATUS_LABEL[thread.status]}
            </span>
          )}
          {following ? (
            <span className="thread-following-tag" title="Following">
              <Icon name="bell" size={12} />
              <span className="visually-hidden">Following</span>
            </span>
          ) : null}
        </span>
      </button>
    </li>
  );
}

/**
 * The Threads pane: the room's threads by tab (Active, Following, Closed, All), most recently
 * active first, each with its starter, reply count, last activity, status and unread dot. The
 * list follows thread events live; choosing one opens it on top of the pane.
 */
export function ThreadsPane({ roomId }: { readonly roomId: number }) {
  const [tab, setTab] = useState<ThreadTab>(lastTab);
  const { openThread } = usePaneNavigation(roomId);
  const list = useStore((state) => state.roomThreads[roomId]);
  const threads = useStore((state) => state.threads);
  const memberships = useStore((state) => state.threadMemberships);
  const view = visibleThreads(tab, { list, threads, memberships });
  const filter = serverFilter(tab);

  useEffect(() => {
    void actions.threads.list(roomId, filter).catch(() => undefined);
  }, [roomId, filter]);

  const choose = (value: string) => {
    if (isThreadTab(value)) {
      lastTab = value;
      setTab(value);
    }
  };

  const retry = () => void actions.threads.list(roomId, filter).catch(() => undefined);

  return (
    <PaneFrame
      title="Threads"
      subtitle={<RoomName roomId={roomId} />}
      toolbar={<Tabs items={THREAD_TABS} value={tab} onValueChange={choose} label="Show threads" />}
    >
      {view.status === "error" && view.threads.length === 0 ? (
        <PaneError message="The threads couldn't be loaded." onRetry={retry} />
      ) : (
        <SkeletonReveal loading={view.status === "loading"} skeleton={<PaneListSkeleton />}>
          {view.threads.length === 0 ? (
            view.status === "loading" ? null : (
              <PaneEmpty icon="thread" title="Nothing here" text={EMPTY_TAB_TEXT[tab]} />
            )
          ) : (
            <ul
              className="thread-list"
              aria-label={`${THREAD_TABS.find((t) => t.value === tab)?.label} threads`}
            >
              {view.threads.map((thread) => (
                <ThreadRow key={thread.id} thread={thread} onOpen={() => openThread(thread.id)} />
              ))}
            </ul>
          )}
        </SkeletonReveal>
      )}
    </PaneFrame>
  );
}
