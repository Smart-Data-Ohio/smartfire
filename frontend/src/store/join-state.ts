/**
 * Join bookkeeping that must outlive navigation. A full reload clears it with the module; leaving,
 * removal, deletion and an unavailable room clear the join epoch here. The store reset does not.
 */

/** Epoch at which a join installed this room's detail. A later 404 keeps that detail only while it holds. */
const joinedAt = new Map<number, number>();

/**
 * Start sequence of the request whose detail is installed. A preview applies unless a request that
 * began later installed detail.
 */
const detailInstalled = new Map<number, number>();

let nextRequest = 0;

export function resetJoinState(): void {
  joinedAt.clear();
  detailInstalled.clear();
  nextRequest = 0;
}

/** A terminal outcome: the id must not keep a join alive for the rest of the page. */
export function clearRoomJoin(roomId: number): void {
  joinedAt.delete(roomId);
}

/** Taken when a room load or metadata refresh begins. */
export function beginRoomRequest(): number {
  return ++nextRequest;
}

/** Detail from a request that began at `started`. An older start must not replace a newer one. */
export function noteDetailInstalled(roomId: number, started: number): void {
  const current = detailInstalled.get(roomId) ?? 0;

  if (started > current) {
    detailInstalled.set(roomId, started);
  }
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
