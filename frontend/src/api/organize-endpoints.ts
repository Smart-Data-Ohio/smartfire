/**
 * The S3 sidebar organisation endpoints (`crates/api_types/src/organize.rs`): the viewer's own
 * categories, favourites and notification level, and the shared workspace layout administrators
 * set for everyone. Each validates its reply with the pinned schema; every write also publishes
 * its sync event, which the reply has already landed.
 */
import { Effect } from "effect";
import type { CreateRoomCategory } from "../gen/CreateRoomCategory.ts";
import type { FavoriteList } from "../gen/FavoriteList.ts";
import type { Involvement } from "../gen/Involvement.ts";
import type { InvolvementChange } from "../gen/InvolvementChange.ts";
import type { MoveWorkspaceRoom } from "../gen/MoveWorkspaceRoom.ts";
import type { ReorderWorkspaceCategories } from "../gen/ReorderWorkspaceCategories.ts";
import type { RoomCategory } from "../gen/RoomCategory.ts";
import type { RoomCategoryList } from "../gen/RoomCategoryList.ts";
import type { SidebarRow } from "../gen/SidebarRow.ts";
import type { UpdateRoomCategory } from "../gen/UpdateRoomCategory.ts";
import type { WorkspaceCategory } from "../gen/WorkspaceCategory.ts";
import type { WorkspaceLayout } from "../gen/WorkspaceLayout.ts";
import type { WriteWorkspaceCategory } from "../gen/WriteWorkspaceCategory.ts";
import { call, noContent } from "./call.ts";
import {
  FavoriteList as FavoriteListSchema,
  InvolvementChange as InvolvementChangeSchema,
  RoomCategoryList as RoomCategoryListSchema,
} from "./schema/organize.ts";
import {
  RoomCategory as RoomCategorySchema,
  SidebarRow as SidebarRowSchema,
} from "./schema/sidebar.ts";
import {
  WorkspaceCategory as WorkspaceCategorySchema,
  WorkspaceLayout as WorkspaceLayoutSchema,
} from "./schema/workspace-layout.ts";
import { wire } from "./wire.ts";

/** `POST /room_categories` (201): a new category at the end. 422 for a blank or long name. */
export const createCategory = Effect.fn("api.createCategory")(function* (body: CreateRoomCategory) {
  return yield* call(
    { method: "POST", path: "/room_categories", body },
    wire<RoomCategory>(RoomCategorySchema),
  );
});

/** `PATCH /room_categories/:id`: renames or folds it; a field left out keeps its value. */
export const updateCategory = Effect.fn("api.updateCategory")(function* (
  categoryId: number,
  body: UpdateRoomCategory,
) {
  return yield* call(
    { method: "PATCH", path: `/room_categories/${categoryId}`, body },
    wire<RoomCategory>(RoomCategorySchema),
  );
});

/** `DELETE /room_categories/:id` (204): its rooms go back to Channels. */
export const deleteCategory = Effect.fn("api.deleteCategory")(function* (categoryId: number) {
  return yield* call({ method: "DELETE", path: `/room_categories/${categoryId}` }, noContent);
});

/**
 * `PUT /room_categories/order`: every category once, in the new order. A list that doesn't match
 * the server's (a category added or removed elsewhere) is a 409 `Conflict`.
 */
export const reorderCategories = Effect.fn("api.reorderCategories")(function* (
  categoryIds: readonly number[],
) {
  return yield* call(
    { method: "PUT", path: "/room_categories/order", body: { categoryIds: [...categoryIds] } },
    wire<RoomCategoryList>(RoomCategoryListSchema),
  );
});

/** `PUT /rooms/:id/category`: into a category, or out with `null`. Open and closed channels only. */
export const assignCategory = Effect.fn("api.assignCategory")(function* (
  roomId: number,
  roomCategoryId: number | null,
) {
  return yield* call(
    { method: "PUT", path: `/rooms/${roomId}/category`, body: { roomCategoryId } },
    wire<SidebarRow>(SidebarRowSchema),
  );
});

/** `POST /rooms/:id/favorite`: appends the room to the favourites (already one: unchanged). */
export const favorite = Effect.fn("api.favorite")(function* (roomId: number) {
  return yield* call(
    { method: "POST", path: `/rooms/${roomId}/favorite` },
    wire<SidebarRow>(SidebarRowSchema),
  );
});

/** `DELETE /rooms/:id/favorite`: no longer a favourite; the others keep their positions. */
export const unfavorite = Effect.fn("api.unfavorite")(function* (roomId: number) {
  return yield* call(
    { method: "DELETE", path: `/rooms/${roomId}/favorite` },
    wire<SidebarRow>(SidebarRowSchema),
  );
});

/** `PATCH /rooms/:id/favorite`: to a 0-based position among the favourites (clamped). */
export const moveFavorite = Effect.fn("api.moveFavorite")(function* (
  roomId: number,
  position: number,
) {
  return yield* call(
    { method: "PATCH", path: `/rooms/${roomId}/favorite`, body: { position } },
    wire<FavoriteList>(FavoriteListSchema),
  );
});

/** `PUT /rooms/:id/involvement`: the membership and revised settings. */
export const updateInvolvement = Effect.fn("api.updateInvolvement")(function* (
  roomId: number,
  involvement: Involvement,
) {
  return yield* call(
    { method: "PUT", path: `/rooms/${roomId}/involvement`, body: { involvement } },
    wire<InvolvementChange>(InvolvementChangeSchema),
  );
});

/** `POST /workspace_categories` (201, administrators): a shared category at the end. */
export const createWorkspaceCategory = Effect.fn("api.createWorkspaceCategory")(function* (
  body: WriteWorkspaceCategory,
) {
  return yield* call(
    { method: "POST", path: "/workspace_categories", body },
    wire<WorkspaceCategory>(WorkspaceCategorySchema),
  );
});

/** `PATCH /workspace_categories/:id` (administrators): renames a shared category. */
export const updateWorkspaceCategory = Effect.fn("api.updateWorkspaceCategory")(function* (
  categoryId: number,
  body: WriteWorkspaceCategory,
) {
  return yield* call(
    { method: "PATCH", path: `/workspace_categories/${categoryId}`, body },
    wire<WorkspaceCategory>(WorkspaceCategorySchema),
  );
});

/** `DELETE /workspace_categories/:id` (204, administrators): its rooms go to the uncategorized end. */
export const deleteWorkspaceCategory = Effect.fn("api.deleteWorkspaceCategory")(function* (
  categoryId: number,
) {
  return yield* call({ method: "DELETE", path: `/workspace_categories/${categoryId}` }, noContent);
});

/**
 * `PUT /workspace_categories/order` (administrators): every shared category once, in the new
 * order; a list that doesn't match the server's is a 409 `Conflict`. Answers the layout.
 */
export const reorderWorkspaceCategories = Effect.fn("api.reorderWorkspaceCategories")(function* (
  categoryIds: readonly number[],
) {
  return yield* call(
    {
      method: "PUT",
      path: "/workspace_categories/order",
      body: { categoryIds: [...categoryIds] } satisfies ReorderWorkspaceCategories,
    },
    wire<WorkspaceLayout>(WorkspaceLayoutSchema),
  );
});

/**
 * `PUT /rooms/:id/workspace_category` (administrators): into a shared category (`null`: none) at a
 * 0-based, clamped position. Not for direct messages (422). Answers the layout.
 */
export const moveWorkspaceRoom = Effect.fn("api.moveWorkspaceRoom")(function* (
  roomId: number,
  body: MoveWorkspaceRoom,
) {
  return yield* call(
    { method: "PUT", path: `/rooms/${roomId}/workspace_category`, body },
    wire<WorkspaceLayout>(WorkspaceLayoutSchema),
  );
});
