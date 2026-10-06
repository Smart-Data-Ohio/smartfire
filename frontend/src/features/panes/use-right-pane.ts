import { useMatchRoute, useNavigate, useParams, useSearch } from "@tanstack/react-router";
import { useEffect, useState } from "react";
import {
  closeStep,
  isPaneShowing,
  type RightPaneView,
  selectRightPaneView,
} from "./pane-selection.ts";
import { openPane, type PaneKind, togglePane, useOpenPane } from "./pane-store.ts";

/** What the right pane shows now: the thread or draft in the URL, else the open side pane. */
export function useRightPaneView(): RightPaneView | null {
  const params = useParams({ strict: false });
  const search = useSearch({ strict: false });
  const matchRoute = useMatchRoute();
  const pane = useOpenPane();
  const drafting = matchRoute({ to: "/r/$roomId/t/new" }) !== false;

  return selectRightPaneView({
    threadId: params.threadId ?? null,
    newThreadParent: drafting ? (search.parent ?? null) : null,
    openPane: pane,
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

/** The right pane's navigation for one room. */
export function usePaneNavigation(roomId: number): PaneNavigation {
  const navigate = useNavigate();
  const view = useRightPaneView();

  const leaveThread = () => {
    void navigate({ to: "/r/$roomId", params: { roomId } });
  };

  return {
    view,
    openThread: (threadId, options) => {
      void navigate({
        to: "/r/$roomId/t/$threadId",
        params: { roomId, threadId },
        replace: options?.replace ?? false,
      });
    },
    closeTop: () => {
      const step = closeStep(view);

      if (step === "leave-thread") {
        leaveThread();
      } else if (step === "close-pane") {
        openPane(null);
      }
    },
    closeAll: () => {
      openPane(null);

      if (view !== null && view.kind !== "pane") {
        leaveThread();
      }
    },
    toggle: (pane) => {
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
