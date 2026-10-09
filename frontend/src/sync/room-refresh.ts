import { Cause, Effect, Exit, Fiber, Result } from "effect";
import type { SyncEvent } from "../gen/SyncEvent.ts";
import { claimRoomReread, dropRoomReread, finishRoomReread } from "../store/join-state.ts";

type Listener = () => void;

const revisions = new Map<number, number>();

/** Every management change, workspace-wide, counts up one epoch. */
let epoch = 0;

/** The epoch of each room's latest management change. */
const changedAt = new Map<number, number>();

/** The epoch of the latest whole-sidebar snapshot, which rewrote every room the viewer can see. */
let snapshotAt = 0;

const listeners = new Map<number, Set<Listener>>();

type RecoveryStop = {
  readonly id: number;
  readonly interrupt: Effect.Effect<void>;
};

/** The recovery fiber for each room, so leaving or a new visit can interrupt it. */
const recoveryStops = new Map<number, RecoveryStop>();

let nextRecoveryStop = 0;

/**
 * A full room read a re-read loop in flight absorbed (a visit's first read lost to a membership
 * fact while a metadata refresh held the slot). That loop runs it next, in place of its own read,
 * so the visit still settles.
 */
const handoffs = new Map<number, Map<number | null, () => Effect.Effect<void>>>();

/** Test isolation, with `resetRoomRereads`. */
export function resetRoomHandoffs(): void {
  handoffs.clear();
}

function takeHandoff(
  roomId: number,
  visit: number | null,
): (() => Effect.Effect<void>) | undefined {
  const visits = handoffs.get(roomId);
  const read = visits?.get(visit);

  visits?.delete(visit);

  if (visits?.size === 0) handoffs.delete(roomId);

  return read;
}

/** The management revision a pending header/member-list read must still match. */
export function roomRevision(roomId: number): number {
  return revisions.get(roomId) ?? 0;
}

/** Management changes notify mounted readers; message/read/sidebar organisation events do not. */
export function invalidateRoom(roomId: number): number {
  const revision = roomRevision(roomId) + 1;

  revisions.set(roomId, revision);
  epoch += 1;
  changedAt.set(roomId, epoch);

  for (const listener of listeners.get(roomId) ?? []) {
    listener();
  }

  return revision;
}

/**
 * Rooms a snapshot (a reconnect's sidebar, a room's resync) just rewrote: a write that started
 * before it mustn't land over it. Unlike `invalidateRoom` it doesn't ask readers to read again;
 * the snapshot is already the newest.
 */
export function markRoomsChanged(roomIds: Iterable<number>): void {
  epoch += 1;

  for (const roomId of roomIds) {
    changedAt.set(roomId, epoch);
  }
}

/**
 * A reconnect's sidebar snapshot replaced every row, so every write that started before it is
 * older than the store, including a create whose new room the snapshot didn't list.
 */
export function markSidebarSnapshot(): void {
  epoch += 1;
  snapshotAt = epoch;
}

/** The workspace-wide management epoch now: take it before a write, to ask about it afterwards. */
export function managementEpoch(): number {
  return epoch;
}

/**
 * Whether a management change to `roomId` landed after `since` (a `managementEpoch()`): a write's
 * reply that started before it is older than what the store has, so it must not land over it.
 * It works for a room the write itself created, whose id wasn't known when it started.
 */
export function changedSince(roomId: number, since: number): boolean {
  return snapshotAt > since || (changedAt.get(roomId) ?? 0) > since;
}

/** A mounted reader subscribes until its room changes or it unmounts. */
export function onRoomRefresh(roomId: number, listener: Listener): () => void {
  const roomListeners = listeners.get(roomId) ?? new Set<Listener>();

  roomListeners.add(listener);
  listeners.set(roomId, roomListeners);

  return () => {
    roomListeners.delete(listener);

    if (roomListeners.size === 0) {
      listeners.delete(roomId);
    }
  };
}

/** Stops the room's recovery fiber. Leaving, or opening a new visit, calls this. */
export function interruptRoomRecovery(roomId: number): Effect.Effect<void> {
  const stop = recoveryStops.get(roomId);

  if (stop === undefined) return Effect.void;

  return stop.interrupt;
}

/**
 * A room read lost to a newer membership fact. Starts one re-read for `visit`; `read` runs then,
 * so it takes a new sequence. A loss while that re-read is in flight only marks it dirty. When
 * the read finishes, a pending loss starts exactly one more. If `visit` is no longer current,
 * the loop stops without writing and leaves the visit on screen alone — a dirty flag on the old
 * visit is not handed off.
 *
 * `handoff` is a full read of the room (detail, page, and an error or unavailable outcome). When
 * a loop already in flight absorbs this loss, that loop runs `handoff` from its next pass on, so
 * a metadata refresh that holds the slot can't leave the visit loading.
 */
export function recoverRejectedRoomRead<E, R>(
  roomId: number,
  visit: number | null,
  owns: () => boolean,
  read: () => Effect.Effect<void, E, R>,
  handoff?: () => Effect.Effect<void>,
): Effect.Effect<void, E, R> {
  if (!owns()) return Effect.void;

  if (!claimRoomReread(roomId, visit)) {
    if (handoff !== undefined) {
      const visits = handoffs.get(roomId) ?? new Map<number | null, () => Effect.Effect<void>>();

      visits.set(visit, handoff);
      handoffs.set(roomId, visits);
    }

    return Effect.void;
  }

  takeHandoff(roomId, visit);

  return runRoomReread(roomId, visit, owns, read);
}

function runRoomReread<E, R>(
  roomId: number,
  visit: number | null,
  owns: () => boolean,
  read: () => Effect.Effect<void, E, R>,
): Effect.Effect<void, E, R> {
  const drop = () => {
    dropRoomReread(roomId, visit);
    takeHandoff(roomId, visit);
  };

  const loop = Effect.gen(function* () {
    let next = read;

    while (true) {
      if (!owns()) {
        drop();

        return;
      }

      const result = yield* Effect.result(next());

      if (!owns()) {
        drop();

        return;
      }

      if (finishRoomReread(roomId, visit)) {
        next = takeHandoff(roomId, visit) ?? next;
        continue;
      }

      takeHandoff(roomId, visit);

      if (Result.isFailure(result)) return yield* Effect.fail(result.failure);

      return;
    }
  }).pipe(Effect.onInterrupt(() => Effect.sync(drop)));

  return Effect.gen(function* () {
    const fiber = yield* Effect.forkChild(loop);
    const id = ++nextRecoveryStop;

    recoveryStops.set(roomId, { id, interrupt: Fiber.interrupt(fiber) });

    const exit = yield* Fiber.await(fiber);

    if (recoveryStops.get(roomId)?.id === id) recoveryStops.delete(roomId);

    if (Exit.isFailure(exit) && !Cause.hasInterruptsOnly(exit.cause)) {
      return yield* Effect.failCause(exit.cause);
    }
  });
}

/** Once per room in a batch, including hidden removals whose membership is still readable. */
export function roomRefreshIds(events: readonly SyncEvent[]): readonly number[] {
  const ids = new Set<number>();

  for (const event of events) {
    if (event.type === "sidebar.row.upserted" && event.data.refreshRoom === true) {
      ids.add(event.data.room.id);
    } else if (event.type === "sidebar.row.removed" && event.data.refreshRoom === true) {
      ids.add(event.data.roomId);
    }
  }

  return [...ids];
}
