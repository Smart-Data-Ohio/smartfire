/**
 * Join bookkeeping that must outlive navigation. A full reload clears it with the module; leaving,
 * removal, deletion and an unavailable room clear the join epoch here. The store reset does not.
 */

/** Epoch at which a join installed this room's detail. A later 404 keeps that detail only while it holds. */
const joinedAt = new Map<number, number>();

/**
 * Times detail has been installed for a room. A preview load captures this and applies only while
 * it is unchanged, so a preview from before the install cannot replace that membership.
 */
const detailInstalled = new Map<number, number>();

let detailInstalls = 0;

export function resetJoinState(): void {
  joinedAt.clear();
  detailInstalled.clear();
  detailInstalls = 0;
}

/** A terminal outcome: the id must not keep a join alive for the rest of the page. */
export function clearRoomJoin(roomId: number): void {
  joinedAt.delete(roomId);
}

/** Detail landed for `roomId`. Previews that started earlier must not paint over it. */
export function noteDetailInstalled(roomId: number): void {
  detailInstalled.set(roomId, ++detailInstalls);
}

export function detailInstalledGeneration(roomId: number): number {
  return detailInstalled.get(roomId) ?? 0;
}

export function noteJoined(roomId: number, epoch: number): void {
  joinedAt.set(roomId, epoch);
}

export function joinedAtEpoch(roomId: number): number | undefined {
  return joinedAt.get(roomId);
}
