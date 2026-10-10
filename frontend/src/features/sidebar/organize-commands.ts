/**
 * The sidebar's organising commands, as the menus, drags and fields call them: each runs the
 * optimistic action and turns a refusal into a toast (the change has already been put back).
 */

import type { Involvement } from "../../gen/Involvement.ts";
import type { RoomCategory, SidebarRow } from "../../store/model.ts";
import type { RoomSlot } from "../../store/organize.ts";
import { mutations, store } from "../../store/store.ts";
import type { ActionError } from "../../sync/run.ts";
import { actions } from "../../sync/runtime.ts";
import { toast } from "../../ui/toast-store.ts";
import { involvementChoice } from "./organize-model.ts";

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
      const preferences = store.getState().sidebar.notificationPreferences;

      if (preferences !== undefined) {
        const roomNotificationLevels = { ...preferences.roomNotificationLevels };
        delete roomNotificationLevels[String(roomId)];
        mutations.setNotificationPreferences({ ...preferences, roomNotificationLevels });
      }

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

export function markRead(row: SidebarRow): void {
  actions.markRead(row.room.id).catch(failed(`Couldn't mark ${row.displayName} as read`));
}
