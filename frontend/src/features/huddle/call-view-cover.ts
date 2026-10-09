/**
 * On a phone the call view covers the whole conversation, so it behaves as a page pushed over
 * it: the room under it goes inert, it holds focus while open and gives it back to the call
 * bar's "Show call" when it closes, and it owns a history entry, so Back closes it rather than
 * leaving the room.
 */
import { useLocation, useRouter } from "@tanstack/react-router";
import { type RefObject, useEffect, useRef } from "react";
import { usePhoneLayout } from "../panes/use-right-pane.ts";
import { callController } from "./call-controller.ts";
import { type CallState, useCall } from "./call-store.ts";

declare module "@tanstack/react-router" {
  interface HistoryState {
    /** The entry the phone's call view pushed over the room: Back closes the view. */
    readonly smartfireCallView?: boolean;
  }
}

/** Whether `roomId`'s call view shows (in its room, open, and in a call that's on or coming). */
export function callViewShown(state: CallState, roomId: number): boolean {
  return (
    state.roomId === roomId &&
    state.viewOpen &&
    (state.phase === "connecting" || state.phase === "connected" || state.phase === "reconnecting")
  );
}

/** Whether `roomId`'s call view covers its conversation (a phone). */
export function useCallViewCovers(roomId: number): boolean {
  const phone = usePhoneLayout();
  const shown = useCall((state) => callViewShown(state, roomId));

  return phone && shown;
}

/**
 * The covering view's history entry, which lets Back close it: pushed when it opens, stepped
 * back over when it closes some other way (Hide call, Escape, leaving the call), and closing the
 * view when Back pops it. One per room: the call view keeps it.
 */
export function useCallViewHistory(covers: boolean): void {
  const router = useRouter();
  const marked = useLocation({ select: (location) => location.state.smartfireCallView === true });
  // Our entry is on top (`marked` has been seen while covering), or is being pushed.
  const entry = useRef<"none" | "pushing" | "top">("none");

  useEffect(() => {
    if (covers) {
      if (marked) {
        entry.current = "top";
      } else if (entry.current === "top") {
        // Back (or a navigation) took the entry away: the view goes with it.
        entry.current = "none";
        callController.setViewOpen(false);
      } else if (entry.current === "none") {
        entry.current = "pushing";
        router.history.push(router.history.location.href, { smartfireCallView: true });
      }

      return;
    }

    entry.current = "none";

    if (marked) {
      router.history.back();
    }
  }, [covers, marked, router]);
}

/**
 * The covering view's focus: moves into `view` when it opens, and back to the call bar's toggle
 * when it closes with focus inside it (Escape, Back), so nothing is left focused under the view.
 */
export function useCallViewFocus(view: RefObject<HTMLElement | null>, covers: boolean): void {
  useEffect(() => {
    const element = view.current;

    if (!covers || element === null) {
      return;
    }

    if (!element.contains(document.activeElement)) {
      element.focus();
    }

    return () => {
      const active = document.activeElement;

      if (active === null || active === document.body || element.contains(active)) {
        document.querySelector<HTMLElement>(".app-main-dock [data-call-view-toggle]")?.focus();
      }
    };
  }, [view, covers]);
}
