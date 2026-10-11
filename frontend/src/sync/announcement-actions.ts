/**
 * Announcement rooms as Effect programs. A published message lands in the store at once (the
 * `message.updated` that follows is then a no-op). The room's flag reaches the header and sidebar
 * through the `sidebar.row.upserted` the server sends for it; the write's own reply lands on the
 * sidebar row only when nothing newer has.
 */
import { Clock, Effect } from "effect";
import * as api from "../api/announcement-endpoints.ts";
import { storedRoom } from "../store/announcements.ts";
import { mutations, sidebarRowClock, store } from "../store/store.ts";
import { changedSince, managementEpoch } from "./room-refresh.ts";

/**
 * Flips the flag. The reply is the room as this write left it, so it must not land over anything
 * sync brought after the write began (another session's change, or this one's own echo): it lands
 * only when the room saw no management change since, its row wasn't touched since, and the reply
 * is newer than every copy the store holds. Otherwise sync's word stands.
 */
export const setAnnouncement = Effect.fn("announcements.set")(function* (
  roomId: number,
  announcement: boolean,
) {
  const since = managementEpoch();
  const rowsSince = sidebarRowClock();
  const room = yield* api.setAnnouncement(roomId, announcement);

  if (changedSince(roomId, since)) return room;

  const state = store.getState();
  const row = state.sidebar.rows[roomId];
  const current = storedRoom(state, roomId);

  if (row === undefined || current === null) return room;

  if (Date.parse(room.updatedAt) <= Date.parse(current.updatedAt)) return room;

  const viewerId = state.me?.user.id ?? state.boot?.user.id ?? 0;

  mutations.landReplyRows(
    [
      {
        seq: 0,
        topic: `user:${viewerId}`,
        type: "sidebar.row.upserted",
        data: { ...row, room },
      },
    ],
    yield* Clock.currentTimeMillis,
    rowsSince,
  );

  return room;
});

export const publish = Effect.fn("announcements.publish")(function* (messageId: number) {
  const message = yield* api.publishMessage(messageId);

  mutations.updateMessage(message);

  return message;
});

export const follows = Effect.fn("announcements.follows")(function* (roomId: number) {
  return yield* api.announcementFollows(roomId);
});

export const follow = Effect.fn("announcements.follow")(function* (
  sourceRoomId: number,
  targetRoomId: number,
) {
  return yield* api.followAnnouncements(sourceRoomId, targetRoomId);
});

export const unfollow = Effect.fn("announcements.unfollow")(function* (
  sourceRoomId: number,
  targetRoomId: number,
) {
  yield* api.unfollowAnnouncements(sourceRoomId, targetRoomId);
});
