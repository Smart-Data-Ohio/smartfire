import { useLocation, useNavigate, useSearch } from "@tanstack/react-router";
import { useEffect, useRef } from "react";
import { useStore as useZustandStore } from "zustand";
import { useShallow } from "zustand/react/shallow";
import type { WorkStatus } from "../../gen/WorkStatus.ts";
import { parseBoardSearch } from "../../lib/board-search.ts";
import { boardColumns, boardPostIds } from "../../store/boards.ts";
import { store, useStore } from "../../store/store.ts";
import { actions } from "../../sync/runtime.ts";
import { Button } from "../../ui/button.tsx";
import { Icon } from "../../ui/icons/icon.tsx";
import { Skeleton, SkeletonReveal } from "../../ui/skeleton.tsx";
import { isPaneShowing } from "../panes/pane-selection.ts";
import { PaneError } from "../panes/pane-states.tsx";
import { usePaneNavigation, useRightPaneView } from "../panes/use-right-pane.ts";
import {
  type BoardQuery,
  boardQuery,
  boardSearch,
  digestDate,
  WORK_STATUS_LABEL,
  WORK_STATUSES,
} from "./board-format.ts";
import { PostCard, PostRow } from "./board-rows.tsx";
import { BoardToolbar } from "./board-toolbar.tsx";
import { NewPostDialog } from "./new-post-dialog.tsx";
import { useLiveFlip } from "./use-live-flip.ts";
import "./boards.css";

const NONE: readonly never[] = [];

const DESCRIPTION =
  "Work threads with a status, an owner, tags, and a pinned result. Open a post to discuss it and record its outcome.";

/** The post open in the right pane, to mark its row. */
function useOpenPostId(): number | null {
  const view = useRightPaneView();

  return view?.kind === "thread" ? view.threadId : null;
}

/** The posts that match the board's filters right now, live facts applied, newest activity first. */
function usePostIds(roomId: number): readonly number[] {
  return useZustandStore(
    store,
    useShallow((state) => boardPostIds(state, roomId)),
  );
}

/** One column's ids from its part of the signature ("" is an empty column). */
function columnIds(part: string | undefined): number[] {
  return part === undefined || part === "" ? [] : part.split(",").map(Number);
}

/**
 * The column view's ids. The selector answers a string ("1,2|3||4": each column's ids in status
 * order), which only changes when a post moves, so the board doesn't re-render on every store
 * write.
 */
function useColumns(roomId: number): Readonly<Record<WorkStatus, readonly number[]>> {
  const signature = useStore((state) => {
    const columns = boardColumns(state, roomId);

    return WORK_STATUSES.map((status) => columns[status].join(",")).join("|");
  });

  const parts = signature.split("|");

  return {
    planned: columnIds(parts[0]),
    in_progress: columnIds(parts[1]),
    blocked: columnIds(parts[2]),
    done: columnIds(parts[3]),
  };
}

function BoardSkeleton() {
  return (
    <div className="board-skeleton" aria-hidden="true">
      {[62, 44, 71, 38, 55].map((width) => (
        <div key={width} className="board-skeleton-row">
          <div className="board-skeleton-lines">
            <Skeleton width={`${width}%`} height={13} />
            <Skeleton width={`${Math.round(width * 0.7)}%`} height={10} />
          </div>
          <Skeleton width={72} height={20} radius="pill" />
        </div>
      ))}
    </div>
  );
}

/** "No posts yet." with a way to start one, or "No posts match these filters." with a way out. */
function BoardEmpty({
  anyPosts,
  onNewPost,
  onClear,
}: {
  readonly anyPosts: boolean;
  readonly onNewPost: () => void;
  readonly onClear: () => void;
}) {
  return (
    <div className="board-empty enter-fade">
      <span className="board-empty-glyph" aria-hidden="true">
        <Icon name="boards" size={22} />
      </span>
      {anyPosts ? (
        <>
          <p className="board-empty-title">No posts match these filters.</p>
          <Button variant="secondary" size="sm" onClick={onClear}>
            Clear filters
          </Button>
        </>
      ) : (
        <>
          <p className="board-empty-title">No posts yet. Start the first one.</p>
          <p className="board-empty-text">{DESCRIPTION}</p>
          <Button variant="primary" size="sm" icon="plus" onClick={onNewPost}>
            New post
          </Button>
        </>
      )}
    </div>
  );
}

/** The newest stale-work digest the board posted, folded to a few lines. */
function Digest({ roomId }: { readonly roomId: number }) {
  const digest = useStore((state) => state.boards[roomId]?.digest ?? null);

  if (digest === null) {
    return null;
  }

  return (
    <details className="board-digest">
      <summary className="board-digest-summary">
        <Icon name="alarm-clock" size={14} />
        <span>Stale-work digest · {digestDate(digest.date)}</span>
        <Icon name="chevron-down" size={14} className="board-digest-chevron" />
      </summary>
      <p className="board-digest-text">{digest.text}</p>
    </details>
  );
}

function LoadMore({ roomId }: { readonly roomId: number }) {
  const hasMore = useStore((state) => state.boards[roomId]?.hasMore ?? false);
  const loading = useStore((state) => state.boards[roomId]?.loadingMore ?? false);

  if (!hasMore) {
    return null;
  }

  return (
    <div className="board-more">
      <Button
        variant="secondary"
        size="sm"
        loading={loading}
        onClick={() => void actions.boards.loadMore(roomId).catch(() => undefined)}
      >
        Load more
      </Button>
    </div>
  );
}

interface BodyProps {
  readonly roomId: number;
  readonly query: BoardQuery;
  readonly onNewPost: () => void;
  readonly onClear: () => void;
}

function BoardList({ roomId, query, onNewPost, onClear }: BodyProps) {
  const ids = usePostIds(roomId);
  const anyPosts = useStore((state) => state.boards[roomId]?.anyPosts ?? false);
  const openId = useOpenPostId();
  const listRef = useRef<HTMLDivElement | null>(null);

  useLiveFlip(listRef, ids.join(","), `${query.status}|${query.owner}|${query.tag}`);

  return (
    <div className="board-scroll" ref={listRef}>
      <Digest roomId={roomId} />
      {ids.length === 0 ? (
        <BoardEmpty anyPosts={anyPosts || ids.length > 0} onNewPost={onNewPost} onClear={onClear} />
      ) : (
        <ul className="board-list" aria-label="Posts">
          {ids.map((id) => (
            <PostRow
              key={id}
              roomId={roomId}
              threadId={id}
              open={id === openId}
              activeTag={query.tag}
            />
          ))}
        </ul>
      )}
      <LoadMore roomId={roomId} />
    </div>
  );
}

function BoardColumns({ roomId, query, onNewPost, onClear }: BodyProps) {
  const columns = useColumns(roomId);
  const anyPosts = useStore((state) => state.boards[roomId]?.anyPosts ?? false);
  const openId = useOpenPostId();
  const boardRef = useRef<HTMLDivElement | null>(null);
  const total = WORK_STATUSES.reduce((sum, status) => sum + columns[status].length, 0);

  const signature = WORK_STATUSES.map((status) => `${status}:${columns[status].join(",")}`).join(
    "|",
  );

  useLiveFlip(boardRef, signature, `${query.owner}|${query.tag}`);

  if (total === 0 && !anyPosts) {
    return (
      <div className="board-scroll">
        <BoardEmpty anyPosts={false} onNewPost={onNewPost} onClear={onClear} />
      </div>
    );
  }

  return (
    <div className="board-columns-wrap">
      <Digest roomId={roomId} />
      <div className="board-columns" ref={boardRef}>
        {WORK_STATUSES.map((status) => (
          <section
            key={status}
            className="board-column"
            data-status={status}
            data-flip-group={status}
            aria-labelledby={`board-column-${roomId}-${status}`}
          >
            <header className="board-column-head">
              <span className="board-status-dot" aria-hidden="true" />
              <h2 id={`board-column-${roomId}-${status}`} className="board-column-title">
                {WORK_STATUS_LABEL[status]}
              </h2>
              <span className="board-column-count tabular">{columns[status].length}</span>
            </header>
            <ul className="board-column-list" data-flip-scroll>
              {columns[status].map((id) => (
                <PostCard
                  key={id}
                  roomId={roomId}
                  threadId={id}
                  open={id === openId}
                  activeTag={query.tag}
                />
              ))}
              {columns[status].length === 0 ? (
                <li className="board-column-empty">No posts</li>
              ) : null}
            </ul>
          </section>
        ))}
      </div>
      <LoadMore roomId={roomId} />
    </div>
  );
}

/**
 * A board room in place of the timeline: the toolbar, then its posts as a list or in four status
 * columns, filtered by status (list only), owner and tag from the URL (the classic `status`,
 * `owner` and `tag` params, plus `view`). Rows follow the posts live: a post that changes status,
 * owner or tags, or gets a reply, glides to its new place or leaves the filtered list.
 * `/r/:roomId/posts/new` opens the new-post dialog over it.
 */
export function BoardView({ roomId }: { readonly roomId: number }) {
  const search = parseBoardSearch(useSearch({ strict: false }));
  const navigate = useNavigate();
  const { pathname } = useLocation();
  const query = boardQuery(search);
  const status = useStore((state) => state.boards[roomId]?.status ?? "loading");
  const error = useStore((state) => state.boards[roomId]?.error ?? null);
  const ownerOptions = useStore((state) => state.boards[roomId]?.ownerOptions ?? NONE);
  const tagCounts = useStore((state) => state.boards[roomId]?.tagCounts ?? NONE);
  const canAdminister = useStore((state) => state.boards[roomId]?.canAdminister ?? false);
  const panes = usePaneNavigation(roomId);
  const composing = pathname.endsWith("/posts/new");

  useEffect(() => {
    void actions.boards
      .open(roomId, { status: query.status, owner: query.owner, tag: query.tag })
      .catch(() => undefined);
  }, [roomId, query.status, query.owner, query.tag]);

  const change = (next: Partial<BoardQuery>) => {
    void navigate({
      to: ".",
      search: boardSearch({
        view: next.view ?? query.view,
        // The column view ignores the status filter; back in the list, the URL's (or Open) holds.
        status: next.status ?? search.status ?? "open",
        owner: next.owner ?? query.owner,
        tag: next.tag ?? query.tag,
      }),
      replace: true,
    });
  };

  const openComposer = () => {
    void navigate({
      to: "/r/$roomId/posts/new",
      params: { roomId },
      search: boardSearch(query),
    });
  };

  const closeComposer = () => {
    void navigate({ to: "/r/$roomId", params: { roomId }, search: boardSearch(query) });
  };

  const clear = () => change({ status: "open", owner: "anyone", tag: "" });
  const body = { roomId, query, onNewPost: openComposer, onClear: clear };

  return (
    <div className="board" data-view={query.view}>
      <BoardToolbar
        query={query}
        ownerOptions={ownerOptions}
        tagCounts={tagCounts}
        onChange={change}
        onNewPost={openComposer}
        canAdminister={canAdminister}
        automationsOpen={isPaneShowing(panes.view, "automations")}
        onAutomations={() => panes.toggle("automations")}
      />
      {status === "error" ? (
        <PaneError
          message={error ?? "This board couldn't be loaded."}
          onRetry={() =>
            void actions.boards
              .open(roomId, { status: query.status, owner: query.owner, tag: query.tag })
              .catch(() => undefined)
          }
        />
      ) : (
        <SkeletonReveal loading={status === "loading"} skeleton={<BoardSkeleton />}>
          {status === "loading" ? (
            <div className="board-scroll" />
          ) : query.view === "board" ? (
            <BoardColumns {...body} />
          ) : (
            <BoardList {...body} />
          )}
        </SkeletonReveal>
      )}
      <NewPostDialog
        roomId={roomId}
        open={composing}
        onClose={closeComposer}
        onCreated={(threadId) =>
          void navigate({
            to: "/r/$roomId/t/$threadId",
            params: { roomId, threadId },
            search: boardSearch(query),
          })
        }
      />
    </div>
  );
}
