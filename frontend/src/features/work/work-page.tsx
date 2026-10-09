import { useNavigate } from "@tanstack/react-router";
import { useEffect, useId } from "react";
import type { WorkFilter } from "../../gen/WorkFilter.ts";
import type { WorkListRow } from "../../gen/WorkListRow.ts";
import { useStore } from "../../store/store.ts";
import { WORK_FILTERS, workListOf } from "../../store/work.ts";
import { actions } from "../../sync/runtime.ts";
import { Tabs, tabId } from "../../ui/tabs.tsx";
import { PageFrame } from "../destinations/page-frame.tsx";
import { PagedList, type PagedState } from "../destinations/paged-list.tsx";
import { PaneEmpty } from "../panes/pane-states.tsx";
import { useNow } from "../threads/use-now.ts";
import { WORK_FILTER_EMPTY, WORK_FILTER_LABEL } from "./work-format.ts";
import { WorkRow } from "./work-row.tsx";
import "./work.css";

/** The page's tabs, in order. */
export const WORK_TABS = WORK_FILTERS.map((filter) => ({
  value: filter,
  label: WORK_FILTER_LABEL[filter],
}));

function isWorkFilter(value: string): value is WorkFilter {
  return WORK_FILTERS.some((filter) => filter === value);
}

interface WorkPageProps {
  readonly filter: WorkFilter;
  readonly onFilterChange: (filter: WorkFilter) => void;
}

/**
 * `/app/work`: every work thread the viewer can see, by tab (open, completed, all, owned by
 * agents, boards only), most recently updated first, all at once. A snapshot, as on the classic
 * page: it loads whenever it's shown (or the tab changes), and again when this browser tab
 * comes back into view, with no live updates.
 */
export function WorkPage({ filter, onFilterChange }: WorkPageProps) {
  const now = useNow();
  const navigate = useNavigate();
  const list = useStore((state) => workListOf(state, filter));
  const ids = useId();
  const filterTabs = `${ids}-filter`;
  const panelId = `${ids}-panel`;

  // Shown, or a new tab: load it (the rows already held stay up meanwhile).
  useEffect(() => {
    void actions.work.loadList(filter);
  }, [filter]);

  // A snapshot: coming back to this browser tab loads it again. A hidden tab doesn't.
  useEffect(() => {
    const refresh = () => {
      if (document.visibilityState === "visible") {
        void actions.work.loadList(filter);
      }
    };

    document.addEventListener("visibilitychange", refresh);
    window.addEventListener("focus", refresh);

    return () => {
      document.removeEventListener("visibilitychange", refresh);
      window.removeEventListener("focus", refresh);
    };
  }, [filter]);

  // A snapshot that reloads when shown: rows appear without motion.
  const rows = list.status === "ready" ? list.rows : [];

  const state: PagedState = {
    status: list.status,
    loadingMore: false,
    hasMore: false,
    error: null,
    loadMore: () => undefined,
    reload: () => void actions.work.loadList(filter),
  };

  const open = (row: WorkListRow) => {
    void navigate({
      to: "/r/$roomId/t/$threadId",
      params: { roomId: row.thread.roomId, threadId: row.thread.id },
    });
  };

  return (
    <PageFrame
      title="Work"
      icon="briefcase"
      back
      toolbar={
        <Tabs
          id={filterTabs}
          panelId={panelId}
          label="Show"
          items={WORK_TABS}
          value={filter}
          onValueChange={(value) => {
            if (isWorkFilter(value)) {
              onFilterChange(value);
            }
          }}
        />
      }
    >
      <div
        role="tabpanel"
        id={panelId}
        aria-labelledby={tabId(filterTabs, filter)}
        className="page-panel"
      >
        {list.status === "ready" && list.error !== null ? (
          <p className="work-page-stale" role="status">
            Couldn't refresh the list: {list.error}
          </p>
        ) : null}
        <PagedList
          state={state}
          label={WORK_FILTER_LABEL[filter]}
          errorText="The work list couldn't be loaded."
          isEmpty={rows.length === 0}
          empty={
            <PaneEmpty icon="briefcase" title="Nothing here" text={WORK_FILTER_EMPTY[filter]} />
          }
        >
          {rows.map((row) => (
            <WorkRow key={row.thread.id} row={row} now={now} motion={undefined} onOpen={open} />
          ))}
        </PagedList>
      </div>
    </PageFrame>
  );
}
