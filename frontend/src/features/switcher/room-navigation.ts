/**
 * Alt+↑/↓ and Alt+Shift+↑/↓: step through conversations in the order the sidebar shows them
 * (Favourites, categories, Channels, Voice, Direct messages), wrapping at the ends.
 */
import type { SidebarRow } from "../../store/model.ts";
import type { SidebarState } from "../../store/state.ts";
import { type SidebarSection, sidebarSections } from "../sidebar/sections.ts";

export type Direction = 1 | -1;

/** The sidebar's rows top to bottom, optionally only some sections (the DMs destination). */
export function sidebarOrder(
  sidebar: SidebarState,
  include: (section: SidebarSection) => boolean = () => true,
): readonly SidebarRow[] {
  return sidebarSections(sidebar)
    .filter(include)
    .flatMap((section) => section.rows);
}

/** Unread and not muted: what "next unread" stops at. */
export function isUnreadStop(row: SidebarRow): boolean {
  return row.membership.unreadAt !== null && row.membership.involvement !== "muted";
}

/**
 * The room `direction` steps from `currentId` (wrapping) whose row passes `accept`; from no
 * room, ↓ starts at the top and ↑ at the bottom. `null` when no other row qualifies.
 */
export function adjacentRoom(
  rows: readonly SidebarRow[],
  currentId: number | null,
  direction: Direction,
  accept: (row: SidebarRow) => boolean = () => true,
): number | null {
  const count = rows.length;
  const index = currentId === null ? -1 : rows.findIndex((row) => row.room.id === currentId);
  const start = index === -1 ? (direction === 1 ? -1 : count) : index;

  for (let step = 1; step <= count; step += 1) {
    const row = rows[(((start + step * direction) % count) + count) % count];

    if (row !== undefined && row.room.id !== currentId && accept(row)) {
      return row.room.id;
    }
  }

  return null;
}
