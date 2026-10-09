import { useMatchRoute, useNavigate, useParams, useSearch } from "@tanstack/react-router";
import { useEffect, useState } from "react";
import { parseBoardSearch } from "../../lib/board-search.ts";
import { useStore } from "../../store/store.ts";
import {
  closeStep,
  isPaneShowing,
  paneRoute,
  type RightPaneView,
  selectRightPaneView,
} from "./pane-selection.ts";
import {
  clearRoutePane,
  openPane,
  openRoutePane,
  type PaneKind,
  type RoutePaneKind,
  togglePane,
  useOpenPane,
  useRoutePaneReturn,
} from "./pane-store.ts";

/** Which classic side-pane page is currently in the URL. */
function useRoutePane(): RoutePaneKind | null {
  const matchRoute = useMatchRoute();

  if (matchRoute({ to: "/r/$roomId/threads" }) !== false) {
    return "threads";
  }

  if (matchRoute({ to: "/r/$roomId/files" }) !== false) {
    return "files";
  }

  if (matchRoute({ to: "/r/$roomId/automations" }) !== false) {
    return "automations";
  }

  return matchRoute({ to: "/r/$roomId/pins" }) === false ? null : "pins";
}

/** What the right pane shows now: the thread or draft in the URL, else the open side pane. */
export function useRightPaneView(): RightPaneView | null {
  const params = useParams({ strict: false });
  const search = useSearch({ strict: false });
  const matchRoute = useMatchRoute();
  const pane = useOpenPane(params.roomId ?? 0);
  const routePane = useRoutePane();

  const roomKind = useStore(
    (state) =>
      state.rooms[params.roomId ?? 0]?.detail?.room.kind ??
      state.sidebar.rows[params.roomId ?? 0]?.room.kind,
  );

  const drafting = matchRoute({ to: "/r/$roomId/t/new" }) !== false;
  // The notification URL is the room with its header menu open, never under a side pane. Read
  // at render, so the conversation is not inert when the menu mounts and takes focus.
  const posting = matchRoute({ to: "/r/$roomId/posts/new" }) !== false;
  const notifying = matchRoute({ to: "/r/$roomId/notifications" }) !== false;

  return selectRightPaneView({
    roomKind,
    newBoardPost: posting,
    threadId: params.threadId ?? null,
    newThreadParent: drafting ? (search.parent ?? null) : null,
    routePane,
    openPane: notifying ? null : pane,
  });
}

export interface PaneNavigation {
  readonly view: RightPaneView | null;
  /** Opens a thread on top of whatever side pane is open. */
  readonly openThread: (threadId: number, options?: { readonly replace?: boolean }) => void;
  /** Esc and the back button: a thread uncovers the side pane under it; a side pane closes. */
  readonly closeTop: () => void;
  /** The close button: the thread and the side pane both go. */
  readonly closeAll: () => void;
  /** A header button: shows its pane (leaving any thread) or, when it's showing, closes it. */
  readonly toggle: (pane: PaneKind) => void;
}

/** The persistent room owns URL pane memory; nested pane bodies only navigate. */
export function useRoomPaneLifecycle(roomId: number): void {
  const routePane = useRoutePane();
  const params = useParams({ strict: false });
  const matchRoute = useMatchRoute();
  const drafting = matchRoute({ to: "/r/$roomId/t/new" }) !== false;
  const posting = matchRoute({ to: "/r/$roomId/posts/new" }) !== false;
  const notifying = matchRoute({ to: "/r/$roomId/notifications" }) !== false;

  // Browser Back and room changes must not leave a URL pane as a local pane on the base room.
  useEffect(() => {
    if (routePane !== null) {
      openRoutePane(roomId, routePane);
    } else if (params.threadId === undefined && !drafting) {
      clearRoutePane();
    }
  }, [params.threadId, drafting, routePane, roomId]);

  // A side pane left open locally (say, opened after Escape and before Back) is hidden at once
  // on the notification URL (see useRightPaneView); this forgets it, so it doesn't reappear when
  // the menu closes.
  useEffect(() => {
    if (notifying || posting) {
      openPane(null);
    }
  }, [notifying, posting]);

  useEffect(() => () => clearRoutePane(roomId), [roomId]);
}

/** The right pane's navigation for one room. */
export function usePaneNavigation(roomId: number): PaneNavigation {
  const navigate = useNavigate();
  const view = useRightPaneView();
  const routePane = useRoutePane();
  const returnPane = useRoutePaneReturn(roomId);

  const leaveThread = () => {
    void navigate({ to: "/r/$roomId", params: { roomId }, search: parseBoardSearch });
  };

  return {
    view,
    openThread: (threadId, options) => {
      if (routePane !== null) {
        openRoutePane(roomId, routePane);
      }

      void navigate({
        to: "/r/$roomId/t/$threadId",
        params: { roomId, threadId },
        search: parseBoardSearch,
        replace: options?.replace ?? false,
      });
    },
    closeTop: () => {
      const step = closeStep(view);

      if (step === "leave-thread") {
        if (returnPane === null) {
          leaveThread();
        } else {
          const to = paneRoute(returnPane);

          if (to !== null) {
            void navigate({ to, params: { roomId }, search: parseBoardSearch });
          }
        }
      } else if (step === "close-pane") {
        openPane(null);

        if (routePane !== null) {
          leaveThread();
        }
      }
    },
    closeAll: () => {
      openPane(null);

      if (routePane !== null || (view !== null && view.kind !== "pane")) {
        leaveThread();
      }
    },
    toggle: (pane) => {
      if (pane === "automations") {
        openPane(null);

        if (isPaneShowing(view, pane)) {
          leaveThread();
        } else {
          void navigate({
            to: "/r/$roomId/automations",
            params: { roomId },
            search: parseBoardSearch,
          });
        }

        return;
      }

      if (routePane !== null) {
        openPane(null);

        if (isPaneShowing(view, pane)) {
          leaveThread();

          return;
        }

        const to = paneRoute(pane);

        if (to === null) {
          openPane(pane);
          leaveThread();
        } else {
          void navigate({ to, params: { roomId }, search: parseBoardSearch });
        }

        return;
      }

      if (view !== null && view.kind !== "pane") {
        openPane(pane);
        leaveThread();

        return;
      }

      if (isPaneShowing(view, pane)) {
        openPane(null);
      } else {
        togglePane(pane);
      }
    },
  };
}

const PHONE_QUERY = "(width < 720px)";

const OVERLAY_QUERY = "(width < 1100px)";

function useMedia(query: string): boolean {
  const [matches, setMatches] = useState(
    () => "matchMedia" in window && window.matchMedia(query).matches,
  );

  useEffect(() => {
    if (!("matchMedia" in window)) {
      return;
    }

    const media = window.matchMedia(query);
    const onChange = () => setMatches(media.matches);

    onChange();
    media.addEventListener("change", onChange);

    return () => media.removeEventListener("change", onChange);
  }, [query]);

  return matches;
}

/** Phones: the pane covers the conversation as a full-screen page. */
export function usePhoneLayout(): boolean {
  return useMedia(PHONE_QUERY);
}

/** Below 1100 px the pane floats over the conversation instead of taking a column. */
export function useOverlayLayout(): boolean {
  return useMedia(OVERLAY_QUERY);
}
