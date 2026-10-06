import type { MessageDTO } from "../../store/model.ts";
import type { State } from "../../store/state.ts";
import { store } from "../../store/store.ts";
import { startEditing } from "./editing-store.ts";
import { messagePermissions } from "./permissions.ts";

/**
 * The newest message the viewer may edit in a conversation's loaded window: the room's root
 * timeline, or the thread's replies when `threadId` is given. Only when that window reaches the
 * present: otherwise the "last" message isn't on screen to edit.
 */
export function lastEditableMessage(
  state: State,
  roomId: number,
  threadId: number | null,
): MessageDTO | null {
  const timeline = threadId === null ? state.timelines[roomId] : state.threadTimelines[threadId];

  if (timeline === undefined || timeline.after !== null) {
    return null;
  }

  const ids = timeline.ids;

  const viewerId = state.me?.user.id ?? state.boot?.user.id ?? null;

  const context = {
    viewerId,
    viewerRole: state.me?.user.role ?? null,
    roomKind: state.rooms[roomId]?.detail?.room.kind ?? null,
    threadStatus: threadId === null ? null : (state.threads[threadId]?.status ?? null),
  };

  for (let index = ids.length - 1; index >= 0; index -= 1) {
    const message = state.messages[ids[index] ?? -1];

    if (message !== undefined && messagePermissions(message, context).edit) {
      return message;
    }
  }

  return null;
}

/**
 * The composer's ↑ with an empty box: opens the viewer's last message in the conversation for
 * editing (the timeline scrolls it into view) and hands focus back to the composer when the edit
 * ends. Answers whether there was one to edit.
 */
export function editLastOwnMessage(roomId: number, threadId: number | null = null): boolean {
  const message = lastEditableMessage(store.getState(), roomId, threadId);

  if (message === null) {
    return false;
  }

  const active = document.activeElement;

  startEditing(message.id, active instanceof HTMLElement ? active : null);

  return true;
}
