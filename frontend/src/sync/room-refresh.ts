import type { SyncEvent } from "../gen/SyncEvent.ts";

type Listener = () => void;

const revisions = new Map<number, number>();

/** Every management change, workspace-wide, counts up one epoch. */
let epoch = 0;

/** The epoch of each room's latest management change. */
const changedAt = new Map<number, number>();

const listeners = new Map<number, Set<Listener>>();

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
  return (changedAt.get(roomId) ?? 0) > since;
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
