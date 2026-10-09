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

/**
 * Per room, then per visit (`null` when the recovery started with no visit open). An old visit's
 * in-flight read must not block the visit now on screen.
 */
const roomRereads = new Map<number, Map<number | null, RoomReread>>();

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

function forgetRoomReread(
  roomId: number,
  visits: Map<number | null, RoomReread>,
  visit: number | null,
): void {
  visits.delete(visit);

  if (visits.size === 0) roomRereads.delete(roomId);
}

/** A membership fact arrived during the in-flight re-read, so that visit must run once more. */
export function dirtyRoomReread(roomId: number): void {
  const visits = roomRereads.get(roomId);

  if (visits === undefined) return;

  for (const current of visits.values()) {
    if (current.inFlight) current.dirty = true;
  }
}

/**
 * A room read lost to a newer membership fact. The caller starts one re-read for `visit` when
 * this returns true. A loss while that re-read is in flight only marks it dirty.
 */
export function claimRoomReread(roomId: number, visit: number | null): boolean {
  const visits = roomRereads.get(roomId) ?? new Map<number | null, RoomReread>();
  const current = visits.get(visit);

  if (current?.inFlight) {
    current.dirty = true;

    return false;
  }

  visits.set(visit, { inFlight: true, dirty: false });
  roomRereads.set(roomId, visits);

  return true;
}

/**
 * The in-flight re-read applied, was rejected, or errored. Returns whether exactly one more
 * should start. A failure with nothing pending returns false, so a network error cannot spin.
 */
export function finishRoomReread(roomId: number, visit: number | null): boolean {
  const visits = roomRereads.get(roomId);
  const current = visits?.get(visit);

  if (current === undefined || visits === undefined) return false;

  current.inFlight = false;

  if (!current.dirty) {
    forgetRoomReread(roomId, visits, visit);

    return false;
  }

  current.dirty = false;
  current.inFlight = true;

  return true;
}

/** This visit's recovery stopped or was interrupted. Drop its hold and leave every other visit. */
export function dropRoomReread(roomId: number, visit: number | null): void {
  const visits = roomRereads.get(roomId);

  if (visits === undefined) return;

  forgetRoomReread(roomId, visits, visit);
}

export function noteJoined(roomId: number, epoch: number): void {
  joinedAt.set(roomId, epoch);
}

export function joinedAtEpoch(roomId: number): number | undefined {
  return joinedAt.get(roomId);
}
