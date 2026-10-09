/**
 * When the sync path (or a read made here) last changed each sidebar row, on a client clock.
 *
 * Sidebar rows arrive two ways: sync events, which the server publishes after every write that
 * changes a row, in order; and HTTP replies (the whole sidebar, an organising reply, a join),
 * which are snapshots taken when the server answered. A reply can be older than a sync row that
 * landed while it was in flight, so every HTTP install takes the clock before its request and
 * leaves alone any row touched after that. Preferring the sync row converges: whatever the
 * reply knew that the sync row didn't, the server publishes again.
 */
import type { SidebarRow, SyncEvent } from "./model.ts";
import type { State } from "./state.ts";

export interface RowTouches {
  readonly clock: number;
  /** The clock at each room's latest touch, kept after its row leaves. */
  readonly at: Readonly<Record<number, number>>;
}

export const noRowTouches: RowTouches = { clock: 0, at: {} };

/** The clock now: take it before an HTTP request whose reply installs sidebar rows. */
export function rowClock(state: State): number {
  return state.rowTouches.clock;
}

/** Whether the sync path changed `roomId`'s row after `since` (a `rowClock`). */
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

  return touched ? { ...state, rowTouches: { clock, at } } : state;
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
 * after `since` (the `rowClock` taken before the request).
 */
export function untouchedReplyEvents(
  state: State,
  events: readonly SyncEvent[],
  since: number,
): readonly SyncEvent[] {
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
