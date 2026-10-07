import type { SyncEvent } from "../gen/SyncEvent.ts";

type Listener = () => void;

const revisions = new Map<number, number>();

const listeners = new Map<number, Set<Listener>>();

/** The management revision a pending header/member-list read must still match. */
export function roomRevision(roomId: number): number {
  return revisions.get(roomId) ?? 0;
}

/** Management changes notify mounted readers; message/read/sidebar organisation events do not. */
export function invalidateRoom(roomId: number): number {
  const revision = roomRevision(roomId) + 1;

  revisions.set(roomId, revision);

  for (const listener of listeners.get(roomId) ?? []) {
    listener();
  }

  return revision;
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
