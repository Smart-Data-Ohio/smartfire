import { useStore } from "../../store/store.ts";

/** A DM's members and name as the header knows them (detail first, the sidebar row meanwhile). */
export interface DirectFacts {
  readonly memberIds: readonly number[];
  readonly name: string | null;
  readonly displayName: string;
}

export function useDirectFacts(roomId: number): DirectFacts | null {
  const detail = useStore((state) => state.rooms[roomId]?.detail ?? null);
  const row = useStore((state) => state.sidebar.rows[roomId] ?? null);
  const source = detail ?? row;

  if (source === null || source.room.kind !== "direct") {
    return null;
  }

  return {
    memberIds: source.directMemberIds,
    name: source.room.name,
    displayName: source.displayName,
  };
}
