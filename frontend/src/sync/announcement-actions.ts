/**
 * Announcement rooms as Effect programs. A published message lands in the store at once (the
 * `message.updated` that follows is then a no-op); the room's flag reaches the header and sidebar
 * through the `sidebar.row.upserted` the server sends for it.
 */
import { Effect } from "effect";
import * as api from "../api/announcement-endpoints.ts";
import { mutations } from "../store/store.ts";

export const setAnnouncement = Effect.fn("announcements.set")(function* (
  roomId: number,
  announcement: boolean,
) {
  return yield* api.setAnnouncement(roomId, announcement);
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
