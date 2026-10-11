/**
 * The sidebar's organising commands, as the menus, drags and fields call them: each runs the
 * optimistic action and turns a refusal into a toast (the change has already been put back).
 */

import type { Involvement } from "../../gen/Involvement.ts";
import type { WorkspaceCategory } from "../../gen/WorkspaceCategory.ts";
import type { RoomCategory, SidebarRow } from "../../store/model.ts";
import { organizedSidebar, type RoomSlot } from "../../store/organize.ts";
import { store } from "../../store/store.ts";
import type { ActionError } from "../../sync/run.ts";
import { actions } from "../../sync/runtime.ts";
import { toast } from "../../ui/toast-store.ts";
import { involvementChoice, personallyPlaced } from "./organize-model.ts";

let beforeChange: (() => void) | null = null;

/**
 * Registers what runs just before each organising change shows (the sidebar's FLIP capture), so
 * moves from a menu animate like drops do. Returns the unregister function.
 */
export function onBeforeOrganize(capture: () => void): () => void {
  beforeChange = capture;

  return () => {
    if (beforeChange === capture) {
      beforeChange = null;
    }
  };
}

/** Where the viewer has put a room of their own accord: starred, or in a category of theirs. */
interface PersonalPlace {
  readonly favoritePosition: number | null;
  readonly roomCategoryId: number | null;
}

/** The room's personal place as the sidebar shows it now, or `null` when it has none. */
function personalPlace(roomId: number): PersonalPlace | null {
  const row = organizedSidebar(store.getState().sidebar).rows[roomId];

  if (row === undefined || !personallyPlaced(row)) {
    return null;
  }

  const { favoritePosition, roomCategoryId } = row.membership;

  return { favoritePosition, roomCategoryId };
}

const samePlace = (left: PersonalPlace | null, right: PersonalPlace | null) =>
  left !== null &&
  right !== null &&
  left.favoritePosition === right.favoritePosition &&
  left.roomCategoryId === right.roomCategoryId;

function failed(title: string): (error: ActionError) => void {
  return (error) => {
    toast({ title, description: error.message, tone: "danger" });
  };
}

export function moveRoom(row: SidebarRow, slot: RoomSlot): void {
  beforeChange?.();

  actions.organize.moveRoom(row.room.id, slot).catch(failed(`Couldn't move ${row.displayName}`));
}

export function toggleFavorite(row: SidebarRow): void {
  beforeChange?.();

  const starred = row.membership.favoritePosition !== null;

  const change = starred
    ? actions.organize.unfavorite(row.room.id)
    : actions.organize.favorite(row.room.id);

  change.catch(
    failed(starred ? "Couldn't remove it from Favourites" : "Couldn't add it to Favourites"),
  );
}

/**
 * A new category at the end; with `row`, that channel moves into it. Resolves to the category the
 * server made, or `null` after a refusal (already toasted).
 */
export function createCategory(
  name: string,
  row: SidebarRow | null = null,
): Promise<RoomCategory | null> {
  beforeChange?.();

  return actions.organize.createCategory(name, row?.room.id ?? null).catch((error: ActionError) => {
    failed(`Couldn't create "${name.trim()}"`)(error);

    return null;
  });
}

export function renameCategory(category: RoomCategory, name: string): void {
  actions.organize.renameCategory(category.id, name).catch(failed("Couldn't rename the category"));
}

export function setCollapsed(category: RoomCategory, collapsed: boolean): void {
  actions.organize
    .setCollapsed(category.id, collapsed)
    .catch(failed(`Couldn't ${collapsed ? "fold" : "unfold"} ${category.name}`));
}

export function deleteCategory(category: RoomCategory): void {
  beforeChange?.();

  actions.organize.deleteCategory(category.id).catch(failed(`Couldn't delete ${category.name}`));
}

export function reorderCategories(categoryIds: readonly number[]): void {
  beforeChange?.();

  actions.organize.reorderCategories(categoryIds).catch((error: ActionError) => {
    if (error.tag === "Conflict") {
      toast({
        title: "Your categories changed elsewhere",
        description: "This is the latest order. Try the move again.",
      });

      return;
    }

    failed("Couldn't reorder the categories")(error);
  });
}

/**
 * Sets the notification level. Hiding a room takes it out of the sidebar, so that one offers an
 * Undo.
 */
export function setInvolvement(
  roomId: number,
  name: string,
  level: Involvement,
  previous: Involvement,
): void {
  beforeChange?.();

  actions.organize
    .setInvolvement(roomId, level)
    .then(() => {
      if (level === "invisible" && previous !== "invisible") {
        toast({
          title: `${name} is hidden from the sidebar`,
          description: "Jump to still finds it.",
          action: { label: "Undo", onClick: () => setInvolvement(roomId, name, previous, level) },
        });
      }
    })
    .catch(failed(`Couldn't change notifications to ${involvementChoice(level).label}`));
}

/**
 * An administrator's move of a room in the shared layout, for everyone: into `categoryId`
 * (`null`: no shared category) just before `beforeRoomId`, or at the end. A drop
 * (`leavePersonal`) also takes a room the viewer had starred or put in a category of their own
 * out of there, so it shows where it was dropped; a menu's move leaves those alone. That waits
 * for the shared move to land, and only takes the room out of the place it had at the drop: when
 * the move is refused, or the room has been put somewhere else meanwhile (here or in another
 * tab), it stays. `beforeLeaving` runs just before it leaves (the sidebar keeps focus on the
 * row); resolves once it has, to whether it did.
 */
export async function moveSharedRoom(
  row: SidebarRow,
  categoryId: number | null,
  beforeRoomId: number | null,
  leavePersonal = true,
  beforeLeaving: (() => void) | null = null,
): Promise<boolean> {
  const roomId = row.room.id;
  const dropped = leavePersonal ? personalPlace(roomId) : null;

  beforeChange?.();

  const moved = await actions.workspaceLayout.moveRoom(roomId, categoryId, beforeRoomId).then(
    () => true,
    (error: ActionError) => {
      failed(`Couldn't move ${row.displayName} for everyone`)(error);

      return false;
    },
  );

  if (!moved || !samePlace(dropped, personalPlace(roomId))) {
    return false;
  }

  beforeChange?.();
  beforeLeaving?.();

  actions.organize
    .moveRoom(roomId, { kind: "channels" })
    .catch(failed(`Couldn't move ${row.displayName}`));

  return true;
}

/** A new shared category at the end; resolves to it, or `null` after a refusal (toasted). */
export function createSharedCategory(name: string): Promise<WorkspaceCategory | null> {
  beforeChange?.();

  return actions.workspaceLayout.createCategory(name).catch((error: ActionError) => {
    failed(`Couldn't create "${name.trim()}"`)(error);

    return null;
  });
}

export function renameSharedCategory(category: WorkspaceCategory, name: string): void {
  actions.workspaceLayout
    .renameCategory(category.id, name)
    .catch(failed("Couldn't rename the shared category"));
}

export function deleteSharedCategory(category: WorkspaceCategory): void {
  beforeChange?.();

  actions.workspaceLayout
    .deleteCategory(category.id)
    .catch(failed(`Couldn't delete ${category.name}`));
}

export function reorderSharedCategories(categoryIds: readonly number[]): void {
  beforeChange?.();

  actions.workspaceLayout.reorderCategories(categoryIds).catch((error: ActionError) => {
    if (error.tag === "Conflict") {
      toast({
        title: "The shared categories changed elsewhere",
        description: "This is the latest order. Try the move again.",
      });

      return;
    }

    failed("Couldn't reorder the shared categories")(error);
  });
}

export function markRead(row: SidebarRow): void {
  actions.markRead(row.room.id).catch(failed(`Couldn't mark ${row.displayName} as read`));
}
