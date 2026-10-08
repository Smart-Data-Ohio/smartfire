import { useEffect, useEffectEvent } from "react";
import { actions } from "../../sync/runtime.ts";

/**
 * Calls `onChange` while mounted whenever one of `roomId`'s events changes elsewhere (another
 * member schedules, edits, cancels or removes one), so an open calendar screen reads itself again
 * rather than going stale.
 */
export function useEventChanges(roomId: number, onChange: () => void) {
  const changed = useEffectEvent(onChange);

  useEffect(() => actions.events.watch(roomId, () => changed()), [roomId]);
}
