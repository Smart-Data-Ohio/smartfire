/**
 * Announcement rooms: marking a room as one, publishing its messages, and the follows that copy
 * published messages into other rooms.
 */
import { Effect } from "effect";
import type { AnnouncementFollows } from "../gen/AnnouncementFollows.ts";
import type { MessageDTO } from "../gen/MessageDTO.ts";
import type { Room } from "../gen/Room.ts";
import { call, get, noContent } from "./call.ts";
import { AnnouncementFollows as AnnouncementFollowsSchema } from "./schema/announcements.ts";
import { MessageDTO as MessageSchema } from "./schema/message.ts";
import { Room as RoomSchema } from "./schema/room.ts";
import { wire } from "./wire.ts";

/** Makes the room an announcement room, or an ordinary one again. Its creator and administrators. */
export const setAnnouncement = Effect.fn("api.setAnnouncement")(function* (
  roomId: number,
  announcement: boolean,
) {
  return yield* call(
    { method: "PUT", path: `/rooms/${roomId}/announcement`, body: { announcement } },
    wire<Room>(RoomSchema),
  );
});

/** Copies the message into every room following its announcement room; again is a no-op. */
export const publishMessage = Effect.fn("api.publishMessage")(function* (messageId: number) {
  return yield* call(
    { method: "POST", path: `/messages/${messageId}/publish` },
    wire<MessageDTO>(MessageSchema),
  );
});

export const announcementFollows = Effect.fn("api.announcementFollows")(function* (roomId: number) {
  return yield* call(
    get(`/rooms/${roomId}/announcement_follows`),
    wire<AnnouncementFollows>(AnnouncementFollowsSchema),
  );
});

/** `targetRoomId` follows `sourceRoomId`. Answers the source room's follows. */
export const followAnnouncements = Effect.fn("api.followAnnouncements")(function* (
  sourceRoomId: number,
  targetRoomId: number,
) {
  return yield* call(
    {
      method: "POST",
      path: `/rooms/${sourceRoomId}/announcement_follows`,
      body: { targetRoomId },
    },
    wire<AnnouncementFollows>(AnnouncementFollowsSchema),
  );
});

export const unfollowAnnouncements = Effect.fn("api.unfollowAnnouncements")(function* (
  sourceRoomId: number,
  targetRoomId: number,
) {
  return yield* call(
    {
      method: "DELETE",
      path: `/rooms/${sourceRoomId}/announcement_follows/${targetRoomId}`,
    },
    noContent,
  );
});
