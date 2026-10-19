import { useLocation, useNavigate, useSearch } from "@tanstack/react-router";
import { useEffect, useRef } from "react";
import { parseBoardSearch } from "../../lib/board-search.ts";
import { boardAwaitsListing } from "../../store/boards.ts";
import { useStore } from "../../store/store.ts";
import { actions } from "../../sync/runtime.ts";
import { Skeleton, SkeletonReveal } from "../../ui/skeleton.tsx";
import { isPaneShowing } from "../panes/pane-selection.ts";
import { PaneError } from "../panes/pane-states.tsx";
import { usePaneNavigation, usePhoneLayout } from "../panes/use-right-pane.ts";
import { settingsOverState } from "../rooms/room-settings-host.tsx";
import {
  BoardEmpty,
  type BodyProps,
  Digest,
  LoadMore,
  useColumns,
  useOpenPostId,
  usePostIds,
} from "./board-body.tsx";
import {
  type BoardQuery,
  boardQuery,
  boardSearch,
  WORK_STATUS_LABEL,
  WORK_STATUSES,
} from "./board-format.ts";
import { NewPostFab, PhoneBoardColumns, PhonePostCard } from "./board-phone.tsx";
import { PostCard, PostRow } from "./board-rows.tsx";
import { BoardTagStrip, BoardToolbar } from "./board-toolbar.tsx";
import { NewPostDialog } from "./new-post-dialog.tsx";
import { DropSlot, useBoardDrag, withSlot } from "./use-board-drag.tsx";
import { useBoardTagCatalog } from "./use-board-tags.ts";
import { useLiveFlip } from "./use-live-flip.ts";
import "./boards.css";

const NONE: readonly never[] = [];

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

function BoardList({ roomId, query, onNewPost, onClear }: BodyProps) {
  const ids = usePostIds(roomId);
  const phone = usePhoneLayout();
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
          {ids.map((id) =>
            phone ? (
              <PhonePostCard
                key={id}
                roomId={roomId}
                threadId={id}
                open={id === openId}
                activeTag={query.tag}
                showStatus
              />
            ) : (
              <PostRow
                key={id}
                roomId={roomId}
                threadId={id}
                open={id === openId}
                activeTag={query.tag}
              />
            ),
          )}
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

  const dnd = useBoardDrag(roomId, boardRef);

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
      <div className="board-columns" ref={boardRef} data-dragging={dnd.drag ? true : undefined}>
        {WORK_STATUSES.map((status) => (
          <section
            key={status}
            className="board-column"
            data-status={status}
            data-flip-group={status}
            data-drop-target={dnd.landing?.status === status || undefined}
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
              {withSlot(columns[status], status, dnd.landing).map((id) =>
                id === "slot" ? (
                  <DropSlot key="slot" drag={dnd.drag} />
                ) : (
                  <PostCard
                    key={id}
                    roomId={roomId}
                    threadId={id}
                    open={id === openId}
                    activeTag={query.tag}
                    drag={dnd.card}
                  />
                ),
              )}
              {columns[status].length === 0 && dnd.landing?.status !== status ? (
                <li className="board-column-empty">No posts</li>
              ) : null}
            </ul>
          </section>
        ))}
      </div>
      {dnd.region}
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
  const catalog = useBoardTagCatalog(roomId)?.tags ?? NONE;
  const panes = usePaneNavigation(roomId);
  const phone = usePhoneLayout();
  const composing = pathname.endsWith("/posts/new");

  const waiting = useStore((state) => boardAwaitsListing(state.boards[roomId]));

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
        catalog={catalog}
        onManageTags={() =>
          void navigate({
            to: "/r/$roomId/settings",
            params: { roomId },
            search: boardSearch(query),
            state: settingsOverState(roomId, "tags"),
          })
        }
      />
      <BoardTagStrip catalog={catalog} active={query.tag} onChange={(tag) => change({ tag })} />
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
        <SkeletonReveal loading={waiting} skeleton={<BoardSkeleton />}>
          {waiting ? (
            <div className="board-scroll" />
          ) : query.view === "board" ? (
            phone ? (
              <PhoneBoardColumns {...body} />
            ) : (
              <BoardColumns {...body} />
            )
          ) : (
            <BoardList {...body} />
          )}
        </SkeletonReveal>
      )}
      {phone ? <NewPostFab onNewPost={openComposer} /> : null}
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
