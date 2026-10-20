import { useStore as useZustandStore } from "zustand";
import { useShallow } from "zustand/react/shallow";
import type { WorkStatus } from "../../gen/WorkStatus.ts";
import { boardColumns, boardPostIds } from "../../store/boards.ts";
import { store, useStore } from "../../store/store.ts";
import { actions } from "../../sync/runtime.ts";
import { Button } from "../../ui/button.tsx";
import { Icon } from "../../ui/icons/icon.tsx";
import { useRightPaneView } from "../panes/use-right-pane.ts";
import { type BoardQuery, digestDate, WORK_STATUSES } from "./board-format.ts";

/** What a board's list and its column views share: the posts they show and their states. */

const DESCRIPTION =
  "Work threads with a status, an owner, tags, and a pinned result. Open a post to discuss it and record its outcome.";

/** The post open in the right pane, to mark its row. */
export function useOpenPostId(): number | null {
  const view = useRightPaneView();

  return view?.kind === "thread" ? view.threadId : null;
}

/** The posts that match the board's filters right now, live facts applied, newest activity first. */
export function usePostIds(roomId: number): readonly number[] {
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
export function useColumns(roomId: number): Readonly<Record<WorkStatus, readonly number[]>> {
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

/** "No posts yet." with a way to start one, or "No posts match these filters." with a way out. */
export function BoardEmpty({
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
export function Digest({ roomId }: { readonly roomId: number }) {
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

export function LoadMore({ roomId }: { readonly roomId: number }) {
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

export interface BodyProps {
  readonly roomId: number;
  readonly query: BoardQuery;
  readonly onNewPost: (status: WorkStatus) => void;
  readonly onClear: () => void;
}
