/**
 * Join bookkeeping that must outlive navigation. A full reload clears it with the module; leaving,
 * removal, deletion and an unavailable room clear it here. The store's own reset does not.
 */

/** The server accepted a join whose on-screen visit still has to subscribe and load. */
const unfinishedJoins = new Set<number>();

/** Epoch at which a join installed this room's detail. A later 404 keeps that detail only while it holds. */
const joinedAt = new Map<number, number>();

export function resetJoinState(): void {
  unfinishedJoins.clear();
  joinedAt.clear();
}

/** A terminal outcome: the id must not keep a join alive for the rest of the page. */
export function clearRoomJoin(roomId: number): void {
  unfinishedJoins.delete(roomId);
  joinedAt.delete(roomId);
}

export function hasUnfinishedJoin(roomId: number): boolean {
  return unfinishedJoins.has(roomId);
}

export function markUnfinishedJoin(roomId: number): void {
  unfinishedJoins.add(roomId);
}

export function finishUnfinishedJoin(roomId: number): void {
  unfinishedJoins.delete(roomId);
}

export function noteJoined(roomId: number, epoch: number): void {
  joinedAt.set(roomId, epoch);
}

export function joinedAtEpoch(roomId: number): number | undefined {
  return joinedAt.get(roomId);
}
