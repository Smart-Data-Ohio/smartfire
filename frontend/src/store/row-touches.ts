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

export interface RowTouches {
  readonly clock: number;
  /** The clock when the latest sync resync landed: a ticket from before it is stale. */
  readonly resyncEpoch: number;
  /**
   * The clock at each room's latest touch, kept after its row leaves, only while a request
   * older than the touch is in flight (`pruneTouches`).
   */
  readonly at: Readonly<Record<number, number>>;
}

export const noRowTouches: RowTouches = { clock: 0, resyncEpoch: 0, at: {} };

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

/** Records that the sync path (or a read made here) just changed these rooms' rows. */
export function touchRows(state: State, roomIds: Iterable<number>): State {
  const clock = state.rowTouches.clock + 1;
  const at = { ...state.rowTouches.at };
  let touched = false;

  for (const roomId of roomIds) {
    at[roomId] = clock;
    touched = true;
  }

  return touched ? { ...state, rowTouches: { ...state.rowTouches, clock, at } } : state;
}

/**
 * Forgets the touches no request in flight can be older than: those at or before `oldest`, the
 * oldest ticket still held, or every one when none is (`undefined`). The same state when nothing
 * goes.
 */
export function pruneTouches(state: State, oldest: number | undefined): State {
  const entries = Object.entries(state.rowTouches.at);
  const kept = entries.filter(([, clock]) => oldest !== undefined && clock > oldest);

  if (kept.length === entries.length) {
    return state;
  }

  return { ...state, rowTouches: { ...state.rowTouches, at: Object.fromEntries(kept) } };
}

/**
 * Whether an HTTP 404 (the request holding `since`) is older than the store's row for `roomId`:
 * a row a resync installed, or sync restored, after the request began stays. With no row, the
 * store already agrees the room is gone.
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
