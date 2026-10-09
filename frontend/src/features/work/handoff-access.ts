/**
 * Who may open the handoff, in the classic page's words (`work_threads#scope`): the thread is
 * tracked, and the viewer may manage it. Anything else is said outright, never a blank route.
 */

/** An untracked thread. */
export const UNTRACKED_HANDOFF = "This thread isn't tracked as work";

/** A viewer who may not manage this work, including a board post they don't manage. */
export const UNMANAGED_HANDOFF = "You cannot manage work in this thread";

/** Why this handoff can't open, or null when it can. */
export function handoffRefusal(input: {
  readonly tracked: boolean;
  readonly canManage: boolean;
}): string | null {
  if (!input.tracked) {
    return UNTRACKED_HANDOFF;
  }

  if (!input.canManage) {
    return UNMANAGED_HANDOFF;
  }

  return null;
}
