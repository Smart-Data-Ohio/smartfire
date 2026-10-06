/** Reducers for what S2 hangs off a message: reactions and boosts, pins and saved marks. */
import type { MessagePage } from "../gen/MessagePage.ts";
import type { MessageReactions } from "../gen/MessageReactions.ts";
import type { PinState } from "../gen/PinState.ts";
import type { Reaction } from "../gen/Reaction.ts";
import type { State } from "./state.ts";

/** A page is authoritative for its own messages' saved marks: those it lists, and no others. */
export function mergeSavedMarks(saved: State["saved"], page: MessagePage): State["saved"] {
  if (page.messages.length === 0) {
    return saved;
  }

  const onPage = new Set(page.messages.map((message) => message.id));
  const kept = Object.entries(saved).filter(([id]) => !onPage.has(Number(id)));

  return Object.fromEntries([
    ...kept,
    ...page.saved.map((mark) => [mark.messageId, mark.savedItemId] as const),
  ]);
}

/** The viewer saved (`savedItemId`) or unsaved (`null`) a message, here or in another tab. */
export function setSavedMark(state: State, messageId: number, savedItemId: number | null): State {
  if ((state.saved[messageId] ?? null) === savedItemId) {
    return state;
  }

  const { [messageId]: _previous, ...others } = state.saved;

  return {
    ...state,
    saved: savedItemId === null ? others : { ...others, [messageId]: savedItemId },
  };
}

/** New reactions and boosts land unless the held copy is already newer. */
export function setReactions(state: State, change: MessageReactions): State {
  const held = state.messages[change.messageId];

  if (held === undefined || held.updatedAt > change.updatedAt) {
    return state;
  }

  return {
    ...state,
    messages: {
      ...state.messages,
      [held.id]: {
        ...held,
        reactions: change.reactions,
        boosts: change.boosts,
        updatedAt: change.updatedAt,
      },
    },
  };
}

/** A pin or unpin: the message's flag and the room header's count. */
export function setPinState(state: State, change: PinState): State {
  const held = state.messages[change.messageId];
  const room = state.rooms[change.roomId];
  let next = state;

  if (held !== undefined && held.pinned !== change.pinned) {
    next = {
      ...next,
      messages: { ...next.messages, [held.id]: { ...held, pinned: change.pinned } },
    };
  }

  if (room?.detail != null && room.detail.pinsCount !== change.pinCount) {
    next = {
      ...next,
      rooms: {
        ...next.rooms,
        [change.roomId]: { ...room, detail: { ...room.detail, pinsCount: change.pinCount } },
      },
    };
  }

  return next;
}

/**
 * The reactions after the viewer toggles `content`, as the server will compute them: their id
 * leaves the reaction if it was there (the reaction goes when nobody's left), else joins it (a
 * new reaction goes last). For the optimistic update; the server's reply replaces it.
 */
/**
 * `reactions` with the viewer in or out of `content`'s reactors, as `present` says: undoes one
 * toggle on top of whatever arrived since, without touching anyone else's change.
 */
export function withViewerReaction(
  reactions: readonly Reaction[],
  content: string,
  viewerId: number,
  present: boolean,
  shown: { readonly title: string; readonly imageUrl: string | null },
): Reaction[] {
  const existing = reactions.find((reaction) => reaction.content === content);
  const mine = existing?.reactorIds.includes(viewerId) ?? false;

  return mine === present ? [...reactions] : toggledReactions(reactions, content, viewerId, shown);
}

export function toggledReactions(
  reactions: readonly Reaction[],
  content: string,
  viewerId: number,
  shown: { readonly title: string; readonly imageUrl: string | null },
): Reaction[] {
  const existing = reactions.find((reaction) => reaction.content === content);

  if (existing === undefined) {
    return [
      ...reactions,
      { content, title: shown.title, imageUrl: shown.imageUrl, reactorIds: [viewerId] },
    ];
  }

  const mine = existing.reactorIds.includes(viewerId);

  const reactorIds = mine
    ? existing.reactorIds.filter((id) => id !== viewerId)
    : [...existing.reactorIds, viewerId];

  return reactions.flatMap((reaction) => {
    if (reaction !== existing) {
      return [reaction];
    }

    return reactorIds.length === 0 ? [] : [{ ...reaction, reactorIds }];
  });
}
