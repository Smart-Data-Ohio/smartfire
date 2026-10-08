import { useLocation, useRouter } from "@tanstack/react-router";

declare module "@tanstack/react-router" {
  interface HistoryState {
    /** Set by the message menu when it opened "Create Fizzy card" over the conversation. */
    readonly smartfireFizzyOver?: boolean;
  }
}

/** The history state the message menu's navigation to the dialog's URL carries. */
export function overConversationState() {
  return { smartfireFizzyOver: true };
}

/**
 * Closing the dialog: step back when the message menu opened it over the conversation (so browser
 * Back doesn't reopen it), else replace its entry with `fallback`'s (someone opened its URL).
 */
export function useCloseOverlay(fallback: () => void) {
  const router = useRouter();
  const over = useLocation({ select: (location) => location.state.smartfireFizzyOver === true });

  return () => {
    if (over) {
      router.history.back();
    } else {
      fallback();
    }
  };
}
