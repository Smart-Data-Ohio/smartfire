import { useLocation, useRouter } from "@tanstack/react-router";

declare module "@tanstack/react-router" {
  interface HistoryState {
    /** Set by an in-app link that opened an event dialog over the page it came from. */
    readonly smartfireEventOver?: boolean;
  }
}

/** The history state an in-app link to an event dialog's URL carries. */
export function overPageState() {
  return { smartfireEventOver: true };
}

/**
 * Closing an event dialog that has its own URL: step back when an in-app link opened it over the
 * page beneath (so browser Back doesn't reopen it), else replace its entry with `fallback`'s (the
 * dialog was the page someone opened).
 */
export function useCloseOverlay(fallback: () => void) {
  const router = useRouter();
  const over = useLocation({ select: (location) => location.state.smartfireEventOver === true });

  return () => {
    if (over) {
      router.history.back();
    } else {
      fallback();
    }
  };
}
