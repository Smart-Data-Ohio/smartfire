import { actions } from "../../sync/runtime.ts";

const fetched = new Set<number>();

/**
 * Once per room per session: learn the viewer's place in the room's active threads, so the reply
 * indicators know which threads are unread for them. Later changes arrive as events.
 */
export function prefetchThreadMemberships(roomId: number): void {
  if (fetched.has(roomId)) {
    return;
  }

  fetched.add(roomId);
  void actions.threads.prefetchMemberships(roomId).catch(() => fetched.delete(roomId));
}
