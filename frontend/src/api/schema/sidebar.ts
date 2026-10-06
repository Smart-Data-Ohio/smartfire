import { Schema } from "effect";
import type { RoomCategory as GeneratedRoomCategory } from "../../gen/RoomCategory.ts";
import type { Sidebar as GeneratedSidebar } from "../../gen/Sidebar.ts";
import type { SidebarRow as GeneratedSidebarRow } from "../../gen/SidebarRow.ts";
import type { SidebarRowRemoved as GeneratedSidebarRowRemoved } from "../../gen/SidebarRowRemoved.ts";
import { RoomCategoryId, RoomId, UserId } from "./ids.ts";
import type { Assert, Pinned } from "./pin.ts";
import { Membership, Room } from "./room.ts";
import { User } from "./user.ts";

/** A room as it appears in one person's sidebar, with its unread and mention counts. */
export const SidebarRow = Schema.Struct({
  room: Room,
  membership: Membership,
  displayName: Schema.String,
  directMemberIds: Schema.Array(UserId),
  unreadCount: Schema.Int,
  mentionCount: Schema.Int,
});

export type SidebarRow = typeof SidebarRow.Type;

export type SidebarRowPin = Assert<Pinned<typeof SidebarRow, GeneratedSidebarRow>>;

/** A person's own sidebar section. */
export const RoomCategory = Schema.Struct({
  id: RoomCategoryId,
  name: Schema.String,
  collapsed: Schema.Boolean,
  position: Schema.Int,
});

export type RoomCategory = typeof RoomCategory.Type;

export type RoomCategoryPin = Assert<Pinned<typeof RoomCategory, GeneratedRoomCategory>>;

/** `GET /api/v1/sidebar`: every row of the viewer's sidebar plus what the rows refer to. */
export const Sidebar = Schema.Struct({
  rows: Schema.Array(SidebarRow),
  categories: Schema.Array(RoomCategory),
  users: Schema.Array(User),
  directPlaceholderUserIds: Schema.Array(UserId),
  canCreateRooms: Schema.Boolean,
});

export type Sidebar = typeof Sidebar.Type;

export type SidebarPin = Assert<Pinned<typeof Sidebar, GeneratedSidebar>>;

/** The `sidebar.row.removed` event's data. */
export const SidebarRowRemoved = Schema.Struct({ roomId: RoomId });

export type SidebarRowRemovedPin = Assert<
  Pinned<typeof SidebarRowRemoved, GeneratedSidebarRowRemoved>
>;
