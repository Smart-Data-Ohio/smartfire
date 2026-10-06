import { Schema } from "effect";
import type { AssignRoomCategory as GeneratedAssignRoomCategory } from "../../gen/AssignRoomCategory.ts";
import type { CreateRoomCategory as GeneratedCreateRoomCategory } from "../../gen/CreateRoomCategory.ts";
import type { FavoriteList as GeneratedFavoriteList } from "../../gen/FavoriteList.ts";
import type { MoveFavorite as GeneratedMoveFavorite } from "../../gen/MoveFavorite.ts";
import type { ReorderRoomCategories as GeneratedReorderRoomCategories } from "../../gen/ReorderRoomCategories.ts";
import type { RoomCategoryList as GeneratedRoomCategoryList } from "../../gen/RoomCategoryList.ts";
import type { RoomCategoryRemoved as GeneratedRoomCategoryRemoved } from "../../gen/RoomCategoryRemoved.ts";
import type { UpdateInvolvement as GeneratedUpdateInvolvement } from "../../gen/UpdateInvolvement.ts";
import type { UpdateRoomCategory as GeneratedUpdateRoomCategory } from "../../gen/UpdateRoomCategory.ts";
import { RoomCategoryId } from "./ids.ts";
import type { Assert, Pinned } from "./pin.ts";
import { Involvement } from "./room.ts";
import { RoomCategory, SidebarRow } from "./sidebar.ts";

/** The body of `POST /api/v1/room_categories`: a name of 1 to 50 characters. */
export const CreateRoomCategory = Schema.Struct({
  name: Schema.String,
  collapsed: Schema.optionalKey(Schema.Boolean),
});

export type CreateRoomCategory = typeof CreateRoomCategory.Type;

export type CreateRoomCategoryPin = Assert<
  Pinned<typeof CreateRoomCategory, GeneratedCreateRoomCategory>
>;

/** The body of `PATCH /api/v1/room_categories/:id`; `null` keeps a field. */
export const UpdateRoomCategory = Schema.Struct({
  name: Schema.optionalKey(Schema.String),
  collapsed: Schema.optionalKey(Schema.Boolean),
});

export type UpdateRoomCategory = typeof UpdateRoomCategory.Type;

export type UpdateRoomCategoryPin = Assert<
  Pinned<typeof UpdateRoomCategory, GeneratedUpdateRoomCategory>
>;

/** The body of `PUT /api/v1/room_categories/order`: every category once, in the new order. */
export const ReorderRoomCategories = Schema.Struct({ categoryIds: Schema.Array(RoomCategoryId) });

export type ReorderRoomCategories = typeof ReorderRoomCategories.Type;

export type ReorderRoomCategoriesPin = Assert<
  Pinned<typeof ReorderRoomCategories, GeneratedReorderRoomCategories>
>;

export const RoomCategoryList = Schema.Struct({ categories: Schema.Array(RoomCategory) });

export type RoomCategoryList = typeof RoomCategoryList.Type;

export type RoomCategoryListPin = Assert<
  Pinned<typeof RoomCategoryList, GeneratedRoomCategoryList>
>;

/** The `sidebar.category.removed` event. */
export const RoomCategoryRemoved = Schema.Struct({ id: RoomCategoryId });

export type RoomCategoryRemoved = typeof RoomCategoryRemoved.Type;

export type RoomCategoryRemovedPin = Assert<
  Pinned<typeof RoomCategoryRemoved, GeneratedRoomCategoryRemoved>
>;

/** The body of `PUT /api/v1/rooms/:id/category`; open and closed channels only. */
export const AssignRoomCategory = Schema.Struct({ roomCategoryId: Schema.NullOr(RoomCategoryId) });

export type AssignRoomCategory = typeof AssignRoomCategory.Type;

export type AssignRoomCategoryPin = Assert<
  Pinned<typeof AssignRoomCategory, GeneratedAssignRoomCategory>
>;

/** The body of `PATCH /api/v1/rooms/:id/favorite`: a 0-based position, clamped. */
export const MoveFavorite = Schema.Struct({ position: Schema.Int });

export type MoveFavorite = typeof MoveFavorite.Type;

export type MoveFavoritePin = Assert<Pinned<typeof MoveFavorite, GeneratedMoveFavorite>>;

/** Every favourite in order, after a move. */
export const FavoriteList = Schema.Struct({ rows: Schema.Array(SidebarRow) });

export type FavoriteList = typeof FavoriteList.Type;

export type FavoriteListPin = Assert<Pinned<typeof FavoriteList, GeneratedFavoriteList>>;

/** The body of `PUT /api/v1/rooms/:id/involvement`. */
export const UpdateInvolvement = Schema.Struct({ involvement: Involvement });

export type UpdateInvolvement = typeof UpdateInvolvement.Type;

export type UpdateInvolvementPin = Assert<
  Pinned<typeof UpdateInvolvement, GeneratedUpdateInvolvement>
>;
