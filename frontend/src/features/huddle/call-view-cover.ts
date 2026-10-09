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
    /** The room whose phone call view pushed this entry over it: Back closes the view. */
    readonly smartfireCallView?: number;
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
 * view when Back pops it. Leaving the room with the view open closes it too, so the entry it
 * leaves behind is one Back steps straight over (it isn't covering when that lands there), not
 * a second stop. One per room: the call view keeps it.
 */
export function useCallViewHistory(roomId: number, covers: boolean): void {
  const router = useRouter();

  // Only this room's entry: another room's call view, on its way out, mustn't act on it.
  const marked = useLocation({
    select: (location) => location.state.smartfireCallView === roomId,
  });

  // Our entry is on top (`marked` has been seen while covering), being pushed, or being left.
  const entry = useRef<"none" | "pushing" | "top" | "leaving">("none");

  // The room the view covered as of the last commit, for the cleanup below.
  const covering = useRef<number | null>(null);

  // Another room (or none) is showing: a view left open would cover it again on the way Back.
  useEffect(
    () => () => {
      if (covering.current === roomId) {
        covering.current = null;
        callController.setViewOpen(false);
      }
    },
    [roomId],
  );

  useEffect(() => {
    covering.current = covers ? roomId : null;

    if (covers) {
      if (marked) {
        entry.current = "top";
      } else if (entry.current === "top") {
        // Back (or a navigation) took the entry away: the view goes with it.
        entry.current = "none";
        callController.setViewOpen(false);
      } else if (entry.current !== "pushing") {
        entry.current = "pushing";
        router.history.push(router.history.location.href, { smartfireCallView: roomId });
      }

      return;
    }

    if (!marked) {
      entry.current = "none";
    } else if (entry.current !== "leaving") {
      // Closed some other way, or Back landed on an entry the view left behind: step over it
      // (once, though a remount's effect runs again before the step lands).
      entry.current = "leaving";
      router.history.back();
    }
  }, [covers, marked, router, roomId]);
}

/**
 * Escape anywhere in the covering view or on the call bar over it closes the view, unless
 * something in there (a menu, a popover) took the key first.
 */
export function useCallViewEscape(covers: boolean): void {
  useEffect(() => {
    if (!covers) {
      return;
    }

    const onKeyDown = (event: KeyboardEvent) => {
      if (
        event.key === "Escape" &&
        !event.defaultPrevented &&
        event.target instanceof Element &&
        event.target.closest(".call-view, .app-main-dock") !== null
      ) {
        event.preventDefault();
        callController.setViewOpen(false);
      }
    };

    window.addEventListener("keydown", onKeyDown);

    return () => window.removeEventListener("keydown", onKeyDown);
  }, [covers]);
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
