/** The Threads pane's tabs and which threads each one lists. */
import type { RoomThreadList, Thread, ThreadFilter, ThreadMembership } from "../../store/model.ts";

/** "Following" is the viewer's own view of every thread; the rest are the server's filters. */
export type ThreadTab = "active" | "following" | "closed" | "all";

export const THREAD_TABS: readonly { readonly value: ThreadTab; readonly label: string }[] = [
  { value: "active", label: "Active" },
  { value: "following", label: "Following" },
  { value: "closed", label: "Closed" },
  { value: "all", label: "All" },
];

export function isThreadTab(value: string): value is ThreadTab {
  return THREAD_TABS.some((tab) => tab.value === value);
}

/** The list the server is asked for: Following filters every thread on the client. */
export function serverFilter(tab: ThreadTab): ThreadFilter {
  return tab === "following" ? "all" : tab;
}

export interface ThreadListInput {
  readonly list: RoomThreadList | undefined;
  readonly threads: Readonly<Record<number, Thread>>;
  readonly memberships: Readonly<Record<number, ThreadMembership | null>>;
}

export interface ThreadListView {
  readonly status: "loading" | "ready" | "error";
  readonly threads: readonly Thread[];
}

/**
 * The threads a tab shows, most recently active first (the store keeps that order). A list
 * loaded for another filter doesn't count: the tab reads as loading until its own lands.
 */
export function visibleThreads(tab: ThreadTab, input: ThreadListInput): ThreadListView {
  const { list, threads, memberships } = input;

  if (list === undefined || list.filter !== serverFilter(tab)) {
    return { status: "loading", threads: [] };
  }

  const rows = list.ids.flatMap((id): Thread[] => {
    const thread = threads[id];

    if (thread === undefined) {
      return [];
    }

    return tab !== "following" || memberships[id]?.involvement === "everything" ? [thread] : [];
  });

  if (list.status === "error") {
    return { status: "error", threads: rows };
  }

  // A reload keeps the rows it had; only an empty list waits on the skeleton.
  return {
    status: list.status === "ready" || rows.length > 0 ? "ready" : "loading",
    threads: rows,
  };
}

/** What an empty tab says. */
export const EMPTY_TAB_TEXT = {
  active: "No active threads. Start one from any message with Reply in thread.",
  following: "You aren't following any threads. Follow one to hear about every reply.",
  closed: "No closed threads.",
  all: "No threads in this channel yet.",
} as const satisfies Record<ThreadTab, string>;
