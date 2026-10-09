/**
 * On a phone the call view covers the whole conversation, so it behaves as a page over it: it is
 * open while the room's URL carries `call=1` (Back closes it, Forward reopens it, another room's
 * URL hasn't got it), the room under it goes inert, it holds focus while open and gives it back
 * to the call bar's "Show call" when it closes. Desktop keeps the controller's `viewOpen`.
 */
import {
  useLocation,
  useMatchRoute,
  useNavigate,
  useRouter,
  useSearch,
} from "@tanstack/react-router";
import { type RefObject, useEffect, useRef } from "react";
import { parseBoardSearch, type RoomSearch } from "../../lib/board-search.ts";
import { usePhoneLayout } from "../panes/use-right-pane.ts";
import { type CallState, useCall } from "./call-store.ts";

declare module "@tanstack/react-router" {
  interface HistoryState {
    /** "Show call" pushed this entry over the room's own page: closing steps back to it. */
    readonly smartfireCallOver?: boolean;
  }
}

/** Whether `roomId`'s call is on (or coming), so it has a view to show. */
export function callActiveIn(state: CallState, roomId: number): boolean {
  return (
    state.roomId === roomId &&
    (state.phase === "connecting" || state.phase === "connected" || state.phase === "reconnecting")
  );
}

/** Whether the URL asks for the call view over the room (`call=1`). */
function useCallParam(): boolean {
  return useSearch({ strict: false, select: (search) => search.call === 1 });
}

/** Whether `roomId`'s call view covers its conversation: a phone, its call on, and `call=1`. */
export function useCallViewCovers(roomId: number): boolean {
  const phone = usePhoneLayout();
  const active = useCall((state) => callActiveIn(state, roomId));
  const asked = useCallParam();

  return phone && active && asked;
}

export interface CallViewNavigation {
  /** Pushes `roomId`'s page with the call view over it. */
  readonly open: (roomId: number) => void;
  /**
   * Takes `call=1` off this page. A person closing the view (`"user"`) on the entry "Show call"
   * pushed over the room's page steps back to that page; anything else, and every automatic
   * cleanup (`"replace"`), replaces this entry with the same URL less `call`.
   */
  readonly close: (how: "user" | "replace") => void;
}

/** This search less `call`, whatever else it holds (board filters, a reply's `m`, ...). */
function withoutCall<Search extends { readonly call?: unknown }>(
  search: Search,
): Omit<Search, "call"> {
  const { call: _call, ...rest } = search;

  return rest;
}

/** Opening and closing the phone's call view, as navigations that add or remove `call=1`. */
export function useCallViewNavigation(): CallViewNavigation {
  const navigate = useNavigate();
  const router = useRouter();
  const matchRoute = useMatchRoute();

  return {
    open: (roomId) => {
      // Over the room's own page, closing can step back to it; from anywhere else it replaces.
      const match = matchRoute({ to: "/r/$roomId" });

      // (Given `params: { roomId }`, matchRoute never matched the numeric id: compare it here.)
      const onRoomPage =
        match !== false && Number(match.roomId) === roomId && !carriesCallParam(router);

      void navigate({
        to: "/r/$roomId",
        params: { roomId },
        search: (previous): RoomSearch =>
          onRoomPage ? { ...parseBoardSearch(previous), call: 1 } : { call: 1 },
        state: onRoomPage ? { smartfireCallOver: true } : {},
      });
    },
    close: (how) => {
      const location = router.state.location;

      if (closing === location || !carriesCallParam(router)) {
        return;
      }

      closing = location;

      if (how === "user" && location.state.smartfireCallOver === true) {
        router.history.back();

        return;
      }

      void navigate({
        to: ".",
        search: (previous) => withoutCall(previous),
        hash: true,
        state: (previous) => {
          const { smartfireCallOver: _over, ...rest } = previous;

          return rest;
        },
        replace: true,
      });
    },
  };
}

/** Whether the current location carries `call=1` (read at the moment, outside render). */
function carriesCallParam(router: ReturnType<typeof useRouter>): boolean {
  return router.state.location.search.call === 1;
}

/** The location a close is already leaving, so a second Escape or tap before it lands is a no-op. */
let closing: object | null = null;

/**
 * Keeps `call=1` honest: where it can't show (a desktop, no call on in the room) it comes off the
 * URL by replacing this entry. Only a call that ends while its view shows on this very entry
 * counts as the person closing it (which may step back); arriving at a stale `call=1` (Back,
 * Forward, a reload, a link) never does.
 */
export function useCallParamCleanup(roomId: number, close: CallViewNavigation["close"]): void {
  const phone = usePhoneLayout();
  const active = useCall((state) => callActiveIn(state, roomId));
  const asked = useCallParam();
  const entry = useLocation({ select: (location) => location.state.__TSR_key });
  const covers = phone && active && asked;
  const stale = asked && !covers;

  // The entry the view last showed on, while it did.
  const shownOn = useRef<string | undefined | null>(null);
  const closeRef = useRef(close);

  closeRef.current = close;

  useEffect(() => {
    if (covers) {
      shownOn.current = entry;
    }
  }, [covers, entry]);

  useEffect(() => {
    if (!stale) {
      return;
    }

    const endedHere = phone && shownOn.current === entry;

    shownOn.current = null;
    closeRef.current(endedHere ? "user" : "replace");
  }, [stale, phone, entry]);
}

/**
 * Escape anywhere in the covering view or on the call bar over it closes the view, unless
 * something in there (a menu, a popover) took the key first.
 */
export function useCallViewEscape(covers: boolean, close: () => void): void {
  const closeRef = useRef(close);

  closeRef.current = close;

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
        closeRef.current();
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
