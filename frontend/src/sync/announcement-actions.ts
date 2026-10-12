/**
 * Announcement rooms as Effect programs. A published message lands in the store at once (the
 * `message.updated` that follows is then a no-op). The room's flag reaches the header and sidebar
 * through the `sidebar.row.upserted` the server sends for it; the write's own reply lands on the
 * sidebar row only when nothing newer has.
 */
import { Clock, Effect } from "effect";
import * as api from "../api/announcement-endpoints.ts";
import { mutations, store } from "../store/store.ts";
import { changedSince, managementEpoch } from "./room-refresh.ts";
import { withRowTicket } from "./row-ticket.ts";

/**
 * Flips the flag. The reply is the room as this write left it, so it must not land over anything
 * sync brought after the write began (another session's change, or this one's own echo): it lands
 * only when the room saw no management change since, its row wasn't touched since, and the reply
 * is newer than the row. Otherwise sync's word stands.
 */
export const setAnnouncement = Effect.fn("announcements.set")(function* (
  roomId: number,
  announcement: boolean,
) {
  const since = managementEpoch();

  // The ticket keeps the sync touches made while the write is out, until its reply has landed.
  return yield* withRowTicket((rowsSince) =>
    Effect.gen(function* () {
      const room = yield* api.setAnnouncement(roomId, announcement);

      if (changedSince(roomId, since)) return room;

      const state = store.getState();
      const row = state.sidebar.rows[roomId];

      // The row is the flag's authority; a reply that isn't strictly newer leaves it to sync.
      if (row === undefined || Date.parse(room.updatedAt) <= Date.parse(row.room.updatedAt)) {
        return room;
      }

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
    }),
  );
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
