/**
 * ↑ in an empty composer edits your last message, as in Slack. The composer works out which one
 * from the store and hands it to the editing store; the row opens its editor.
 * (The messages slice may export its own `editLastOwnMessage`; the lead reconciles the two.)
 */
import type { State } from "../../store/state.ts";
import { store } from "../../store/store.ts";
import { startEditing } from "../messages/editing-store.ts";

/**
 * The newest confirmed message the viewer can edit in this conversation (the room's root
 * timeline, or one thread's replies), or `null`. Only when the loaded window reaches the present:
 * otherwise "last" isn't on screen to edit.
 */
export function lastOwnMessageId(
  state: State,
  roomId: number,
  threadId: number | null,
): number | null {
  const viewerId = state.me?.user.id;
  const timeline = threadId === null ? state.timelines[roomId] : state.threadTimelines[threadId];

  if (viewerId === undefined || timeline === undefined || timeline.after !== null) {
    return null;
  }

  for (let index = timeline.ids.length - 1; index >= 0; index -= 1) {
    const message = state.messages[timeline.ids[index] ?? -1];

    if (message !== undefined && message.creatorId === viewerId && !message.systemNote) {
      return message.id;
    }
  }

  return null;
}

/** Opens the editor on the viewer's last message here; `false` when there's none to edit. */
export function editLastOwnMessage(roomId: number, threadId: number | null): boolean {
  const id = lastOwnMessageId(store.getState(), roomId, threadId);

  if (id === null) {
    return false;
  }

  startEditing(id);

  return true;
}
