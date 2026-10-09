/**
 * Which HTTP replies are older than the sidebar rows they would install, on a client clock.
 *
 * Sidebar rows arrive two ways: sync events, which the server publishes after every write that
 * changes a row, in order; and HTTP replies (the whole sidebar, an organising reply, a join, a
 * 404), which are snapshots taken when the server answered. A reply can be older than what sync
 * landed while it was in flight, so every HTTP request that installs or removes rows holds a
 * ticket (the clock when it began) from before the request until its reply has landed or failed.
 *
 * Two things make a reply older than the store:
 * - a sync resync (a gap, a rollover, a welcome that couldn't resume) landed after its ticket:
 *   the resync is authoritative, so the whole reply is stale;
 * - the sync path (or a read made here) touched a room after its ticket: that room keeps the
 *   store's row. Touches are kept only while a request older than them is in flight (the store
 *   tracks the tickets held, and prunes after every change).
 *
 * Preferring sync converges: whatever the reply knew that sync didn't, the server publishes again.
 */
import type { SidebarRow, SyncEvent } from "./model.ts";
import type { State } from "./state.ts";

type Touches = Readonly<Record<number, number>>;

export interface RowTouches {
  readonly clock: number;
  /** The clock when the latest sync resync landed: a ticket from before it is stale. */
  readonly resyncEpoch: number;
  /**
   * The clock at each room's latest touch, kept after its row leaves, only while a request
   * older than the touch is in flight (`pruneTouches`).
   */
  readonly at: Touches;
  /** The same for sidebar categories, by category id. */
  readonly categories: Touches;
  /**
   * The clock when a mark-unread reply last moved each room's divider: an older one still on its
   * way doesn't move it back. Pruned as the touches are.
   */
  readonly dividers: Touches;
  /**
   * The clock at each room's latest sync `room.read`: a mark-unread reply from before it neither
   * moves the divider nor counts the row. (The viewer's own `room.unread` echo isn't one.)
   */
  readonly reads: Touches;
}

export const noRowTouches: RowTouches = {
  clock: 0,
  resyncEpoch: 0,
  at: {},
  categories: {},
  dividers: {},
  reads: {},
};

/** The clock now: the ticket of a request beginning now. */
export function rowClock(state: State): number {
  return state.rowTouches.clock;
}

/** Whether a sync resync landed after the request holding `since` began: its reply is stale. */
export function isStale(state: State, since: number): boolean {
  return since < state.rowTouches.resyncEpoch;
}

/** A sync resync landed: every reply to a request already in flight is stale. */
export function markResynced(state: State): State {
  const clock = state.rowTouches.clock + 1;

  return { ...state, rowTouches: { ...state.rowTouches, clock, resyncEpoch: clock } };
}

/** Whether the sync path changed `roomId`'s row after `since` (a ticket still held). */
export function touchedSince(state: State, roomId: number, since: number): boolean {
  return (state.rowTouches.at[roomId] ?? 0) > since;
}

/** Whether the sync path changed (or removed) category `categoryId` after `since`. */
export function categoryTouchedSince(state: State, categoryId: number, since: number): boolean {
  return (state.rowTouches.categories[categoryId] ?? 0) > since;
}

/** Whether a sync `room.read` for `roomId` landed after `since`. */
export function readSince(state: State, roomId: number, since: number): boolean {
  return (state.rowTouches.reads[roomId] ?? 0) > since;
}

/** Whether a newer mark-unread reply moved `roomId`'s divider after `since`. */
export function dividerMovedSince(state: State, roomId: number, since: number): boolean {
  return (state.rowTouches.dividers[roomId] ?? 0) > since;
}

function stamped(touches: Touches, ids: Iterable<number>, clock: number): Touches {
  let next: Record<number, number> | null = null;

  for (const id of ids) {
    next ??= { ...touches };
    next[id] = clock;
  }

  return next ?? touches;
}

/**
 * Records that the sync path (or a read made here) just changed these rooms' rows and these
 * categories, and that sync said these rooms were read.
 */
export function touchRows(
  state: State,
  roomIds: Iterable<number>,
  categoryIds: Iterable<number> = [],
  readRoomIds: Iterable<number> = [],
): State {
  const touches = state.rowTouches;
  const clock = touches.clock + 1;
  const at = stamped(touches.at, roomIds, clock);
  const categories = stamped(touches.categories, categoryIds, clock);
  const reads = stamped(touches.reads, readRoomIds, clock);

  if (at === touches.at && categories === touches.categories && reads === touches.reads) {
    return state;
  }

  return { ...state, rowTouches: { ...touches, clock, at, categories, reads } };
}

/** A mark-unread reply just moved `roomId`'s divider. */
export function claimDivider(state: State, roomId: number): State {
  const touches = state.rowTouches;
  const clock = touches.clock + 1;

  return {
    ...state,
    rowTouches: { ...touches, clock, dividers: stamped(touches.dividers, [roomId], clock) },
  };
}

function pruned(touches: Touches, oldest: number | undefined): Touches {
  const entries = Object.entries(touches);
  const kept = entries.filter(([, clock]) => oldest !== undefined && clock > oldest);

  return kept.length === entries.length ? touches : Object.fromEntries(kept);
}

/**
 * Forgets the touches no request in flight can be older than: those at or before `oldest`, the
 * oldest ticket still held, or every one when none is (`undefined`). The same state when nothing
 * goes.
 */
export function pruneTouches(state: State, oldest: number | undefined): State {
  const touches = state.rowTouches;
  const at = pruned(touches.at, oldest);
  const categories = pruned(touches.categories, oldest);
  const dividers = pruned(touches.dividers, oldest);
  const reads = pruned(touches.reads, oldest);

  if (
    at === touches.at &&
    categories === touches.categories &&
    dividers === touches.dividers &&
    reads === touches.reads
  ) {
    return state;
  }

  return { ...state, rowTouches: { ...touches, at, categories, dividers, reads } };
}

/**
 * Whether an HTTP 404 (the request holding `since`) is older than the store's row for `roomId`:
 * a row a resync installed, or sync (or a read here) touched, after the request began stays. With
 * no row, the store already agrees the room is gone. Only the row: the room's own unavailable
 * outcome is the room request's to settle.
 */
export function removalIsStale(state: State, roomId: number, since: number): boolean {
  return (
    state.sidebar.rows[roomId] !== undefined &&
    (isStale(state, since) || touchedSince(state, roomId, since))
  );
}

/** Rooms whose row is a different object (changed, added or removed) between two states. */
export function changedRowIds(
  before: Readonly<Record<number, SidebarRow>>,
  after: Readonly<Record<number, SidebarRow>>,
): readonly number[] {
  if (before === after) {
    return [];
  }

  const ids = new Set<number>();

  for (const key of Object.keys(before)) {
    const roomId = Number(key);

    if (before[roomId] !== after[roomId]) ids.add(roomId);
  }

  for (const key of Object.keys(after)) {
    const roomId = Number(key);

    if (before[roomId] !== after[roomId]) ids.add(roomId);
  }

  return [...ids];
}

/**
 * An HTTP reply's rows as local sync events, less any row event for a room the sync path changed
 * after `since` (the request's ticket); none at all once a resync made the reply stale.
 */
export function untouchedReplyEvents(
  state: State,
  events: readonly SyncEvent[],
  since: number,
): readonly SyncEvent[] {
  if (isStale(state, since)) {
    return [];
  }

  return events.filter((event) => {
    switch (event.type) {
      case "sidebar.row.upserted":
        return !touchedSince(state, event.data.room.id, since);
      case "sidebar.row.removed":
        return !touchedSince(state, event.data.roomId, since);
      default:
        return true;
    }
  });
}
