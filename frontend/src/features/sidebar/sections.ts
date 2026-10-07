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
    } else if (room.kind === "voice" || room.kind === "stage") {
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
 * The number on a row's red pill: the unread messages that would have notified you under the
 * classic rules (`notificationCount`, the server's `Notifications::Policy` count). Every unread
 * root message in a room set to "everything" (a DM's default), the mentions, replies, thread
 * activity and keyword alerts in a "mentions" room, a muted room's mentions, nothing for
 * "nothing". Plain unread activity shows as a bold name and the left-edge nub instead. The
 * rail, the tab title and the switcher all count with this, so every surface agrees.
 */
export function rowPillCount(row: SidebarRow): number {
  // The involvement may be a pending change the server hasn't counted for yet: muting leaves the
  // mentions (all a muted room notifies for), and "nothing" or hiding leaves none.
  switch (row.membership.involvement) {
    case "muted":
      // The server's muted count is these same mentions (one item per message).
      return row.mentionCount;
    case "nothing":
    case "invisible":
      return 0;
    default:
      return row.notificationCount;
  }
}

/**
 * Whether a row reads as unread: the membership is unread, or something here would have notified
 * you (a red count). A thread ping leaves the room's root timeline read, but it still bolds the
 * row, since a count beside a plain name would say two things at once.
 */
export function rowUnread(row: SidebarRow): boolean {
  return row.membership.unreadAt !== null || rowPillCount(row) > 0;
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

  if (rowUnread(row)) {
    return "unread";
  }

  return row.membership.involvement === "muted" ? "muted" : null;
}

/** What a screen reader hears for a red pill of `count`. */
export function notificationLabel(count: number): string {
  return count === 1 ? "1 notification" : `${count} notifications`;
}

/** A folded section's summary: whether any room in it is unread, and its notifications. */
export interface SectionUnread {
  readonly unread: boolean;
  readonly count: number;
}

export function sectionUnread(rows: readonly SidebarRow[]): SectionUnread {
  let unread = false;
  let count = 0;

  for (const row of rows) {
    unread ||= rowUnread(row);
    count += rowPillCount(row);
  }

  return { unread, count };
}

/**
 * What a folded category's trigger says about the rows it hides: "Unread", "Unread, 2
 * notifications", or nothing when every hidden row is read.
 */
export function sectionStatus({ unread, count }: SectionUnread): string | null {
  if (count > 0) {
    return unread ? `Unread, ${notificationLabel(count)}` : notificationLabel(count);
  }

  return unread ? "Unread" : null;
}

/** Unread rooms and unread mentions across the sidebar, for the rail and the tab title. */
export interface SidebarTotals {
  readonly unreadRooms: number;
  readonly mentions: number;
}

/**
 * Read through the pending organising changes, so muting or hiding a room updates the rail and
 * the tab title at once. Muted rooms count too: one goes unread only when the viewer is
 * mentioned, and then it counts, as the classic app badge counts every `unread` row.
 */
export function sidebarTotals(sidebar: SidebarState): SidebarTotals {
  const view = organizedSidebar(sidebar);
  let unreadRooms = 0;
  let mentions = 0;

  for (const roomId of view.order) {
    const row = view.rows[roomId];

    if (row === undefined || row.membership.involvement === "invisible") {
      continue;
    }

    if (rowUnread(row)) {
      unreadRooms += 1;
    }

    mentions += rowPillCount(row);
  }

  return { unreadRooms, mentions };
}
