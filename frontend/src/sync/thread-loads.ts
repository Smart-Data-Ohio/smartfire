/**
 * Fresh loads of a thread's replies (opening or retrying the pane, and a sync resync) aren't
 * serialized with each other, so each takes a ticket and only the latest one lands: an older
 * snapshot answering late mustn't replace the replies a newer load installed. The latest load
 * then owns everything the superseded ones would have written: the pane's error, and the reply a
 * permalink asked to open at.
 */
interface Load {
  readonly ticket: number;
  /** The reply this load opens the replies around (a permalink's), until that page is in. */
  readonly focus: number | null;
}

const latest = new Map<number, Load>();

let issued = 0;

/** Starts a fresh load of `threadId`'s replies, around `focus` if set; any earlier one is superseded. */
export function beginThreadLoad(threadId: number, focus: number | null): number {
  issued += 1;
  latest.set(threadId, { ticket: issued, focus });

  return issued;
}

/** No fresh load of `threadId` started after the one holding `ticket`. */
export function isLatestThreadLoad(threadId: number, ticket: number): boolean {
  return latest.get(threadId)?.ticket === ticket;
}

/** The reply the latest load of `threadId` still has to open at, which a load superseding it keeps. */
export function pendingThreadFocus(threadId: number): number | null {
  return latest.get(threadId)?.focus ?? null;
}

/** The load holding `ticket` installed its page: its reply is in view, so no later load keeps the focus. */
export function finishThreadLoad(threadId: number, ticket: number): void {
  const load = latest.get(threadId);

  if (load?.ticket === ticket && load.focus !== null) {
    latest.set(threadId, { ticket, focus: null });
  }
}
