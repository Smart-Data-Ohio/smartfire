import type { MessageDTO } from "../../store/model.ts";
import { useStore } from "../../store/store.ts";
import { type MessagePermissions, messagePermissions } from "./permissions.ts";

/** The signed-in person's id, or `null` before boot. */
export function useViewerId(): number | null {
  return useStore((state) => state.me?.user.id ?? state.boot?.user.id ?? null);
}

/** What the viewer may do with `message`, from the store's viewer, room and thread. */
export function useMessagePermissions(message: MessageDTO): MessagePermissions {
  const viewerId = useViewerId();
  const viewerRole = useStore((state) => state.me?.user.role ?? null);
  const roomKind = useStore((state) => state.rooms[message.roomId]?.detail?.room.kind ?? null);

  const threadStatus = useStore((state) =>
    message.threadId === null ? null : (state.threads[message.threadId]?.status ?? null),
  );

  return messagePermissions(message, { viewerId, viewerRole, roomKind, threadStatus });
}

/** The viewer's saved item for `message`, or `null` when it isn't saved. */
export function useSavedItemId(messageId: number): number | null {
  return useStore((state) => state.saved[messageId] ?? null);
}
