/**
 * The sidebar's organising rules as plain data: what can be dragged where, the ordered drop
 * targets a keyboard drag steps through, how they read aloud, and the notification levels the
 * menus offer. No React here, so the rules are tested on their own.
 */
import type { Involvement } from "../../gen/Involvement.ts";
import type { WorkspaceCategory } from "../../gen/WorkspaceCategory.ts";
import type { RoomCategory, SidebarRow } from "../../store/model.ts";
import { canCategorize, type RoomSlot } from "../../store/organize.ts";
import type { IconName } from "../../ui/icons/icon.tsx";
import type { SidebarSection } from "./sections.ts";

/** What is being dragged: a conversation, or a category (yours or a shared one) by its heading. */
export type DragItem =
  | { readonly kind: "room"; readonly row: SidebarRow }
  | { readonly kind: "category"; readonly category: RoomCategory }
  | { readonly kind: "shared-category"; readonly category: WorkspaceCategory };

/**
 * Where it would land: a slot in the viewer's own organisation, a place in the shared layout
 * (just before a room, or at the end of the group with `null`), or a 0-based place among the
 * other categories of the dragged one's kind.
 */
export type DropTarget =
  | { readonly kind: "room"; readonly slot: RoomSlot }
  | {
      readonly kind: "shared";
      readonly categoryId: number | null;
      readonly beforeRoomId: number | null;
    }
  | { readonly kind: "category"; readonly index: number }
  | { readonly kind: "shared-category"; readonly index: number };

/** A drop target with what a screen reader hears for it. */
export interface LabelledTarget {
  readonly target: DropTarget;
  readonly label: string;
}

/**
 * Who is dropping, and the shared layout under the drop. An administrator's drop on the shared
 * hierarchy (a shared category, Channels or Voice) moves the room there for everyone; anyone
 * else's drops change only their own organisation.
 */
export interface DropContext {
  readonly admin: boolean;
  /** The shared category a room falls back to without personal organisation (`null`: none). */
  readonly sharedHome: (roomId: number) => number | null;
}

export const PERSONAL_DROPS: DropContext = { admin: false, sharedHome: () => null };

/** The empty Favourites section shown while a room is dragged, so there is somewhere to star it. */
export const FAVORITES_PLACEHOLDER: SidebarSection = {
  key: "favorites",
  kind: "favorites",
  title: "Favourites",
  rows: [],
  category: null,
  sharedCategory: null,
  collapsed: false,
};

/** The section a room lives in when it isn't a favourite or in a category. */
function homeKind(row: SidebarRow): SidebarSection["kind"] {
  switch (row.room.kind) {
    case "direct":
      return "direct";
    case "voice":
    case "stage":
      return "voice";
    default:
      return "channels";
  }
}

/** Whether the viewer's own organisation holds the room: a favourite, or one of their categories. */
export function personallyPlaced(row: SidebarRow): boolean {
  return (
    row.membership.favoritePosition !== null ||
    (row.membership.roomCategoryId !== null && canCategorize(row))
  );
}

/**
 * The slot a room dropped on `section` goes to in the viewer's own organisation, or `null` when
 * it can't go there. Categories take open and closed channels only. Leaving Favourites or a
 * category puts the room back where the shared layout has it (`home`, its shared category): a
 * drop there or, without one, on its own list (Channels, Voice or Direct messages) does that.
 * `favoriteIndex` is where among the favourites it was dropped.
 */
export function slotForSection(
  section: SidebarSection,
  row: SidebarRow,
  favoriteIndex: number,
  home: number | null = null,
): RoomSlot | null {
  const favorite = row.membership.favoritePosition !== null;
  const ownList = home === null && homeKind(row) === section.kind;

  switch (section.kind) {
    case "favorites":
      return { kind: "favorite", index: favoriteIndex };
    case "category":
      return section.category !== null && canCategorize(row)
        ? { kind: "category", categoryId: section.category.id }
        : null;
    case "shared":
      return home !== null && section.sharedCategory?.id === home ? { kind: "channels" } : null;
    case "channels":
      if (canCategorize(row)) {
        return home === null ? { kind: "channels" } : null;
      }

      return favorite && ownList ? { kind: "unfavorite" } : null;
    case "voice":
    case "direct":
      return favorite && ownList ? { kind: "unfavorite" } : null;
  }
}

/**
 * The shared group an administrator's drop of `row` on `section` moves it into (`null`: no
 * shared category), or `undefined` when the section isn't part of the shared layout for it. A
 * shared category takes any room but a direct message; Channels takes channels, Voice voice and
 * stage rooms.
 */
export function sharedGroupFor(
  section: SidebarSection,
  row: SidebarRow,
): number | null | undefined {
  switch (section.kind) {
    case "shared":
      return row.room.kind === "direct" ? undefined : (section.sharedCategory?.id ?? undefined);
    case "channels":
      return canCategorize(row) ? null : undefined;
    case "voice":
      return row.room.kind === "voice" || row.room.kind === "stage" ? null : undefined;
    default:
      return undefined;
  }
}

/**
 * Where a room dropped on `section` lands, or `null` when it can't go there. `index` is the
 * 0-based place among the section's other rows: among the favourites, or, for an
 * administrator's drop on the shared layout, the room it goes before.
 */
export function targetForSection(
  section: SidebarSection,
  row: SidebarRow,
  index: number,
  context: DropContext = PERSONAL_DROPS,
): DropTarget | null {
  const group = context.admin ? sharedGroupFor(section, row) : undefined;

  if (group !== undefined) {
    const others = section.rows.filter((candidate) => candidate.room.id !== row.room.id);

    return { kind: "shared", categoryId: group, beforeRoomId: others[index]?.room.id ?? null };
  }

  const slot = slotForSection(section, row, index, context.sharedHome(row.room.id));

  return slot === null ? null : { kind: "room", slot };
}

/** Whether anything about `row` can land in `section` (a section that can't is dimmed). */
export function acceptsRoom(
  section: SidebarSection,
  row: SidebarRow,
  context: DropContext = PERSONAL_DROPS,
): boolean {
  return targetForSection(section, row, 0, context) !== null;
}

/** How many of `midpoints` sit above `y`: the insertion index for a pointer at `y`. */
export function insertionIndex(midpoints: readonly number[], y: number): number {
  return midpoints.filter((midpoint) => midpoint < y).length;
}

/** One target per gap among `others`: "Top of Favourites", "Favourites, after #general", … */
function gaps(
  title: string,
  others: readonly SidebarRow[],
  target: (index: number) => DropTarget,
): LabelledTarget[] {
  return Array.from({ length: others.length + 1 }, (_, index) => ({
    target: target(index),
    label:
      index === 0
        ? `Top of ${title}`
        : `${title}, after ${others[index - 1]?.displayName ?? "the last one"}`,
  }));
}

/**
 * Every place a keyboard drag can take `row`, top to bottom: each gap among the favourites, then
 * each section that takes it, with a gap per room in the shared layout's sections for an
 * administrator. Favourites come first even when there are none yet.
 */
export function roomTargets(
  sections: readonly SidebarSection[],
  row: SidebarRow,
  context: DropContext = PERSONAL_DROPS,
): readonly LabelledTarget[] {
  const withFavorites = sections.some((section) => section.kind === "favorites")
    ? sections
    : [FAVORITES_PLACEHOLDER, ...sections];

  return withFavorites.flatMap((section): LabelledTarget[] => {
    const others = section.rows.filter((candidate) => candidate.room.id !== row.room.id);

    if (section.kind === "favorites") {
      return gaps("Favourites", others, (index) => ({
        kind: "room",
        slot: { kind: "favorite", index },
      }));
    }

    const target = targetForSection(section, row, 0, context);

    if (target === null) {
      return [];
    }

    if (target.kind === "shared") {
      return gaps(section.title, others, (index) => ({
        ...target,
        beforeRoomId: others[index]?.room.id ?? null,
      })).map(({ target: gap, label }) => ({ target: gap, label: `${label}, for everyone` }));
    }

    return [
      {
        target,
        label:
          target.kind === "room" && target.slot.kind === "unfavorite"
            ? `${section.title}, out of Favourites`
            : section.title,
      },
    ];
  });
}

/** A category of either kind, as reordering sees it. */
interface Named {
  readonly id: number;
  readonly name: string;
}

/** Every place among the categories (`categories`, of one kind) a keyboard drag can take one. */
export function categoryTargets(
  categories: readonly Named[],
  category: Named,
  kind: "category" | "shared-category" = "category",
): readonly LabelledTarget[] {
  const others = categories.filter((candidate) => candidate.id !== category.id);

  return Array.from({ length: others.length + 1 }, (_, index) => ({
    target: { kind, index },
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
  categories: readonly Named[],
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
export function categoryIndex(categories: readonly Named[], categoryId: number): number {
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
  return levelChoices(row.room.kind === "direct");
}

/** The levels a direct message (`direct`) or another room offers, in the classic bell's order. */
export function levelChoices(direct: boolean): readonly InvolvementChoice[] {
  return direct
    ? [CHOICES.everything, CHOICES.nothing, CHOICES.muted]
    : [CHOICES.everything, CHOICES.mentions, CHOICES.nothing, CHOICES.muted, CHOICES.invisible];
}
