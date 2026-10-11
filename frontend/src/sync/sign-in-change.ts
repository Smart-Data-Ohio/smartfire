/**
 * Leaving the page when someone else has signed in (another tab switched accounts, and this one
 * reconnected as them without a reload). Everything held here was the last person's; the sync
 * engine stops first, then this clears what the tab kept for them and loads the page again.
 */

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
