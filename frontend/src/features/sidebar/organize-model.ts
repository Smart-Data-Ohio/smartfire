/**
 * The sidebar's organising rules as plain data: what can be dragged where, the ordered drop
 * targets a keyboard drag steps through, how they read aloud, and the notification levels the
 * menus offer. No React here, so the rules are tested on their own.
 */
import type { Involvement } from "../../gen/Involvement.ts";
import type { RoomCategory, SidebarRow } from "../../store/model.ts";
import { canCategorize, type RoomSlot } from "../../store/organize.ts";
import type { IconName } from "../../ui/icons/icon.tsx";
import type { SidebarSection } from "./sections.ts";

/** What is being dragged: a conversation, or a category by its heading. */
export type DragItem =
  | { readonly kind: "room"; readonly row: SidebarRow }
  | { readonly kind: "category"; readonly category: RoomCategory };

/** Where it would land: a room slot, or a 0-based place among the other categories. */
export type DropTarget =
  | { readonly kind: "room"; readonly slot: RoomSlot }
  | { readonly kind: "category"; readonly index: number };

/** A drop target with what a screen reader hears for it. */
export interface LabelledTarget {
  readonly target: DropTarget;
  readonly label: string;
}

/** The empty Favourites section shown while a room is dragged, so there is somewhere to star it. */
export const FAVORITES_PLACEHOLDER: SidebarSection = {
  key: "favorites",
  kind: "favorites",
  title: "Favourites",
  rows: [],
  category: null,
  collapsed: false,
};

/** The section a room lives in when it isn't a favourite or in a category. */
function homeKind(row: SidebarRow): SidebarSection["kind"] {
  switch (row.room.kind) {
    case "direct":
      return "direct";
    case "voice":
      return "voice";
    default:
      return "channels";
  }
}

/**
 * The slot a room dropped on `section` goes to, or `null` when it can't go there: categories and
 * Channels take open and closed channels only, and a direct or voice room leaves the favourites
 * by going back to its own list. `favoriteIndex` is where among the favourites it was dropped.
 */
export function slotForSection(
  section: SidebarSection,
  row: SidebarRow,
  favoriteIndex: number,
): RoomSlot | null {
  const favorite = row.membership.favoritePosition !== null;

  switch (section.kind) {
    case "favorites":
      return { kind: "favorite", index: favoriteIndex };
    case "category":
      return section.category !== null && canCategorize(row)
        ? { kind: "category", categoryId: section.category.id }
        : null;
    case "channels":
      if (canCategorize(row)) {
        return { kind: "channels" };
      }

      return favorite && homeKind(row) === "channels" ? { kind: "unfavorite" } : null;
    case "voice":
    case "direct":
      return favorite && homeKind(row) === section.kind ? { kind: "unfavorite" } : null;
  }
}

/** Whether anything about `row` can land in `section` (a section that can't is dimmed). */
export function acceptsRoom(section: SidebarSection, row: SidebarRow): boolean {
  return slotForSection(section, row, 0) !== null;
}

/** How many of `midpoints` sit above `y`: the insertion index for a pointer at `y`. */
export function insertionIndex(midpoints: readonly number[], y: number): number {
  return midpoints.filter((midpoint) => midpoint < y).length;
}

/**
 * Every place a keyboard drag can take `row`, top to bottom: each gap among the favourites, then
 * each section that takes it. Favourites come first even when there are none yet.
 */
export function roomTargets(
  sections: readonly SidebarSection[],
  row: SidebarRow,
): readonly LabelledTarget[] {
  const withFavorites = sections.some((section) => section.kind === "favorites")
    ? sections
    : [FAVORITES_PLACEHOLDER, ...sections];

  return withFavorites.flatMap((section): LabelledTarget[] => {
    if (section.kind === "favorites") {
      const others = section.rows.filter((candidate) => candidate.room.id !== row.room.id);

      return Array.from({ length: others.length + 1 }, (_, index) => ({
        target: { kind: "room", slot: { kind: "favorite", index } },
        label:
          index === 0
            ? "Top of Favourites"
            : `Favourites, after ${others[index - 1]?.displayName ?? "the last one"}`,
      }));
    }

    const slot = slotForSection(section, row, 0);

    if (slot === null) {
      return [];
    }

    return [
      {
        target: { kind: "room", slot },
        label: slot.kind === "unfavorite" ? `${section.title}, out of Favourites` : section.title,
      },
    ];
  });
}

/** Every place among the categories a keyboard drag can take `category`. */
export function categoryTargets(
  categories: readonly RoomCategory[],
  category: RoomCategory,
): readonly LabelledTarget[] {
  const others = categories.filter((candidate) => candidate.id !== category.id);

  return Array.from({ length: others.length + 1 }, (_, index) => ({
    target: { kind: "category", index },
    label:
      index === 0
        ? "First category"
        : `After ${others[index - 1]?.name ?? "the last category"}, position ${index + 1} of ${
            others.length + 1
          }`,
  }));
}

/** The category ids with `categoryId` moved to `index` among the others. */
export function reorderedIds(
  categories: readonly RoomCategory[],
  categoryId: number,
  index: number,
): readonly number[] {
  const others = categories.flatMap((category) =>
    category.id === categoryId ? [] : [category.id],
  );

  const at = Math.min(Math.max(index, 0), others.length);

  return [...others.slice(0, at), categoryId, ...others.slice(at)];
}

/** The index a category sits at now, as a category drop target counts it. */
export function categoryIndex(categories: readonly RoomCategory[], categoryId: number): number {
  return categories.findIndex((category) => category.id === categoryId);
}

/** Whether two drop targets are the same place (both plain data, so their JSON compares). */
export function sameTarget(left: DropTarget | null, right: DropTarget | null): boolean {
  return JSON.stringify(left) === JSON.stringify(right);
}

/** What a notification level says in the menus, and its icon. */
export interface InvolvementChoice {
  readonly level: Involvement;
  readonly label: string;
  readonly description: string;
  readonly icon: IconName;
}

const CHOICES: Readonly<Record<Involvement, InvolvementChoice>> = {
  everything: {
    level: "everything",
    label: "All messages",
    description: "Notify me about every new message",
    icon: "bell-ring",
  },
  mentions: {
    level: "mentions",
    label: "Mentions",
    description: "Notify me only when I'm mentioned",
    icon: "at",
  },
  nothing: {
    level: "nothing",
    label: "No notifications",
    description: "New messages still show as unread",
    icon: "bell",
  },
  muted: {
    level: "muted",
    label: "Muted",
    description: "Dimmed and quiet; mentions still count",
    icon: "bell-off",
  },
  invisible: {
    level: "invisible",
    label: "Hidden",
    description: "Out of the sidebar; Jump to still finds it",
    icon: "eye-off",
  },
};

export function involvementChoice(level: Involvement): InvolvementChoice {
  return CHOICES[level];
}

/**
 * The levels a room offers, in the classic bell's order: a direct message can't be hidden or
 * limited to mentions (every message in it is for you).
 */
export function involvementChoices(row: Pick<SidebarRow, "room">): readonly InvolvementChoice[] {
  return row.room.kind === "direct"
    ? [CHOICES.everything, CHOICES.nothing, CHOICES.muted]
    : [CHOICES.everything, CHOICES.mentions, CHOICES.nothing, CHOICES.muted, CHOICES.invisible];
}
