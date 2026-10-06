import type { RoomCategory, SidebarRow } from "../../store/model.ts";
import type { SidebarState } from "../../store/state.ts";

/** One collapsible group of sidebar rows. */
export interface SidebarSection {
  /** Stable across renders: "favorites", "category-3", "channels", "voice", "direct". */
  readonly key: string;
  readonly title: string;
  readonly rows: readonly SidebarRow[];
  /** A user category the server remembers as collapsed. */
  readonly collapsed: boolean;
}

function byFavoritePosition(left: SidebarRow, right: SidebarRow): number {
  const leftPosition = left.membership.favoritePosition ?? 0;
  const rightPosition = right.membership.favoritePosition ?? 0;

  return leftPosition === rightPosition
    ? left.membership.id - right.membership.id
    : leftPosition - rightPosition;
}

function byNewest(left: SidebarRow, right: SidebarRow): number {
  if (left.room.updatedAt === right.room.updatedAt) {
    return right.room.id - left.room.id;
  }

  return left.room.updatedAt < right.room.updatedAt ? 1 : -1;
}

function categorySection(category: RoomCategory, rows: readonly SidebarRow[]): SidebarSection {
  return {
    key: `category-${category.id}`,
    title: category.name,
    rows,
    collapsed: category.collapsed,
  };
}

/**
 * Groups the sidebar the way the classic one does (`sidebar_in_zone`): Favourites by position,
 * then each user category, then Channels, Voice, and Direct messages newest first. Empty
 * sections are dropped, except Channels and Direct messages, which always show (they hold the
 * "add" affordances).
 */
export function sidebarSections(sidebar: SidebarState): readonly SidebarSection[] {
  const favorites: SidebarRow[] = [];
  const channels: SidebarRow[] = [];
  const voice: SidebarRow[] = [];
  const direct: SidebarRow[] = [];
  const categorized = new Map<number, SidebarRow[]>();
  const categoryIds = new Set(sidebar.categories.map((category) => category.id));

  for (const roomId of sidebar.order) {
    const row = sidebar.rows[roomId];

    if (row === undefined) {
      continue;
    }

    const { membership, room } = row;

    if (membership.favoritePosition !== null) {
      favorites.push(row);
    } else if (membership.roomCategoryId !== null && categoryIds.has(membership.roomCategoryId)) {
      const list = categorized.get(membership.roomCategoryId) ?? [];

      list.push(row);
      categorized.set(membership.roomCategoryId, list);
    } else if (room.kind === "direct") {
      direct.push(row);
    } else if (room.kind === "voice") {
      voice.push(row);
    } else {
      channels.push(row);
    }
  }

  favorites.sort(byFavoritePosition);
  direct.sort(byNewest);

  const sections: SidebarSection[] = [];

  if (favorites.length > 0) {
    sections.push({ key: "favorites", title: "Favourites", rows: favorites, collapsed: false });
  }

  for (const category of sidebar.categories) {
    const rows = categorized.get(category.id);

    if (rows !== undefined) {
      sections.push(categorySection(category, rows));
    }
  }

  sections.push({ key: "channels", title: "Channels", rows: channels, collapsed: false });

  if (voice.length > 0) {
    sections.push({ key: "voice", title: "Voice", rows: voice, collapsed: false });
  }

  sections.push({ key: "direct", title: "Direct messages", rows: direct, collapsed: false });

  return sections;
}

/**
 * The number on a row's pill: unread mentions, or for a direct message every unread message
 * (each one is addressed to you, as in Slack).
 */
export function rowPillCount(row: SidebarRow): number {
  return row.room.kind === "direct" ? row.unreadCount : row.mentionCount;
}

/** Unread rooms and unread mentions across the sidebar, for the rail and the tab title. */
export interface SidebarTotals {
  readonly unreadRooms: number;
  readonly mentions: number;
}

export function sidebarTotals(sidebar: SidebarState): SidebarTotals {
  let unreadRooms = 0;
  let mentions = 0;

  for (const roomId of sidebar.order) {
    const row = sidebar.rows[roomId];

    if (row === undefined || row.membership.involvement === "muted") {
      continue;
    }

    if (row.membership.unreadAt !== null) {
      unreadRooms += 1;
    }

    mentions += rowPillCount(row);
  }

  return { unreadRooms, mentions };
}
