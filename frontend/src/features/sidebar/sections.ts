import type { WorkspaceCategory } from "../../gen/WorkspaceCategory.ts";
import type { RoomCategory, RoomKind, SidebarRow } from "../../store/model.ts";
import { isNoteToSelf } from "../../store/note-to-self.ts";
import { byFavoritePosition, organizedSidebar } from "../../store/organize.ts";
import type { SidebarState } from "../../store/state.ts";
import { sharedCategories, sharedLayout } from "../../store/workspace-layout.ts";

/** What a section holds, which decides what can be dropped in it. */
export type SidebarSectionKind =
  | "favorites"
  | "category"
  | "shared"
  | "channels"
  | "boards"
  | "voice"
  | "direct";

/** The built-in section each kind of room lives in when nothing else places it. */
const HOME_SECTIONS = {
  open: "channels",
  closed: "channels",
  board: "boards",
  voice: "voice",
  stage: "voice",
  direct: "direct",
} as const satisfies Record<RoomKind, SidebarSectionKind>;

export type HomeSectionKind = (typeof HOME_SECTIONS)[RoomKind];

export function homeSection(row: Pick<SidebarRow, "room">): HomeSectionKind {
  return HOME_SECTIONS[row.room.kind];
}

/** One collapsible group of sidebar rows. */
export interface SidebarSection {
  /**
   * Stable across renders: "favorites", "category-3" (yours), "shared-4" (the workspace's),
   * "channels", "boards", "voice", "direct".
   */
  readonly key: string;
  readonly kind: SidebarSectionKind;
  readonly title: string;
  readonly rows: readonly SidebarRow[];
  /** The user category this section shows, or `null` for the others. */
  readonly category: RoomCategory | null;
  /** The shared workspace category this section shows, or `null` for the others. */
  readonly sharedCategory: WorkspaceCategory | null;
  /** A user category the server remembers as collapsed. */
  readonly collapsed: boolean;
}

export interface SectionOptions {
  /** An administrator sees every shared category, empty ones too, so there is somewhere to drop. */
  readonly admin?: boolean;
  /** The viewer, whose note-to-self leads Direct messages. */
  readonly viewerId?: number | null;
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
  return { key, kind: key, title, rows, category: null, sharedCategory: null, collapsed: false };
}

/** Each home section's title. */
const HOME_TITLES = {
  channels: "Channels",
  boards: "Boards",
  voice: "Voice",
  direct: "Direct messages",
} as const satisfies Record<HomeSectionKind, string>;

/** The home sections in the order the sidebar shows them. */
export const HOME_ORDER: readonly HomeSectionKind[] = ["channels", "boards", "voice", "direct"];

/** A home section with nothing in it, as a drag shows Boards or Voice when hidden for that. */
export function emptyHomeSection(kind: HomeSectionKind): SidebarSection {
  return builtIn(kind, HOME_TITLES[kind], []);
}

function categorySection(category: RoomCategory, rows: readonly SidebarRow[]): SidebarSection {
  return {
    key: `category-${category.id}`,
    kind: "category",
    title: category.name,
    rows,
    category,
    sharedCategory: null,
    collapsed: category.collapsed,
  };
}

function sharedSection(category: WorkspaceCategory, rows: readonly SidebarRow[]): SidebarSection {
  return {
    key: `shared-${category.id}`,
    kind: "shared",
    title: category.name,
    rows,
    category: null,
    sharedCategory: category,
    collapsed: false,
  };
}

/**
 * Groups the sidebar. Personal organisation comes first and wins: Favourites by position, then
 * every user category (empty ones too, so there is somewhere to drop). Every other room follows
 * the workspace's shared layout: the shared categories in the administrators' order, each room in
 * its shared position, then Channels, Boards and Voice for the rooms in no shared category (in
 * their shared order too, rooms never organized after the rest by name), and Direct messages
 * newest first after your note-to-self. A shared category with nothing left to show is dropped, except for administrators
 * (a drag shows the dragged room's own again: `dragSections` in organize-model.ts).
 * Pending organising changes are drawn in, and invisible rooms stay out. Empty Favourites, Boards
 * and Voice are dropped (a drag shows them again where the dragged room can land); Channels and
 * Direct messages always show (they hold the "add" affordances).
 */
export function sidebarSections(
  sidebar: SidebarState,
  { admin = false, viewerId = null }: SectionOptions = {},
): readonly SidebarSection[] {
  const view = organizedSidebar(sidebar);
  const layout = sharedLayout(sidebar);
  const shared = sharedCategories(layout);
  const sharedIds = new Set(shared.map((category) => category.id));
  const places = new Map(layout.rooms.map((place) => [place.roomId, place]));
  const favorites: SidebarRow[] = [];
  const categorized = new Map<number, SidebarRow[]>();
  const placed = new Map<number, SidebarRow[]>();
  const categoryIds = new Set(view.categories.map((category) => category.id));

  const homes: Record<HomeSectionKind, SidebarRow[]> = {
    channels: [],
    boards: [],
    voice: [],
    direct: [],
  };

  for (const roomId of view.order) {
    const row = view.rows[roomId];

    if (row === undefined || row.membership.involvement === "invisible") {
      continue;
    }

    const { membership, room } = row;
    const sharedId = places.get(room.id)?.workspaceCategoryId ?? null;

    if (membership.favoritePosition !== null) {
      favorites.push(row);
    } else if (membership.roomCategoryId !== null && categoryIds.has(membership.roomCategoryId)) {
      const list = categorized.get(membership.roomCategoryId) ?? [];

      list.push(row);
      categorized.set(membership.roomCategoryId, list);
    } else if (room.kind !== "direct" && sharedId !== null && sharedIds.has(sharedId)) {
      const list = placed.get(sharedId) ?? [];

      list.push(row);
      placed.set(sharedId, list);
    } else {
      homes[homeSection(row)].push(row);
    }
  }

  // Shared positions first; rooms never organized keep the server's name order after them.
  const sharedPosition = (row: SidebarRow) =>
    places.get(row.room.id)?.position ?? Number.POSITIVE_INFINITY;

  const bySharedPosition = (left: SidebarRow, right: SidebarRow) => {
    const a = sharedPosition(left);
    const b = sharedPosition(right);

    return a === b ? 0 : a < b ? -1 : 1;
  };

  favorites.sort(byFavoritePosition);
  homes.channels.sort(bySharedPosition);
  homes.boards.sort(bySharedPosition);
  homes.voice.sort(bySharedPosition);
  const self = (row: SidebarRow) => Number(isNoteToSelf(row, viewerId));

  homes.direct.sort((left, right) => self(right) - self(left) || byNewest(left, right));

  const sections: SidebarSection[] = [];

  if (favorites.length > 0) {
    sections.push(builtIn("favorites", "Favourites", favorites));
  }

  for (const category of view.categories) {
    sections.push(categorySection(category, categorized.get(category.id) ?? []));
  }

  for (const category of shared) {
    const rows = (placed.get(category.id) ?? []).sort(bySharedPosition);

    if (admin || rows.length > 0) {
      sections.push(sharedSection(category, rows));
    }
  }

  sections.push(builtIn("channels", HOME_TITLES.channels, homes.channels));

  if (homes.boards.length > 0) {
    sections.push(builtIn("boards", HOME_TITLES.boards, homes.boards));
  }

  if (homes.voice.length > 0) {
    sections.push(builtIn("voice", HOME_TITLES.voice, homes.voice));
  }

  sections.push(builtIn("direct", HOME_TITLES.direct, homes.direct));

  return sections;
}

/**
 * The number on a row's red pill, on every surface (rows, folded categories, the rail, the tab
 * title and the switcher): the unread messages classic would have pushed to you
 * (`notificationCount`, the server's count under `Notifications::Policy#push`). Every unread root
 * message in a room set to "everything" (a DM's default); the mentions and replies to you in a
 * "mentions" room; a muted room's mentions; ping and reply items in threads you follow; nothing
 * for keyword alerts alone, or for a "nothing" room. Plain unread activity shows as a bold name
 * and the left-edge nub instead.
 */
export function rowPillCount(row: SidebarRow): number {
  // The involvement may be a pending change the server hasn't counted for yet: muting leaves the
  // mentions (all a muted room notifies for), and "nothing" or hiding leaves none.
  switch (row.membership.involvement) {
    case "muted":
      // Muted rooms notify for mentions only. When the server already counted it as muted, its
      // count is those mentions; while the change is pending, the old count can't exceed it
      // either way, so the smaller of the two is right or nearly so until the new row lands.
      return Math.min(row.mentionCount, row.notificationCount);
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
 * The rows a folded section still shows below its heading: the open room, and every row that
 * reads as unread, a ping-only one included.
 */
export function peekingRows(
  rows: readonly SidebarRow[],
  selectedRoomId: number | null,
): readonly SidebarRow[] {
  return rows.filter((row) => row.room.id === selectedRoomId || rowUnread(row));
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
