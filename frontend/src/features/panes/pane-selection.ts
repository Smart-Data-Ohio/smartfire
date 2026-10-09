/**
 * What the right pane shows. A thread lives in the URL (`/r/$roomId/t/$threadId`, or
 * `/r/$roomId/t/new?parent=` while drafting one); a side pane can live in the URL or the per-tab
 * pane store. The URL wins, so a thread opened from the Threads pane sits on
 * top of it and closing the thread goes back to the list.
 */
import type { RoomKind } from "../../gen/RoomKind.ts";
import type { PaneKind, RoutePaneKind } from "./pane-store.ts";

export type RightPaneView =
  | { readonly kind: "thread"; readonly threadId: number }
  | { readonly kind: "new-thread"; readonly parentId: number }
  | { readonly kind: "pane"; readonly pane: PaneKind };

export interface PaneInputs {
  readonly roomKind?: RoomKind | undefined;
  readonly newBoardPost?: boolean;
  /** `$threadId` from the URL, when a thread route matched. */
  readonly threadId: number | null;
  /** `?parent=` on the new-thread route, when it matched. */
  readonly newThreadParent: number | null;
  /** A side-pane page in the URL wins over a locally remembered side pane. */
  readonly routePane?: RoutePaneKind | null;
  readonly openPane: PaneKind | null;
}

/** The thread route, then the new-thread route, then the side pane, else nothing. */
export function selectRightPaneView({
  roomKind,
  threadId,
  newThreadParent,
  newBoardPost = false,
  routePane = null,
  openPane,
}: PaneInputs): RightPaneView | null {
  if (newBoardPost) return null;

  if (threadId !== null) {
    return { kind: "thread", threadId };
  }

  if (newThreadParent !== null) {
    return { kind: "new-thread", parentId: newThreadParent };
  }

  const pane = routePane ?? openPane;

  if (pane === "automations" && roomKind !== "board") return null;

  return pane === null ? null : { kind: "pane", pane };
}

/** The URL of a side pane's classic page mapping, or null for a local-only pane. */
export function paneRoute(
  pane: PaneKind,
): "/r/$roomId/threads" | "/r/$roomId/files" | "/r/$roomId/pins" | "/r/$roomId/automations" | null {
  switch (pane) {
    case "threads":
      return "/r/$roomId/threads";
    case "files":
      return "/r/$roomId/files";
    case "pins":
      return "/r/$roomId/pins";
    case "automations":
      return "/r/$roomId/automations";
    case "members":
    case "stage":
    case "details":
      return null;
  }
}

/** A stable key per view: a different thread or pane remounts the pane's body. */
export function viewKey(view: RightPaneView): string {
  switch (view.kind) {
    case "thread":
      return `thread-${view.threadId}`;
    case "new-thread":
      return `new-thread-${view.parentId}`;
    case "pane":
      return `pane-${view.pane}`;
  }
}

/** A header button is pressed while its own pane is what's showing. */
export function isPaneShowing(view: RightPaneView | null, pane: PaneKind): boolean {
  return view?.kind === "pane" && view.pane === pane;
}

/**
 * What closing the top of the pane does: a thread (or a draft) leaves the URL and uncovers the
 * side pane under it, if any; a side pane closes.
 */
export type CloseStep = "leave-thread" | "close-pane" | "none";

export function closeStep(view: RightPaneView | null): CloseStep {
  if (view === null) {
    return "none";
  }

  return view.kind === "pane" ? "close-pane" : "leave-thread";
}

/** The label of the back button that uncovers the side pane under a thread. */
export const PANE_TITLES = {
  automations: "Automations",
  members: "Members",
  pins: "Pinned messages",
  files: "Files",
  threads: "Threads",
  stage: "Stage",
  details: "Details",
} as const satisfies Record<PaneKind, string>;
