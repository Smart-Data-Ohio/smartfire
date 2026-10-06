/**
 * Small Effect programs the message UI needs beyond `actions.messages`: fetching people a
 * reaction tooltip names but the store hasn't seen, marking a room unread from a message, and the
 * workspace's custom icons for the emoji picker. React runs them with `runAction(...)`.
 */
import { Clock, Effect } from "effect";
import { icons } from "../api/composer-endpoints.ts";
import { markUnread, users } from "../api/endpoints.ts";
import { mutations, store } from "../store/store.ts";

/** Loads the people among `ids` the store doesn't hold yet (at most 100 per request). */
export const ensureUsers = Effect.fn("messages.ensureUsers")(function* (ids: readonly number[]) {
  const known = store.getState().users;
  const missing = [...new Set(ids)].filter((id) => known[id] === undefined).slice(0, 100);

  if (missing.length === 0) {
    return;
  }

  const list = yield* users(missing);

  mutations.mergeUsers(list.users);
});

/**
 * Marks the room unread from `messageId` on, here at once (the sidebar row turns unread) and on
 * the server. Answers the first unread message's id.
 */
export const markUnreadFrom = Effect.fn("messages.markUnreadFrom")(function* (
  roomId: number,
  messageId: number,
) {
  const reply = yield* markUnread(roomId, messageId);
  const userId = store.getState().me?.user.id ?? 0;

  mutations.applyEvents(
    [
      {
        // A local echo, not a frame from the socket: the engine's cursor never sees it.
        seq: 0,
        topic: `user:${userId}`,
        type: "room.unread",
        data: { roomId, messageId: null, mentioned: false },
      },
    ],
    yield* Clock.currentTimeMillis,
  );

  return reply.firstUnreadMessageId;
});

/** Every brand and workspace icon, for the picker's Custom tab. */
export const customIcons = Effect.fn("messages.customIcons")(function* () {
  return (yield* icons()).icons;
});
