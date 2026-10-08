import { Schema } from "effect";
import type { OpenRoomPreview as GeneratedOpenRoomPreview } from "../../gen/OpenRoomPreview.ts";
import type { RoomJoin as GeneratedRoomJoin } from "../../gen/RoomJoin.ts";
import { RoomId } from "./ids.ts";
import type { Assert, Pinned } from "./pin.ts";
import { RoomDetail } from "./room.ts";
import { SidebarRow } from "./sidebar.ts";

/** `GET /rooms/:id/preview`: name and member count, and nothing from the timeline. */
export const OpenRoomPreview = Schema.Struct({
  id: RoomId,
  name: Schema.String,
  memberCount: Schema.Int,
});

export type OpenRoomPreview = typeof OpenRoomPreview.Type;

export type OpenRoomPreviewPin = Assert<Pinned<typeof OpenRoomPreview, GeneratedOpenRoomPreview>>;

/** `POST /rooms/:id/join`: the room once the viewer belongs to it, and their sidebar row. */
export const RoomJoin = Schema.Struct({
  detail: RoomDetail,
  row: SidebarRow,
});

export type RoomJoin = typeof RoomJoin.Type;

export type RoomJoinPin = Assert<Pinned<typeof RoomJoin, GeneratedRoomJoin>>;
