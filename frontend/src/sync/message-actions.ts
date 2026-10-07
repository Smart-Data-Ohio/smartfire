/**
 * The S2 message actions as Effect programs: each calls its endpoint and lands the reply in the
 * store (the sync event that follows is then a no-op). Reactions, pins and unsaves show at once
 * and roll back if the server refuses.
 */
import { Clock, Effect } from "effect";
import * as api from "../api/message-endpoints.ts";
import { createUpload } from "../api/message-endpoints.ts";
import type { CreateUpload } from "../gen/CreateUpload.ts";
import type { ForwardTarget } from "../gen/ForwardTarget.ts";
import type { Reaction } from "../gen/Reaction.ts";
import { toggledReactions, withViewerReaction } from "../store/message-extras.ts";
import { mutations, store } from "../store/store.ts";

const viewerId = () => {
  const state = store.getState();

  return state.me?.user.id ?? state.boot?.user.id ?? 0;
};

/** `PATCH`es the Markdown and keeps the updated message. */
export const edit = Effect.fn("messages.edit")(function* (messageId: number, markdown: string) {
  const message = yield* api.updateMessage(messageId, markdown);

  mutations.updateMessage(message);

  return message;
});

/** The Markdown the edit box starts from. */
export const source = Effect.fn("messages.source")(function* (messageId: number) {
  return (yield* api.messageSource(messageId)).markdownSource;
});

/** Deletes it; it leaves every timeline (and can't come back from a late event). */
export const remove = Effect.fn("messages.remove")(function* (messageId: number) {
  yield* api.deleteMessage(messageId);

  const held = store.getState().messages[messageId];

  if (held !== undefined) {
    mutations.removeMessage(held, yield* Clock.currentTimeMillis);
  }
});

/**
 * Toggles the viewer's reaction, showing it at once. A refusal undoes only this toggle, on top of
 * whatever reactions and boosts arrived meanwhile.
 */
export const toggleReaction = Effect.fn("messages.toggleReaction")(function* (
  messageId: number,
  content: string,
  shown: { readonly title: string; readonly imageUrl: string | null },
) {
  const viewer = viewerId();

  /** The message as held now, with its reactions changed by `change`. */
  const landReactions = (change: (reactions: readonly Reaction[]) => Reaction[]) => {
    const held = store.getState().messages[messageId];

    if (held !== undefined) {
      mutations.setReactions({
        messageId,
        roomId: held.roomId,
        threadId: held.threadId,
        reactions: change(held.reactions),
        boosts: held.boosts,
        updatedAt: held.updatedAt,
      });
    }
  };

  const before = store.getState().messages[messageId];

  const wasMine =
    before?.reactions.some(
      (reaction) => reaction.content === content && reaction.reactorIds.includes(viewer),
    ) ?? false;

  landReactions((reactions) => toggledReactions(reactions, content, viewer, shown));

  const reply = yield* api
    .react(messageId, content)
    .pipe(
      Effect.tapError(() =>
        Effect.sync(() =>
          landReactions((reactions) =>
            withViewerReaction(reactions, content, viewer, wasMine, shown),
          ),
        ),
      ),
    );

  mutations.setReactions(reply);
});

/** Adds a text boost (up to 16 characters). */
export const boost = Effect.fn("messages.boost")(function* (messageId: number, text: string) {
  mutations.setReactions(yield* api.react(messageId, text));
});

/** Takes the viewer's boost back. */
export const removeBoost = Effect.fn("messages.removeBoost")(function* (
  messageId: number,
  boostId: number,
) {
  mutations.setReactions(yield* api.deleteBoost(messageId, boostId));
});

/** Pins or unpins, flipping the flag at once. */
export const setPinned = Effect.fn("messages.setPinned")(function* (
  messageId: number,
  pinned: boolean,
) {
  const state = store.getState();
  const held = state.messages[messageId];
  const count = held === undefined ? null : (state.rooms[held.roomId]?.detail?.pinsCount ?? null);

  if (held !== undefined && count !== null) {
    mutations.setPinState({
      messageId,
      roomId: held.roomId,
      pinned,
      pinCount: Math.max(0, count + (pinned ? 1 : -1)),
    });
  }

  const reply = yield* api.setPinned(messageId, pinned).pipe(
    Effect.tapError(() =>
      Effect.sync(() => {
        if (held !== undefined && count !== null) {
          mutations.setPinState({
            messageId,
            roomId: held.roomId,
            pinned: held.pinned,
            pinCount: count,
          });
        }
      }),
    ),
  );

  mutations.setPinState(reply);
});

/** The room's pins (the Pins pane); their authors join the store. */
export const pins = Effect.fn("messages.pins")(function* (roomId: number) {
  const list = yield* api.pins(roomId);

  mutations.mergeUsers(list.users);

  return list;
});

/** Saves it for later. */
export const save = Effect.fn("messages.save")(function* (
  messageId: number,
  remindAt: string | null = null,
) {
  const item = yield* api.saveMessage(messageId, remindAt);

  mutations.applySavedChange(messageId, item);

  return item;
});

/** Unsaves it, at once. */
export const unsave = Effect.fn("messages.unsave")(function* (messageId: number) {
  const savedItemId = store.getState().saved[messageId];

  if (savedItemId === undefined) {
    return;
  }

  const item = store.getState().savedList.items[savedItemId];

  mutations.applySavedChange(messageId, null);

  yield* api
    .unsave(savedItemId)
    .pipe(
      Effect.tapError(() =>
        Effect.sync(() =>
          item === undefined
            ? mutations.setSavedMark(messageId, savedItemId)
            : mutations.applySavedChange(messageId, item),
        ),
      ),
    );
});

/** Where the viewer may forward to. */
export const forwardDestinations = Effect.fn("messages.forwardDestinations")(function* () {
  return yield* api.forwardDestinations();
});

/** Forwards it with an optional note; the copies join their timelines. */
export const forward = Effect.fn("messages.forward")(function* (
  messageId: number,
  note: string | null,
  destinations: readonly ForwardTarget[],
) {
  const result = yield* api.forward(messageId, { note, destinations: [...destinations] });

  for (const copy of result.forwards) {
    mutations.receiveMessage(copy);
  }

  return result.forwards;
});

/** Starts a direct upload (`PUT` the bytes to the reply's `uploadUrl`). */
export const startUpload = Effect.fn("messages.startUpload")(function* (body: CreateUpload) {
  return yield* createUpload(body);
});
