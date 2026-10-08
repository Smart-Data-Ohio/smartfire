/**
 * Fresh loads of a thread's replies (opening or retrying the pane, and a sync resync) aren't
 * serialized with each other, so each takes a ticket and only the latest one lands: an older
 * snapshot answering late mustn't replace the replies a newer load installed.
 */
const latest = new Map<number, number>();

let issued = 0;

/** Starts a fresh load of `threadId`'s replies; any load started before it is superseded. */
export function beginThreadLoad(threadId: number): number {
  issued += 1;
  latest.set(threadId, issued);

  return issued;
}

/** No fresh load of `threadId` started after the one holding `ticket`. */
export function isLatestThreadLoad(threadId: number, ticket: number): boolean {
  return latest.get(threadId) === ticket;
}
