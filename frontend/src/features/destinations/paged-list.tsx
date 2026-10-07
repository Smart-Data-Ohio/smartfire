import { type KeyboardEvent, type ReactElement, type ReactNode, useEffect, useRef } from "react";
import { VList, type VListHandle } from "virtua";
import { Button, Spinner } from "../../ui/button.tsx";
import { SkeletonReveal } from "../../ui/skeleton.tsx";
import { PaneError, PaneListSkeleton } from "../panes/pane-states.tsx";
import { edgeRowOpen, useKeepRowFocus } from "./row-focus.ts";
import "../panes/panes.css";

/** How close to the end (px) the next page starts loading. */
const PAGE_AHEAD = 600;

/** How many frames Home/End waits for the virtual list to draw its first or last row. */
const EDGE_WAIT_FRAMES = 30;

/** A list's loading state, as the data layer's paged views report it. */
export interface PagedState {
  /** The first page: `idle`/`loading` (skeleton), `ready`, or `error` (nothing shown). */
  readonly status: "idle" | "loading" | "ready" | "error";
  readonly loadingMore: boolean;
  readonly hasMore: boolean;
  readonly error: string | null;
  readonly loadMore: () => void;
  readonly reload: () => void;
}

interface PagedListProps {
  readonly state: PagedState;
  /** The list's accessible name. */
  readonly label: string;
  /** What a failed first load says. */
  readonly errorText: string;
  /** Shown when the first page came back empty. */
  readonly empty: ReactNode;
  /** True when there's nothing to show (sections included). */
  readonly isEmpty: boolean;
  /** Rows and section headings (each a listitem too), each keyed, in order. */
  readonly children: readonly ReactElement[];
}

/**
 * A destination's list on virtua: a skeleton while the first page loads (revealed with the
 * skeleton-reveal recipe), an error with Retry, the empty state, or the rows; the next page loads
 * as the end nears (and at once while the rows don't fill the view), with a spinner, or a Retry
 * when it failed. When the row holding focus leaves, focus moves to its neighbour (row-focus.ts);
 * Home and End on a row go to the first or last loaded row.
 */
export function PagedList({ state, label, errorText, empty, isEmpty, children }: PagedListProps) {
  const rootRef = useRef<HTMLDivElement | null>(null);
  const listRef = useRef<VListHandle | null>(null);
  const edgeFrame = useRef<number | undefined>(undefined);
  const { status, loadingMore, hasMore, error, loadMore } = state;
  const canLoad = status === "ready" && hasMore && !loadingMore && error === null;
  // Rows handled or removed here can empty the loaded window while more wait on the server: that
  // is still loading, not "all caught up".
  const starved = isEmpty && hasMore;

  useKeepRowFocus(rootRef);

  // A short first page (or an emptied one) leaves nothing to scroll: keep loading until the view
  // fills.
  useEffect(() => {
    const list = listRef.current;

    if (
      canLoad &&
      (starved || (list !== null && list.scrollSize <= list.viewportSize + PAGE_AHEAD))
    ) {
      loadMore();
    }
  });

  useEffect(
    () => () => {
      if (edgeFrame.current !== undefined) {
        cancelAnimationFrame(edgeFrame.current);
      }
    },
    [],
  );

  if (status === "error") {
    return <PaneError message={errorText} onRetry={state.reload} />;
  }

  /** Scrolls the first or last loaded row into view, then focuses it once virtua has drawn it. */
  const focusEdge = (edge: "first" | "last") => {
    const list = listRef.current;

    if (list === null) {
      return;
    }

    list.scrollToIndex(edge === "first" ? 0 : children.length - 1, {
      align: edge === "first" ? "start" : "end",
    });

    if (edgeFrame.current !== undefined) {
      cancelAnimationFrame(edgeFrame.current);
    }

    let frames = 0;

    const attempt = () => {
      const root = rootRef.current;
      const target = root === null ? null : edgeRowOpen(root, edge);

      const atEdge =
        edge === "first"
          ? list.scrollOffset <= 1
          : list.scrollOffset + list.viewportSize >= list.scrollSize - 1;

      frames += 1;

      // A frame past reaching the edge, so the rows drawn are the edge's, not the old window's.
      if (target !== null && ((atEdge && frames > 1) || frames >= EDGE_WAIT_FRAMES)) {
        edgeFrame.current = undefined;
        target.focus({ preventScroll: true });
        target.scrollIntoView({ block: "nearest" });
      } else if (frames < EDGE_WAIT_FRAMES) {
        edgeFrame.current = requestAnimationFrame(attempt);
      }
    };

    edgeFrame.current = requestAnimationFrame(attempt);
  };

  // Home and End from a row (not its hover actions or their menus) reach the list's edges.
  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    const target = event.target;
    const plain = !event.metaKey && !event.ctrlKey && !event.altKey && !event.shiftKey;

    if (
      event.defaultPrevented ||
      !plain ||
      (event.key !== "Home" && event.key !== "End") ||
      !(target instanceof Element) ||
      target.closest(".list-row") === null ||
      target.closest(".list-row-bar") !== null
    ) {
      return;
    }

    event.preventDefault();
    focusEdge(event.key === "Home" ? "first" : "last");
  };

  const onScroll = (offset: number) => {
    const list = listRef.current;

    if (list !== null && canLoad && list.scrollSize - offset - list.viewportSize < PAGE_AHEAD) {
      loadMore();
    }
  };

  const footer =
    error !== null && status === "ready" ? (
      <div className="page-more">
        <span className="page-more-error" role="alert">
          {error}
          <Button variant="ghost" size="sm" icon="rotate-ccw" onClick={loadMore}>
            Try again
          </Button>
        </span>
      </div>
    ) : loadingMore ? (
      <div className="page-more" role="status">
        <Spinner label="Loading more" />
      </div>
    ) : null;

  return (
    // biome-ignore lint/a11y/noStaticElementInteractions: it only hears Home and End bubbling up from a row's open button
    <div ref={rootRef} className="page-list" onKeyDown={onKeyDown}>
      <SkeletonReveal
        loading={status !== "ready" || (starved && error === null)}
        skeleton={
          <div className="page-skeleton">
            <PaneListSkeleton rows={7} square={36} />
          </div>
        }
      >
        {starved ? (
          error === null ? null : (
            footer
          )
        ) : isEmpty ? (
          empty
        ) : (
          <>
            <VList
              ref={listRef}
              className="page-list-scroll"
              bufferSize={400}
              onScroll={onScroll}
              role="list"
              aria-label={label}
              data-list-root=""
            >
              {children}
            </VList>
            {/* Under the list, not in it: a list holds only its items. */}
            {footer}
          </>
        )}
      </SkeletonReveal>
    </div>
  );
}
