import { useEffect, useRef } from "react";
import type { BoardTagCatalog } from "../../gen/BoardTagCatalog.ts";
import { useStore } from "../../store/store.ts";
import { actions } from "../../sync/runtime.ts";

/** One catalog read per board at a time; a change that lands during it asks for one more. */
const reading = new Map<number, { again: boolean }>();

/** Reads the board's catalog into the store, folding overlapping asks into one request. */
export function refreshBoardTags(roomId: number): void {
  const held = reading.get(roomId);

  if (held !== undefined) {
    held.again = true;

    return;
  }

  const read = { again: false };

  reading.set(roomId, read);
  actions.boards
    .tagCatalog(roomId)
    // The catalog shown stays; the next change or opening tries again.
    .catch(() => undefined)
    .finally(() => {
      reading.delete(roomId);

      if (read.again) refreshBoardTags(roomId);
    });
}

/**
 * The board's curated tags and policy from the store, kept current: `board.automations.changed`
 * (someone saved the catalog or policy, or the room resynced) reads it again. `fetchOnMount`
 * reads it on mount too, for views that don't arrive with it (the tag editor).
 */
export function useBoardTagCatalog(
  roomId: number,
  fetchOnMount = false,
): BoardTagCatalog | undefined {
  const catalog = useStore((state) => state.boardTagCatalogs[roomId]);
  const signal = useStore((state) => state.boardAutomationsChanged[roomId] ?? 0);
  const seen = useRef(signal);

  useEffect(() => {
    if (fetchOnMount) refreshBoardTags(roomId);
  }, [roomId, fetchOnMount]);

  useEffect(() => {
    if (signal !== seen.current) {
      seen.current = signal;
      refreshBoardTags(roomId);
    }
  }, [signal, roomId]);

  return catalog;
}
