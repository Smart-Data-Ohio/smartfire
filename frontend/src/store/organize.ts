/**
 * Sidebar organisation in the store (S3): the viewer's categories, favourites and notification
 * levels. Pure reducers for the category events and membership replies, plus the overlay that
 * shows an organising change at once while the server confirms it.
 */
import type { Involvement } from "../gen/Involvement.ts";
import type { Membership, RoomCategory, SidebarRow } from "./model.ts";
import type { SidebarState, State } from "./state.ts";

/** The organisation fields a pending change can set on a membership. */
export interface MembershipPatch {
  readonly favoritePosition?: number | null;
  readonly roomCategoryId?: number | null;
  readonly involvement?: Involvement;
  /** Muting marks the room read: shown read at once, like the rest of the change. */
  readonly unreadAt?: null;
}

/**
 * What the viewer asked for and the server hasn't confirmed yet, drawn over the rows and
 * categories the server sent. Sync events keep updating the base underneath; when the change
 * settles its entries are dropped, leaving the server's answer (or, after a refusal, the state
 * from before).
 */
export interface SidebarOverlay {
  /** Pending membership fields by room id. */
  readonly memberships: Readonly<Record<number, MembershipPatch>>;
  /** A category as it will be (a new one under a temporary negative id), or `null` while deleted. */
  readonly categories: Readonly<Record<number, RoomCategory | null>>;
}

export const emptyOverlay: SidebarOverlay = { memberships: {}, categories: {} };

/** Where a drag or a menu puts a room. */
export type RoomSlot =
  /** Among the favourites, at this 0-based index (counted without the room itself). */
  | { readonly kind: "favorite"; readonly index: number }
  /** Out of the favourites, into this category. */
  | { readonly kind: "category"; readonly categoryId: number }
  /** Out of the favourites and out of any category: back to Channels. */
  | { readonly kind: "channels" }
  /** Out of the favourites, keeping its category (direct and voice rooms go back to their list). */
  | { readonly kind: "unfavorite" };

/** The server's category limit (`RoomCategory::NAME_LIMIT`). */
export const CATEGORY_NAME_LIMIT = 50;

/** Only open and closed channels can go in a category (the server answers 422 for the rest). */
export function canCategorize(row: SidebarRow): boolean {
  return row.room.kind === "open" || row.room.kind === "closed";
}

/** What "Mute" toggles back to: everything for direct messages, mentions for the rest. */
export function defaultInvolvement(row: SidebarRow): Involvement {
  return row.room.kind === "direct" ? "everything" : "mentions";
}

function byPosition(left: RoomCategory, right: RoomCategory): number {
  return left.position === right.position ? left.id - right.id : left.position - right.position;
}

/** Favourites the way the server orders them: `(favoritePosition, membership.id)`. */
export function byFavoritePosition(left: SidebarRow, right: SidebarRow): number {
  const leftPosition = left.membership.favoritePosition ?? 0;
  const rightPosition = right.membership.favoritePosition ?? 0;

  return leftPosition === rightPosition
    ? left.membership.id - right.membership.id
    : leftPosition - rightPosition;
}

function patchedCategories(
  base: readonly RoomCategory[],
  pending: SidebarOverlay["categories"],
): readonly RoomCategory[] {
  const byId = new Map(base.map((category) => [category.id, category]));

  for (const [id, category] of Object.entries(pending)) {
    if (category === null) {
      byId.delete(Number(id));
    } else {
      byId.set(category.id, category);
    }
  }

  return [...byId.values()].sort(byPosition);
}

function patchedRows(
  base: SidebarState["rows"],
  pending: SidebarOverlay["memberships"],
): SidebarState["rows"] {
  const rows = { ...base };

  for (const [id, patch] of Object.entries(pending)) {
    const row = rows[Number(id)];

    if (row !== undefined) {
      rows[Number(id)] = { ...row, membership: { ...row.membership, ...patch } };
    }
  }

  return rows;
}

const views = new WeakMap<SidebarState, SidebarState>();

/**
 * The sidebar as the viewer should see it: the server's rows and categories with every pending
 * change drawn on top. The same object while nothing is pending; cached per sidebar state.
 */
export function organizedSidebar(sidebar: SidebarState): SidebarState {
  const { memberships, categories } = sidebar.overlay;

  if (Object.keys(memberships).length === 0 && Object.keys(categories).length === 0) {
    return sidebar;
  }

  const cached = views.get(sidebar);

  if (cached !== undefined) {
    return cached;
  }

  const view: SidebarState = {
    ...sidebar,
    rows: patchedRows(sidebar.rows, memberships),
    categories: patchedCategories(sidebar.categories, categories),
  };

  views.set(sidebar, view);

  return view;
}

function withOverlay(state: State, overlay: SidebarOverlay): State {
  return { ...state, sidebar: { ...state.sidebar, overlay } };
}

/** Draws a pending change over the sidebar (a later entry for the same key wins). */
export function addOverlay(state: State, entry: SidebarOverlay): State {
  const { overlay } = state.sidebar;

  return withOverlay(state, {
    memberships: { ...overlay.memberships, ...entry.memberships },
    categories: { ...overlay.categories, ...entry.categories },
  });
}

/** Keeps the keys whose value isn't the one `entry` put there. */
function without<V>(
  current: Readonly<Record<number, V>>,
  entry: Readonly<Record<number, V>>,
): Readonly<Record<number, V>> {
  return Object.fromEntries(
    Object.entries(current).filter(([id, value]) => entry[Number(id)] !== value),
  );
}

/**
 * Drops a settled change's entries. Only the values this entry put there go: a newer change to
 * the same room or category keeps showing.
 */
export function dropOverlay(state: State, entry: SidebarOverlay): State {
  const { overlay } = state.sidebar;

  return withOverlay(state, {
    memberships: without(overlay.memberships, entry.memberships),
    categories: without(overlay.categories, entry.categories),
  });
}

function withCategories(state: State, categories: readonly RoomCategory[]): State {
  return { ...state, sidebar: { ...state.sidebar, categories } };
}

/** A category was created, renamed, folded or moved (`sidebar.category.upserted`, or a reply). */
export function upsertCategory(state: State, category: RoomCategory): State {
  const others = state.sidebar.categories.filter((held) => held.id !== category.id);

  return withCategories(state, [...others, category].sort(byPosition));
}

/**
 * A new category the server has made takes over from its draft in one step, so the sidebar never
 * shows both: the category lands, the draft's entry goes, and `settled` (the room being moved
 * into it, placed under its real id) shows until that move settles too.
 */
export function landCreatedCategory(
  state: State,
  category: RoomCategory,
  draft: SidebarOverlay,
  settled: SidebarOverlay,
): State {
  return addOverlay(dropOverlay(upsertCategory(state, category), draft), settled);
}

/** Every category, after a reorder. */
export function setCategories(state: State, categories: readonly RoomCategory[]): State {
  return withCategories(state, categories.toSorted(byPosition));
}

/**
 * A category was deleted (`sidebar.category.removed`): it leaves, and any row still pointing at
 * it goes back to Channels (the server sends those rows first, so usually none do).
 */
export function removeCategory(state: State, categoryId: number): State {
  const categories = state.sidebar.categories.filter((category) => category.id !== categoryId);
  const rows = { ...state.sidebar.rows };

  for (const row of Object.values(state.sidebar.rows)) {
    if (row.membership.roomCategoryId === categoryId) {
      rows[row.room.id] = { ...row, membership: { ...row.membership, roomCategoryId: null } };
    }
  }

  return { ...state, sidebar: { ...state.sidebar, categories, rows } };
}

/**
 * Lands an organising reply's rows the sidebar already has, taking only what organising changes:
 * the category, the favourite position and the involvement (the room header's copy follows). The
 * reply may be older than a sync event that has since brought newer read state or counts, which
 * stay.
 */
export function mergeOrganization(state: State, replies: readonly SidebarRow[]): State {
  return replies.reduce((current, reply) => {
    const row = current.sidebar.rows[reply.room.id];

    if (row === undefined) {
      return current;
    }

    const { favoritePosition, roomCategoryId, involvement } = reply.membership;
    const membership = { ...row.membership, favoritePosition, roomCategoryId, involvement };

    const next: State = {
      ...current,
      sidebar: {
        ...current.sidebar,
        rows: { ...current.sidebar.rows, [reply.room.id]: { ...row, membership } },
      },
    };

    return setDetailMembership(next, membership);
  }, state);
}

/** The room header's copy of the viewer's membership follows the sidebar's. */
export function setDetailMembership(state: State, membership: Membership): State {
  const room = state.rooms[membership.roomId];

  if (room?.detail === null || room?.detail === undefined) {
    return state;
  }

  return {
    ...state,
    rooms: {
      ...state.rooms,
      [membership.roomId]: { ...room, detail: { ...room.detail, membership } },
    },
  };
}

/**
 * The viewer's membership changed (an involvement reply): the sidebar row, when the room has
 * one, and the room header take it.
 */
export function setMembership(state: State, membership: Membership): State {
  const row = state.sidebar.rows[membership.roomId];

  const next =
    row === undefined
      ? state
      : {
          ...state,
          sidebar: {
            ...state.sidebar,
            rows: { ...state.sidebar.rows, [membership.roomId]: { ...row, membership } },
          },
        };

  return setDetailMembership(next, membership);
}

/** The favourites in order, as `view` shows them. */
export function favoriteRows(view: SidebarState): readonly SidebarRow[] {
  return Object.values(view.rows)
    .filter((row) => row.membership.favoritePosition !== null)
    .sort(byFavoritePosition);
}

/**
 * Where `roomId` sits among the favourites `view` shows, as a favourite slot's index counts it
 * (the favourites ahead of it); -1 when it isn't one.
 */
export function shownFavoriteIndex(view: SidebarState, roomId: number): number {
  return favoriteRows(view).findIndex((row) => row.room.id === roomId);
}

/**
 * The `position` a favourite move sends to put `roomId` at `index` among the other favourites
 * the sidebar shows: just before the one there now, or just after the last.
 *
 * The server counts every favourite, hidden (`invisible`) ones included, which the client has
 * no rows for (`Membership#move_favorite_to`). A move renumbers every favourite 0, 1, …, so a
 * hidden one shows as a gap in the shown positions, and the shown position of the favourite to
 * land before is its place in the full list. Until the next move an unstarred favourite leaves a
 * gap too, which this reads as a hidden one: the room then lands a little late, and the caller
 * moves it again from the renumbered positions the reply brings.
 */
export function serverFavoritePosition(view: SidebarState, roomId: number, index: number): number {
  const own = view.rows[roomId]?.membership.favoritePosition ?? Number.POSITIVE_INFINITY;
  const others = favoriteRows(view).filter((row) => row.room.id !== roomId);
  const anchor = others[Math.min(Math.max(index, 0), others.length)];
  const last = others.at(-1);

  // The anchor's place among every other favourite: its position, less this room's own place
  // when this room sits ahead of it.
  const placeOf = (row: SidebarRow) => {
    const position = row.membership.favoritePosition ?? 0;

    return own < position ? position - 1 : position;
  };

  if (anchor !== undefined) {
    return placeOf(anchor);
  }

  return last === undefined ? 0 : placeOf(last) + 1;
}

/**
 * The membership fields that put `roomId` in `slot`, as the server will leave them. A favourite
 * move renumbers every favourite 0, 1, … (`Membership#move_favorite_to`), so it touches them all.
 */
export function placementPatches(
  view: SidebarState,
  roomId: number,
  slot: RoomSlot,
): SidebarOverlay["memberships"] {
  switch (slot.kind) {
    case "favorite": {
      const others = favoriteRows(view)
        .map((row) => row.room.id)
        .filter((id) => id !== roomId);

      const index = Math.min(Math.max(slot.index, 0), others.length);
      const order = [...others.slice(0, index), roomId, ...others.slice(index)];

      return Object.fromEntries(order.map((id, position) => [id, { favoritePosition: position }]));
    }

    case "category":
      return patchFor(roomId, { favoritePosition: null, roomCategoryId: slot.categoryId });
    case "channels":
      return patchFor(roomId, { favoritePosition: null, roomCategoryId: null });
    case "unfavorite":
      return patchFor(roomId, { favoritePosition: null });
  }
}

/** A single room's patch, as the overlay keys it. */
function patchFor(roomId: number, patch: MembershipPatch): SidebarOverlay["memberships"] {
  return Object.fromEntries([[roomId, patch]]);
}

/** Whether `row` already sits in `slot` (a drop there changes nothing). */
export function isInSlot(view: SidebarState, row: SidebarRow, slot: RoomSlot): boolean {
  const { favoritePosition, roomCategoryId } = row.membership;

  switch (slot.kind) {
    case "favorite":
      return favoritePosition !== null && shownFavoriteIndex(view, row.room.id) === slot.index;
    case "category":
      return favoritePosition === null && roomCategoryId === slot.categoryId;
    case "channels":
      return favoritePosition === null && (roomCategoryId === null || !canCategorize(row));
    case "unfavorite":
      return favoritePosition === null;
  }
}
