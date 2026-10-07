import type { RoomCategory, SidebarRow } from "../../store/model.ts";
import { byFavoritePosition, organizedSidebar } from "../../store/organize.ts";
import type { SidebarState } from "../../store/state.ts";

/** What a section holds, which decides what can be dropped in it. */
export type SidebarSectionKind = "favorites" | "category" | "channels" | "voice" | "direct";

/** One collapsible group of sidebar rows. */
export interface SidebarSection {
  /** Stable across renders: "favorites", "category-3", "channels", "voice", "direct". */
  readonly key: string;
  readonly kind: SidebarSectionKind;
  readonly title: string;
  readonly rows: readonly SidebarRow[];
  /** The user category this section shows, or `null` for the built-in ones. */
  readonly category: RoomCategory | null;
  /** A user category the server remembers as collapsed. */
  readonly collapsed: boolean;
}

function byNewest(left: SidebarRow, right: SidebarRow): number {
  if (left.room.updatedAt === right.room.updatedAt) {
    return right.room.id - left.room.id;
  }

  return left.room.updatedAt < right.room.updatedAt ? 1 : -1;
}

function builtIn(
  key: SidebarSectionKind,
  title: string,
  rows: readonly SidebarRow[],
): SidebarSection {
  return { key, kind: key, title, rows, category: null, collapsed: false };
}

function categorySection(category: RoomCategory, rows: readonly SidebarRow[]): SidebarSection {
  return {
    key: `category-${category.id}`,
    kind: "category",
    title: category.name,
    rows,
    category,
    collapsed: category.collapsed,
  };
}

/**
 * Groups the sidebar the way the classic one does (`sidebar_in_zone`): Favourites by position,
 * then every user category (empty ones too, so there is somewhere to drop), then Channels, Voice,
 * and Direct messages newest first. Pending organising changes are drawn in, and invisible rooms
 * stay out. Empty Favourites and Voice are dropped; Channels and Direct messages always show
 * (they hold the "add" affordances).
 */
export function sidebarSections(sidebar: SidebarState): readonly SidebarSection[] {
  const view = organizedSidebar(sidebar);
  const favorites: SidebarRow[] = [];
  const channels: SidebarRow[] = [];
  const voice: SidebarRow[] = [];
  const direct: SidebarRow[] = [];
  const categorized = new Map<number, SidebarRow[]>();
  const categoryIds = new Set(view.categories.map((category) => category.id));

  for (const roomId of view.order) {
    const row = view.rows[roomId];

    if (row === undefined || row.membership.involvement === "invisible") {
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
    sections.push(builtIn("favorites", "Favourites", favorites));
  }

  for (const category of view.categories) {
    sections.push(categorySection(category, categorized.get(category.id) ?? []));
  }

  sections.push(builtIn("channels", "Channels", channels));

  if (voice.length > 0) {
    sections.push(builtIn("voice", "Voice", voice));
  }

  sections.push(builtIn("direct", "Direct messages", direct));

  return sections;
}

/**
 * The number on a row's pill: unread mentions, or for a direct message every unread message
 * (each one is addressed to you, as in Slack). A muted room counts its mentions only: the server
 * makes it unread for nothing else (`Room#unread_memberships`).
 */
export function rowPillCount(row: SidebarRow): number {
  if (row.membership.involvement === "muted") {
    return row.mentionCount;
  }

  return row.room.kind === "direct" ? row.unreadCount : row.mentionCount;
}

/**
 * How a row reads: selected, unread (bold), muted (faint), or plain. A muted room a mention made
 * unread reads as unread, as the classic sidebar's `unread muted` row does; it stays dimmed but
 * for its count.
 */
export function rowState(
  row: SidebarRow,
  selected: boolean,
): "selected" | "unread" | "muted" | null {
  if (selected) {
    return "selected";
  }

  if (row.membership.unreadAt !== null) {
    return "unread";
  }

  return row.membership.involvement === "muted" ? "muted" : null;
}

/** Unread rooms and unread mentions across the sidebar, for the rail and the tab title. */
export interface SidebarTotals {
  readonly unreadRooms: number;
  readonly mentions: number;
}

/**
 * Muted rooms count too: one goes unread only when the viewer is mentioned, and then it counts,
 * as the classic app badge counts every `unread` row.
 */
export function sidebarTotals(sidebar: SidebarState): SidebarTotals {
  let unreadRooms = 0;
  let mentions = 0;

  for (const roomId of sidebar.order) {
    const row = sidebar.rows[roomId];

    if (row === undefined) {
      continue;
    }

    if (row.membership.unreadAt !== null) {
      unreadRooms += 1;
    }

    mentions += rowPillCount(row);
  }

  return { unreadRooms, mentions };
}
