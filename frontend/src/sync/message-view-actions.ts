/**
 * Small Effect programs the message UI needs beyond the message writes: fetching people the
 * store hasn't seen (reaction tooltips, reply indicators), marking a room unread from a message,
 * and the workspace's custom icons for the emoji picker. React reaches them through `actions`.
 */
import { Clock, Effect } from "effect";
import { icons } from "../api/composer-endpoints.ts";
import { markUnread, users } from "../api/endpoints.ts";
import { mutations, store } from "../store/store.ts";

/** The most people one `GET /users` names. */
const USERS_PER_REQUEST = 100;

/** Loads the people among `ids` the store doesn't hold yet, 100 per request. */
export const ensureUsers = Effect.fn("messages.ensureUsers")(function* (ids: readonly number[]) {
  const known = store.getState().users;
  const missing = [...new Set(ids)].filter((id) => known[id] === undefined);

  for (let start = 0; start < missing.length; start += USERS_PER_REQUEST) {
    const list = yield* users(missing.slice(start, start + USERS_PER_REQUEST));

    mutations.mergeUsers(list.users);
  }
});

/**
 * Marks the room unread from `messageId` on, on the server and then here: the divider moves to the
 * first unread message and the sidebar row counts what the divider counts. Answers that message's
 * id.
 */
export const markUnreadFrom = Effect.fn("messages.markUnreadFrom")(function* (
  roomId: number,
  messageId: number,
) {
  const reply = yield* markUnread(roomId, messageId);

  mutations.markUnreadFrom(
    roomId,
    reply.firstUnreadMessageId ?? messageId,
    yield* Clock.currentTimeMillis,
  );

  return reply.firstUnreadMessageId;
});

/** Every brand and workspace icon, for the picker's Custom tab. */
export const customIcons = Effect.fn("messages.customIcons")(function* () {
  return (yield* icons()).icons;
});
