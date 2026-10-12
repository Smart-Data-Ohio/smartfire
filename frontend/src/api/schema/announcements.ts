import { Schema } from "effect";
import type { AnnouncementFollow as GeneratedAnnouncementFollow } from "../../gen/AnnouncementFollow.ts";
import type { AnnouncementFollows as GeneratedAnnouncementFollows } from "../../gen/AnnouncementFollows.ts";
import type { CreateAnnouncementFollow as GeneratedCreateAnnouncementFollow } from "../../gen/CreateAnnouncementFollow.ts";
import type { SetAnnouncement as GeneratedSetAnnouncement } from "../../gen/SetAnnouncement.ts";
import { RoomId, UserId } from "./ids.ts";
import type { Assert, Pinned } from "./pin.ts";
import { Timestamp } from "./time.ts";

/** `PUT /api/v1/rooms/:room_id/announcement`. */
export const SetAnnouncement = Schema.Struct({ announcement: Schema.Boolean });

export type SetAnnouncementPin = Assert<Pinned<typeof SetAnnouncement, GeneratedSetAnnouncement>>;

/** `POST /api/v1/rooms/:source_room_id/announcement_follows`: the room that gets the copies. */
export const CreateAnnouncementFollow = Schema.Struct({ targetRoomId: RoomId });

export type CreateAnnouncementFollowPin = Assert<
  Pinned<typeof CreateAnnouncementFollow, GeneratedCreateAnnouncementFollow>
>;

/** An announcement room (`source`) whose published messages are copied into `target`. */
export const AnnouncementFollow = Schema.Struct({
  sourceRoomId: RoomId,
  targetRoomId: RoomId,
  creatorId: UserId,
  createdAt: Timestamp,
});

export type AnnouncementFollowPin = Assert<
  Pinned<typeof AnnouncementFollow, GeneratedAnnouncementFollow>
>;

/**
 * `GET /api/v1/rooms/:room_id/announcement_follows`: the rooms following this one (for its
 * managers only) and the announcement rooms it follows, each limited to rooms the viewer can see.
 */
export const AnnouncementFollows = Schema.Struct({
  followers: Schema.Array(AnnouncementFollow),
  following: Schema.Array(AnnouncementFollow),
});

export type AnnouncementFollowsPin = Assert<
  Pinned<typeof AnnouncementFollows, GeneratedAnnouncementFollows>
>;
