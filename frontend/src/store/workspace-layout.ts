/**
 * The shared workspace hierarchy (C3, C14): the categories and channel order an administrator
 * sets for everyone (`WorkspaceLayout` in `crates/api_types/src/organize.rs`). Pure transforms
 * that draw an administrator's change at once, the overlay that shows it while the server
 * confirms it, and the reducers that land the replies.
 *
 * The layout lists every room the viewer may see in it, in the server's order: by category, then
 * position (rooms never organized last), then name. Personal organisation is drawn over it: a
 * favourite, or a channel in one of the viewer's own categories, shows there instead
 * (`sidebarSections` in features/sidebar/sections.ts).
 */
import type { WorkspaceCategory } from "../gen/WorkspaceCategory.ts";
import type { WorkspaceLayout } from "../gen/WorkspaceLayout.ts";
import type { WorkspaceRoomPosition } from "../gen/WorkspaceRoomPosition.ts";
import { isStale } from "./row-touches.ts";
import type { SidebarState, State } from "./state.ts";

/** The layout as the viewer should see it: the pending change while one is on its way. */
export function sharedLayout(sidebar: SidebarState): WorkspaceLayout {
  return sidebar.workspaceOverlay ?? sidebar.workspaceLayout;
}

function byPosition(left: WorkspaceCategory, right: WorkspaceCategory): number {
  return left.position - right.position || left.id - right.id;
}

/** The shared categories in their order. */
export function sharedCategories(layout: WorkspaceLayout): readonly WorkspaceCategory[] {
  return layout.categories.toSorted(byPosition);
}

/** Where `roomId` sits in the layout, if the layout has it. */
export function placementOf(
  layout: WorkspaceLayout,
  roomId: number,
): WorkspaceRoomPosition | undefined {
  return layout.rooms.find((room) => room.roomId === roomId);
}

/**
 * The rooms of one shared group (`null`: the uncategorized one) in order. The layout's own order
 * breaks ties, which is the server's name order for rooms never organized.
 */
export function groupRooms(
  layout: WorkspaceLayout,
  categoryId: number | null,
): readonly WorkspaceRoomPosition[] {
  return layout.rooms
    .filter((room) => room.workspaceCategoryId === categoryId)
    .toSorted(
      (left, right) =>
        (left.position ?? Number.POSITIVE_INFINITY) - (right.position ?? Number.POSITIVE_INFINITY),
    );
}

/** `ids` as one group's placements, numbered 0, 1, … as the server leaves a group it wrote. */
function numbered(
  categoryId: number | null,
  ids: readonly number[],
): readonly WorkspaceRoomPosition[] {
  return ids.map((roomId, position) => ({ roomId, workspaceCategoryId: categoryId, position }));
}

/** `ids` with `id` inserted before `beforeId`, or at the end when that isn't among them. */
function insertBefore(ids: readonly number[], id: number, beforeId: number | null): number[] {
  const at = beforeId === null ? -1 : ids.indexOf(beforeId);

  return at === -1 ? [...ids, id] : [...ids.slice(0, at), id, ...ids.slice(at)];
}

/**
 * The layout with `roomId` moved into `categoryId` just before `beforeRoomId` (at the end with
 * `null`). Both groups it touches are renumbered, as `WorkspaceCategory::move_room` does.
 */
export function withRoomMoved(
  layout: WorkspaceLayout,
  roomId: number,
  categoryId: number | null,
  beforeRoomId: number | null,
): WorkspaceLayout {
  const source = placementOf(layout, roomId)?.workspaceCategoryId ?? null;

  const ids = (group: number | null) =>
    groupRooms(layout, group)
      .map((room) => room.roomId)
      .filter((id) => id !== roomId);

  const target = numbered(categoryId, insertBefore(ids(categoryId), roomId, beforeRoomId));
  const sourceGroup = source === categoryId ? [] : numbered(source, ids(source));

  const rewritten = new Set([source, categoryId]);

  return {
    categories: layout.categories,
    rooms: [
      ...layout.rooms.filter(
        (room) => room.roomId !== roomId && !rewritten.has(room.workspaceCategoryId),
      ),
      ...sourceGroup,
      ...target,
    ],
  };
}

/**
 * The 0-based position `MoveWorkspaceRoom` takes for the move `withRoomMoved` draws. The server
 * counts every room in the group, including any the administrator isn't in, so the place is read
 * off the room it lands before (or after): an organized group is numbered densely, so its
 * neighbour's position is its index in the whole group.
 */
export function serverPosition(
  layout: WorkspaceLayout,
  roomId: number,
  categoryId: number | null,
  beforeRoomId: number | null,
): number {
  const moving = placementOf(layout, roomId);
  const others = groupRooms(layout, categoryId).filter((room) => room.roomId !== roomId);

  const anchorIndex =
    beforeRoomId === null ? -1 : others.findIndex((room) => room.roomId === beforeRoomId);

  const anchor = anchorIndex === -1 ? others.at(-1) : others[anchorIndex];
  const fallback = anchorIndex === -1 ? others.length : anchorIndex;

  if (anchor?.position === null || anchor === undefined) {
    return fallback;
  }

  // The server takes the moving room out of its group before it inserts it again.
  const shift =
    moving?.workspaceCategoryId === categoryId &&
    moving.position !== null &&
    moving.position < anchor.position
      ? 1
      : 0;

  return (anchorIndex === -1 ? anchor.position + 1 : anchor.position) - shift;
}

/** Whether the move would leave the room where it already is. */
export function isPlacedAt(
  layout: WorkspaceLayout,
  roomId: number,
  categoryId: number | null,
  beforeRoomId: number | null,
): boolean {
  const current = placementOf(layout, roomId);

  if (current === undefined || current.workspaceCategoryId !== categoryId) {
    return false;
  }

  const before = groupRooms(layout, categoryId).map((room) => room.roomId);

  const after = groupRooms(withRoomMoved(layout, roomId, categoryId, beforeRoomId), categoryId).map(
    (room) => room.roomId,
  );

  return before.length === after.length && before.every((id, index) => after[index] === id);
}

/** The categories in the order `categoryIds` gives, numbered from 0 as the server does. */
export function withCategoriesOrdered(
  layout: WorkspaceLayout,
  categoryIds: readonly number[],
): WorkspaceLayout {
  const rank = new Map(categoryIds.map((id, index) => [id, index]));

  return {
    categories: sharedCategories(layout).map((category, index) => ({
      ...category,
      position: rank.get(category.id) ?? categoryIds.length + index,
    })),
    rooms: layout.rooms,
  };
}

/** The category added (at its position) or renamed. */
export function withCategory(
  layout: WorkspaceLayout,
  category: WorkspaceCategory,
): WorkspaceLayout {
  return {
    categories: [...layout.categories.filter((held) => held.id !== category.id), category].sort(
      byPosition,
    ),
    rooms: layout.rooms,
  };
}

/**
 * The category gone, as `WorkspaceCategory::destroy` leaves it: its rooms appended to the
 * uncategorized group and both renumbered, the other categories numbered from 0 again.
 */
export function withoutCategory(layout: WorkspaceLayout, categoryId: number): WorkspaceLayout {
  const ids = [
    ...groupRooms(layout, null).map((room) => room.roomId),
    ...groupRooms(layout, categoryId).map((room) => room.roomId),
  ];

  return {
    categories: sharedCategories(layout)
      .filter((category) => category.id !== categoryId)
      .map((category, position) => ({ ...category, position })),
    rooms: [
      ...layout.rooms.filter(
        (room) => room.workspaceCategoryId !== null && room.workspaceCategoryId !== categoryId,
      ),
      ...numbered(null, ids),
    ],
  };
}

/** Draws a pending shared change over the layout (a later one replaces it). */
export function addWorkspaceOverlay(state: State, layout: WorkspaceLayout): State {
  return { ...state, sidebar: { ...state.sidebar, workspaceOverlay: layout } };
}

/** Drops a settled change's overlay, unless a newer change has replaced it meanwhile. */
export function dropWorkspaceOverlay(state: State, layout: WorkspaceLayout): State {
  return state.sidebar.workspaceOverlay === layout
    ? { ...state, sidebar: { ...state.sidebar, workspaceOverlay: null } }
    : state;
}

/**
 * Lands an administrator's reply, `since` its request's ticket, as `change` to the layout held:
 * nothing once a resync or a `workspace.layout.updated` has landed after the request began (the
 * server publishes the change to every member, so that copy already has it, or will).
 */
export function landWorkspaceChange(
  state: State,
  change: (layout: WorkspaceLayout) => WorkspaceLayout,
  since: number,
): State {
  if (isStale(state, since) || state.sidebar.workspaceLayoutTouchedAt > since) {
    return state;
  }

  const clock = state.rowTouches.clock + 1;

  return {
    ...state,
    rowTouches: { ...state.rowTouches, clock },
    sidebar: {
      ...state.sidebar,
      workspaceLayout: change(state.sidebar.workspaceLayout),
      workspaceLayoutTouchedAt: clock,
    },
  };
}
