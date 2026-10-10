import { Schema } from "effect";
import type { WorkspaceCategory as GeneratedWorkspaceCategory } from "../../gen/WorkspaceCategory.ts";
import type { WorkspaceLayout as GeneratedWorkspaceLayout } from "../../gen/WorkspaceLayout.ts";
import type { WorkspaceRoomPosition as GeneratedWorkspaceRoomPosition } from "../../gen/WorkspaceRoomPosition.ts";
import { RoomId } from "./ids.ts";
import type { Assert, Pinned } from "./pin.ts";

export const WorkspaceCategory = Schema.Struct({
  id: Schema.Int,
  name: Schema.String,
  position: Schema.Int,
});

export type WorkspaceCategoryPin = Assert<
  Pinned<typeof WorkspaceCategory, GeneratedWorkspaceCategory>
>;

export const WorkspaceRoomPosition = Schema.Struct({
  roomId: RoomId,
  workspaceCategoryId: Schema.NullOr(Schema.Int),
  position: Schema.NullOr(Schema.Int),
});

export type WorkspaceRoomPositionPin = Assert<
  Pinned<typeof WorkspaceRoomPosition, GeneratedWorkspaceRoomPosition>
>;

export const WorkspaceLayout = Schema.Struct({
  categories: Schema.Array(WorkspaceCategory),
  rooms: Schema.Array(WorkspaceRoomPosition),
});

export type WorkspaceLayoutPin = Assert<Pinned<typeof WorkspaceLayout, GeneratedWorkspaceLayout>>;
