import { Schema } from "effect";
import type { ChannelDirectory as GeneratedChannelDirectory } from "../../gen/ChannelDirectory.ts";
import type { DirectoryChannel as GeneratedDirectoryChannel } from "../../gen/DirectoryChannel.ts";
import type { OpenRoomPreview as GeneratedOpenRoomPreview } from "../../gen/OpenRoomPreview.ts";
import type { RoomJoin as GeneratedRoomJoin } from "../../gen/RoomJoin.ts";
import { RoomId } from "./ids.ts";
import type { Assert, Pinned } from "./pin.ts";
import { RoomDetail } from "./room.ts";
import { SidebarRow } from "./sidebar.ts";
import { Timestamp } from "./time.ts";

/** `GET /rooms/:id/preview`: the name, and nothing from the timeline. */
export const OpenRoomPreview = Schema.Struct({
  id: RoomId,
  name: Schema.String,
});

export type OpenRoomPreview = typeof OpenRoomPreview.Type;

export type OpenRoomPreviewPin = Assert<Pinned<typeof OpenRoomPreview, GeneratedOpenRoomPreview>>;

/**
 * `POST /rooms/:id/join`: the room once the viewer belongs to it, a hidden membership shown
 * again. `row` is their sidebar row, null only when it was hidden again before the reply.
 */
export const RoomJoin = Schema.Struct({
  detail: RoomDetail,
  row: Schema.NullOr(SidebarRow),
});

export type RoomJoin = typeof RoomJoin.Type;

export type RoomJoinPin = Assert<Pinned<typeof RoomJoin, GeneratedRoomJoin>>;

/** One open room in `GET /rooms/browse`. `joined`: it shows in the viewer's sidebar. */
export const DirectoryChannel = Schema.Struct({
  roomId: RoomId,
  name: Schema.String,
  topic: Schema.NullOr(Schema.String),
  memberCount: Schema.Number,
  lastActiveAt: Timestamp,
  joined: Schema.Boolean,
});

export type DirectoryChannel = typeof DirectoryChannel.Type;

export type DirectoryChannelPin = Assert<
  Pinned<typeof DirectoryChannel, GeneratedDirectoryChannel>
>;

/** `GET /rooms/browse?q=&cursor=`: a page of the open rooms anyone may join, best match first. */
export const ChannelDirectory = Schema.Struct({
  channels: Schema.Array(DirectoryChannel),
  nextCursor: Schema.NullOr(Schema.String),
});

export type ChannelDirectory = typeof ChannelDirectory.Type;

export type ChannelDirectoryPin = Assert<
  Pinned<typeof ChannelDirectory, GeneratedChannelDirectory>
>;
