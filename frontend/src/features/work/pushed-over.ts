/**
 * An in-app open of a thread's handoff dialog or links editor pushes a history entry over the
 * thread. Closing steps back to that entry. A direct arrival (no flag) replaces this entry with
 * the thread, so Back after closing leaves the thread instead of landing on a second copy of it.
 */
import { useLocation, useRouter } from "@tanstack/react-router";

declare module "@tanstack/react-router" {
  interface HistoryState {
    /** The work bar or a board post pushed this entry over the thread. */
    readonly smartfirePushedOver?: boolean;
  }
}

/** The history state an in-app open carries, so closing can step back to the entry under it. */
export function pushedOverState() {
  return { smartfirePushedOver: true };
}

/** Step back when this entry was pushed in-app; otherwise run `replace`. */
export function useClosePushedOver() {
  const router = useRouter();
  const over = useLocation({ select: (location) => location.state.smartfirePushedOver === true });

  return (replace: () => void) => {
    if (over) {
      router.history.back();

      return;
    }

    replace();
  };
}
