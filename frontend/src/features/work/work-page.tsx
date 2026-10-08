import { useId } from "react";
import type { WorkFilter } from "../../gen/WorkFilter.ts";
import { Tabs, tabId } from "../../ui/tabs.tsx";
import { PageFrame } from "../destinations/page-frame.tsx";
import { PagedList } from "../destinations/paged-list.tsx";
import { PaneEmpty } from "../panes/pane-states.tsx";
import { useNow } from "../threads/use-now.ts";
import { useWorkList } from "./use-work-list.ts";
import { WorkRow } from "./work-row.tsx";
import { isWorkFilter } from "./work-search.ts";
import "../boards/boards.css";
import "./work.css";

/** The page's filter tabs (`?state=`), in the classic page's order. */
export const WORK_TABS = [
  { value: "open", label: "Open" },
  { value: "done", label: "Done" },
  { value: "all", label: "All" },
  { value: "agents", label: "Agents" },
  { value: "boards", label: "Boards" },
] as const satisfies readonly { value: WorkFilter; label: string }[];

/** Each filter's empty state, worded as the classic page words it. */
const EMPTY = {
  open: {
    title: "No open work",
    text: "No open work yet. Track a channel thread as work and it will appear here.",
  },
  done: {
    title: "Nothing completed",
    text: "No completed work yet. Work shows up here once its status is set to Done.",
  },
  all: {
    title: "No work threads",
    text: "No work threads yet. Track a channel thread as work and it will appear here.",
  },
  agents: {
    title: "No agent-owned work",
    text: "No agent-owned work yet. Assign a work thread to an agent and it will appear here.",
  },
  boards: {
    title: "No board work",
    text: "No board work yet. Create a post in one of your boards and it will appear here.",
  },
} as const satisfies Record<WorkFilter, { title: string; text: string }>;

interface WorkPageProps {
  readonly filter: WorkFilter;
  readonly onFilterChange: (filter: WorkFilter) => void;
}

/**
 * `/app/work` (classic `work_threads#index`): tracked threads from every room the viewer is in,
 * most recently updated first, filtered to open, done, all, agent-owned or board work. Each row
 * opens its thread.
 */
export function WorkPage({ filter, onFilterChange }: WorkPageProps) {
  const now = useNow();
  const view = useWorkList(filter);
  const ids = useId();
  const filterTabs = `${ids}-filter`;
  const panelId = `${ids}-panel`;
  const rows = view.list?.threads ?? [];
  const empty = EMPTY[filter];

  return (
    <PageFrame
      title="Work"
      icon="list-checks"
      back
      meta={
        view.status === "ready" && rows.length > 0 ? (
          <span className="page-count" title={`${rows.length} work threads`}>
            {rows.length}
          </span>
        ) : null
      }
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
        <PagedList
          key={filter}
          state={view}
          label="Work threads"
          errorText="Your work couldn't be loaded."
          isEmpty={rows.length === 0}
          empty={<PaneEmpty icon="list-checks" title={empty.title} text={empty.text} />}
        >
          {rows.map((row) => (
            <WorkRow key={row.thread.id} row={row} now={now} />
          ))}
        </PagedList>
      </div>
    </PageFrame>
  );
}
