import type { Room } from "../gen/Room.ts";
import type { State } from "./state.ts";
import { useStore } from "./store.ts";

/**
 * The newest copy of a room the store holds: its open detail or its sidebar row, whichever the
 * server touched last. A flag change reaches the row first (sync, or a write's reply); the detail
 * catches up on the room's next read.
 */
export function storedRoom(state: State, roomId: number): Room | null {
  const detail = state.rooms[roomId]?.detail?.room ?? null;
  const row = state.sidebar.rows[roomId]?.room ?? null;

  if (detail === null || row === null) return detail ?? row;

  return Date.parse(row.updatedAt) > Date.parse(detail.updatedAt) ? row : detail;
}

/** Whether the store has the room down as an announcement room. */
export function isAnnouncementRoom(state: State, roomId: number): boolean {
  return storedRoom(state, roomId)?.announcement === true;
}

/** `isAnnouncementRoom`, kept current. */
export function useAnnouncementRoom(roomId: number): boolean {
  return useStore((state) => isAnnouncementRoom(state, roomId));
}
