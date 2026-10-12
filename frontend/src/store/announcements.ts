import type { Room } from "../gen/Room.ts";
import type { State } from "./state.ts";
import { useStore } from "./store.ts";

/**
 * The room as the store holds it for its flag: the sidebar row, which sync keeps current and a
 * write's reply can't overwrite once sync has touched it; the open detail only for a room with no
 * row (one the viewer isn't in). Timestamps can't pick between the two: they tie to the
 * millisecond, and a tie says nothing about which is current.
 */
export function storedRoom(state: State, roomId: number): Room | null {
  return state.sidebar.rows[roomId]?.room ?? state.rooms[roomId]?.detail?.room ?? null;
}

/** Whether the store has the room down as an announcement room. */
export function isAnnouncementRoom(state: State, roomId: number): boolean {
  return storedRoom(state, roomId)?.announcement === true;
}

/** `isAnnouncementRoom`, kept current. */
export function useAnnouncementRoom(roomId: number): boolean {
  return useStore((state) => isAnnouncementRoom(state, roomId));
}
