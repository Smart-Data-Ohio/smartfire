import { type ReactElement, type ReactNode, useEffect, useRef } from "react";
import { VList, type VListHandle } from "virtua";
import { Button, Spinner } from "../../ui/button.tsx";
import { SkeletonReveal } from "../../ui/skeleton.tsx";
import { PaneError, PaneListSkeleton } from "../panes/pane-states.tsx";
import { useKeepRowFocus } from "./row-focus.ts";
import "../panes/panes.css";

/** How close to the end (px) the next page starts loading. */
const PAGE_AHEAD = 600;

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
  /** Rows and section headings, each keyed, in order. */
  readonly children: readonly ReactElement[];
}

/**
 * A destination's list on virtua: a skeleton while the first page loads (revealed with the
 * skeleton-reveal recipe), an error with Retry, the empty state, or the rows; the next page loads
 * as the end nears (and at once while the rows don't fill the view), with a spinner, or a Retry
 * when it failed. When the row holding focus leaves, focus moves to its neighbour (row-focus.ts).
 */
export function PagedList({ state, label, errorText, empty, isEmpty, children }: PagedListProps) {
  const rootRef = useRef<HTMLDivElement | null>(null);
  const listRef = useRef<VListHandle | null>(null);
  const { status, loadingMore, hasMore, error, loadMore } = state;
  const canLoad = status === "ready" && hasMore && !loadingMore && error === null;

  useKeepRowFocus(rootRef);

  // A short first page leaves nothing to scroll: keep loading until the view fills.
  useEffect(() => {
    const list = listRef.current;

    if (canLoad && list !== null && list.scrollSize <= list.viewportSize + PAGE_AHEAD) {
      loadMore();
    }
  });

  if (status === "error") {
    return <PaneError message={errorText} onRetry={state.reload} />;
  }

  const onScroll = (offset: number) => {
    const list = listRef.current;

    if (list !== null && canLoad && list.scrollSize - offset - list.viewportSize < PAGE_AHEAD) {
      loadMore();
    }
  };

  const footer =
    error !== null && status === "ready" ? (
      <div key="more-error" className="page-more">
        <span className="page-more-error" role="alert">
          {error}
          <Button variant="ghost" size="sm" icon="rotate-ccw" onClick={loadMore}>
            Try again
          </Button>
        </span>
      </div>
    ) : loadingMore ? (
      <div key="more-loading" className="page-more" role="status">
        <Spinner label="Loading more" />
      </div>
    ) : null;

  return (
    <div ref={rootRef} className="page-list">
      <SkeletonReveal
        loading={status !== "ready"}
        skeleton={
          <div className="page-skeleton">
            <PaneListSkeleton rows={7} square={36} />
          </div>
        }
      >
        {isEmpty ? (
          empty
        ) : (
          <VList
            ref={listRef}
            className="page-list-scroll"
            bufferSize={400}
            onScroll={onScroll}
            role="list"
            aria-label={label}
            data-list-root=""
          >
            {footer === null ? children : [...children, footer]}
          </VList>
        )}
      </SkeletonReveal>
    </div>
  );
}
