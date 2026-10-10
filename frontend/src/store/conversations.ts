/**
 * What cross-room lists (saved items, scheduled messages) call the room and thread a row is in.
 * The lists carry a `ConversationName` per `(roomId, threadId)` they name; they're kept here,
 * shared, and `conversationNameOf` falls back to the sidebar and thread records for a row that
 * arrived live without one.
 */
import type { ConversationName } from "../gen/ConversationName.ts";
import type { State } from "./state.ts";

/** The key of a room's root timeline (`threadId` `null`) or one of its threads. */
export function conversationKey(roomId: number, threadId: number | null): string {
  return `${roomId}:${threadId ?? ""}`;
}

/** Names from a list join the store; the list's copy is the latest. */
export function mergeConversationNames(state: State, names: readonly ConversationName[]): State {
  if (names.length === 0) {
    return state;
  }

  return {
    ...state,
    conversationNames: {
      ...state.conversationNames,
      ...Object.fromEntries(
        names.map((name) => [conversationKey(name.roomId, name.threadId), name] as const),
      ),
    },
  };
}

export function updateConversationRoom(
  state: State,
  room: Pick<ConversationName, "roomId" | "roomName" | "roomKind" | "roomIconName">,
): State {
  return mergeConversationNames(
    state,
    Object.values(state.conversationNames)
      .filter((name) => name.roomId === room.roomId)
      .map((name) => ({ ...name, ...room })),
  );
}

/**
 * What to call a conversation: the name a list brought, else one made from the sidebar row and
 * the thread record; `null` when the store knows neither (the row can say "a conversation").
 */
export function conversationNameOf(
  state: Pick<State, "conversationNames" | "sidebar" | "threads">,
  roomId: number,
  threadId: number | null,
): ConversationName | null {
  const known = state.conversationNames[conversationKey(roomId, threadId)];

  if (known !== undefined) {
    return known;
  }

  const row = state.sidebar.rows[roomId];
  const room = state.conversationNames[conversationKey(roomId, null)];

  if (row === undefined && room === undefined) {
    return null;
  }

  return {
    roomId,
    threadId,
    roomKind: row?.room.kind ?? room?.roomKind ?? "open",
    roomName: row?.displayName ?? room?.roomName ?? "",
    roomIconName: row?.room.iconName ?? room?.roomIconName ?? null,
    threadName: threadId === null ? null : (state.threads[threadId]?.name ?? null),
  };
}
