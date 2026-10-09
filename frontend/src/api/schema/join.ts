import { Schema } from "effect";
import type { OpenRoomPreview as GeneratedOpenRoomPreview } from "../../gen/OpenRoomPreview.ts";
import type { RoomJoin as GeneratedRoomJoin } from "../../gen/RoomJoin.ts";
import { RoomId } from "./ids.ts";
import type { Assert, Pinned } from "./pin.ts";
import { RoomDetail } from "./room.ts";
import { SidebarRow } from "./sidebar.ts";

/** `GET /rooms/:id/preview`: the name, and nothing from the timeline. */
export const OpenRoomPreview = Schema.Struct({
  id: RoomId,
  name: Schema.String,
});

export type OpenRoomPreview = typeof OpenRoomPreview.Type;

export type OpenRoomPreviewPin = Assert<Pinned<typeof OpenRoomPreview, GeneratedOpenRoomPreview>>;

/**
 * `POST /rooms/:id/join`: the room once the viewer belongs to it. `row` is their sidebar row, or
 * null when the membership is invisible and stays out of the sidebar.
 */
export const RoomJoin = Schema.Struct({
  detail: RoomDetail,
  row: Schema.NullOr(SidebarRow),
});

export type RoomJoin = typeof RoomJoin.Type;

export type RoomJoinPin = Assert<Pinned<typeof RoomJoin, GeneratedRoomJoin>>;
