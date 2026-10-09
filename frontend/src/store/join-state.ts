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

type RoomReread = {
  /** A recovery read is running. */
  inFlight: boolean;
  /** A read was rejected, or a membership fact landed, while that read was in flight. */
  dirty: boolean;
};

const roomRereads = new Map<number, RoomReread>();

export function resetJoinState(): void {
  joinedAt.clear();
  appliedOutcome.clear();
  nextRequest = 0;
}

/** Test isolation. A rejected read's re-read outlives the store. */
export function resetRoomRereads(): void {
  roomRereads.clear();
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

/** A membership fact arrived during the in-flight re-read, so it must run once more. */
export function dirtyRoomReread(roomId: number): void {
  const current = roomRereads.get(roomId);

  if (current?.inFlight) current.dirty = true;
}

/**
 * A room read lost to a newer membership fact. The caller starts one re-read when this returns
 * true. A loss while that re-read is in flight only marks it dirty.
 */
export function claimRoomReread(roomId: number): boolean {
  const current = roomRereads.get(roomId);

  if (current?.inFlight) {
    current.dirty = true;

    return false;
  }

  roomRereads.set(roomId, { inFlight: true, dirty: false });

  return true;
}

/**
 * The in-flight re-read applied, was rejected, or errored. Returns whether exactly one more
 * should start. A failure with nothing pending returns false, so a network error cannot spin.
 */
export function finishRoomReread(roomId: number): boolean {
  const current = roomRereads.get(roomId);

  if (current === undefined) return false;

  current.inFlight = false;

  if (!current.dirty) {
    roomRereads.delete(roomId);

    return false;
  }

  current.dirty = false;
  current.inFlight = true;

  return true;
}

/** The recovery fiber was interrupted. Drop the hold so the next visit can read. */
export function dropRoomReread(roomId: number): void {
  roomRereads.delete(roomId);
}

export function noteJoined(roomId: number, epoch: number): void {
  joinedAt.set(roomId, epoch);
}

export function joinedAtEpoch(roomId: number): number | undefined {
  return joinedAt.get(roomId);
}
