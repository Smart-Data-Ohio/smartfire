/**
 * Who may open the handoff, in the classic page's words (`work_threads#scope`): the thread is
 * tracked, the viewer may manage it, and some agent in the room can take it. Anything else is
 * said outright, never a blank route.
 */

/** An untracked thread. */
export const UNTRACKED_HANDOFF = "This thread isn't tracked as work";

/** A viewer who may not manage this work, including a board post they don't manage. */
export const UNMANAGED_HANDOFF = "You cannot manage work in this thread";

/**
 * A manager of tracked work, and no agent here can receive it. The classic handoff said this
 * under the receiver field.
 */
export const NO_RECEIVER_HANDOFF = [
  "No agent here can take this work.",
  "An agent needs to be in this room and allowed to post, manage threads and read messages.",
].join(" ");

/**
 * Why this handoff can't open, or null when it can. `receiverCount` stays null until the work
 * detail is known, so a thread still loading is not treated as having nobody to receive it.
 */
export function handoffRefusal(input: {
  readonly tracked: boolean;
  readonly canManage: boolean;
  readonly receiverCount: number | null;
}): string | null {
  if (!input.tracked) {
    return UNTRACKED_HANDOFF;
  }

  if (!input.canManage) {
    return UNMANAGED_HANDOFF;
  }

  if (input.receiverCount === 0) {
    return NO_RECEIVER_HANDOFF;
  }

  return null;
}
