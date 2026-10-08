/**
 * Join bookkeeping that must outlive navigation. A full reload clears it with the module; leaving,
 * removal, deletion and an unavailable room clear the join epoch here. The store reset does not.
 */

/** Epoch at which a join installed this room's detail. A later 404 keeps that detail only while it holds. */
const joinedAt = new Map<number, number>();

/**
 * Sequence of the latest outcome applied for a room. Detail, a join preview, unavailable,
 * and a confirmed membership change all share it. An older sequence does not apply.
 */
const appliedOutcome = new Map<number, number>();

let nextRequest = 0;

export function resetJoinState(): void {
  joinedAt.clear();
  appliedOutcome.clear();
  nextRequest = 0;
}

/** A terminal outcome: the id must not keep a join alive for the rest of the page. */
export function clearRoomJoin(roomId: number): void {
  joinedAt.delete(roomId);
}

/**
 * Next outcome sequence. A room read takes one when the request starts. A server-confirmed
 * fact — a mutation's success, or a socket membership change — takes one when it arrives,
 * so any read that started earlier loses.
 */
export function beginRoomRequest(): number {
  return ++nextRequest;
}

/** Records `started` when it is newer than the outcome already applied for `roomId`. */
export function claimRoomOutcome(roomId: number, started: number): boolean {
  const current = appliedOutcome.get(roomId) ?? 0;

  if (started <= current) {
    return false;
  }

  appliedOutcome.set(roomId, started);

  return true;
}

export function noteJoined(roomId: number, epoch: number): void {
  joinedAt.set(roomId, epoch);
}

export function joinedAtEpoch(roomId: number): number | undefined {
  return joinedAt.get(roomId);
}
