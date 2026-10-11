/**
 * Leaving the page when someone else has signed in (another tab switched accounts, and this one
 * reconnected as them without a reload). Everything held here was the last person's; the sync
 * engine stops first, then this clears what the tab kept for them and loads the page again.
 */

import { mutations, store } from "../store/store.ts";

/** What this tab keeps in sessionStorage: drafts, the sync resume point. */
const TAB_STORAGE_PREFIX = "smartfire.";

/** The reload itself, as an object so tests can stub it. */
export const pageReload = {
  reload(): void {
    window.location.reload();
  },
};

/** Drops the tab's stored drafts and resume point, then reloads as whoever is signed in now. */
export function reloadForNewSignIn(): void {
  try {
    const keys = Array.from({ length: sessionStorage.length }, (_, index) =>
      sessionStorage.key(index),
    );

    for (const key of keys) {
      if (key?.startsWith(TAB_STORAGE_PREFIX) === true) sessionStorage.removeItem(key);
    }
  } catch {
    // Without storage nothing was kept.
  }

  pageReload.reload();
}

/** How the running sync engine stops for a new sign-in (it reloads once stopped); null while none runs. */
let stopRun: (() => void) | null = null;

/** Has a new sign-in stop the engine's run, which then reloads; returns the undo for its end. */
export function stopRunForNewSignIn(stop: () => void): () => void {
  stopRun = stop;

  return () => {
    if (stopRun === stop) stopRun = null;
  };
}

/**
 * Leaves for a new sign-in. The page first stops for good (`accountChanged`: nothing more is
 * requested or stored, and a notice offers the reload), so a reload the browser doesn't carry out
 * leaves nothing working as the old account. Then it stops the sync engine when it runs (which
 * reloads once stopped), or reloads.
 */
export function leaveForNewSignIn(): void {
  try {
    mutations.markAccountChanged();
  } finally {
    // The engine stop and the reload run whatever the freeze ran into.
    if (stopRun === null) {
      reloadForNewSignIn();
    } else {
      stopRun();
    }
  }
}

/**
 * The check for every reply that says whose it is (`/blocks` and the ignore writes): whether the
 * server answered for who this page is signed in as. When it answered for someone else, the page
 * leaves for them, and nothing in the reply may be used.
 */
export function answeredForThisPage(viewerId: number): boolean {
  const me = store.getState().me;

  if (me === null || me.user.id === viewerId) {
    return true;
  }

  leaveForNewSignIn();

  return false;
}
