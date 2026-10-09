/**
 * Sidebar organisation as Effect programs (S3): categories, favourites and notification levels.
 * Every change shows at once as an overlay entry (store/organize.ts) and is dropped when it
 * settles: by then the server's reply has landed, or, after a refusal, the sidebar is as it was.
 *
 * The server calls run one at a time, in the order the person made the changes, so a favourite
 * dragged straight after it was starred moves the star the server already has.
 */
import { Clock, Effect, Semaphore } from "effect";
import { sidebar as fetchSidebar } from "../api/endpoints.ts";
import { NetworkError } from "../api/errors.ts";
import * as api from "../api/organize-endpoints.ts";
import type { Involvement } from "../gen/Involvement.ts";
import type { RoomCategory, SidebarRow } from "../store/model.ts";
import {
  canCategorize,
  favoriteRows,
  isInSlot,
  type MembershipPatch,
  organizedSidebar,
  placementPatches,
  type RoomSlot,
  type SidebarOverlay,
} from "../store/organize.ts";
import { mutations, sidebarRowClock, store } from "../store/store.ts";

/** One server change at a time, in order. */
const lock = Semaphore.makeUnsafe(1);

/** Temporary ids of categories being created, to the ids the server gave them. */
const createdIds = new Map<number, number>();

let nextTemporaryId = -1;

const view = () => organizedSidebar(store.getState().sidebar);

/** The id the server knows a category by (a new one's, once it has been created). */
const serverCategoryId = (categoryId: number) => createdIds.get(categoryId) ?? categoryId;

const memberships = (entries: SidebarOverlay["memberships"]): SidebarOverlay => ({
  memberships: entries,
  categories: {},
});

const categories = (entries: SidebarOverlay["categories"]): SidebarOverlay => ({
  memberships: {},
  categories: entries,
});

/**
 * How long one change may hold the lock. A call the server never answers would otherwise freeze
 * every organising change after it.
 */
const SERVER_TIMEOUT = "15 seconds";

/**
 * Shows `entry` from now until `change` settles; the change waits its turn for the server. The
 * entry goes either way: the reply has landed by then, or the refusal leaves the old state. A
 * change still unanswered after `SERVER_TIMEOUT` is given up (the call is interrupted and the
 * entry goes) so the next one can run; the server may have made it after all, so the sidebar is
 * fetched again in the background.
 */
const pending = <A, E, R>(entry: SidebarOverlay, change: Effect.Effect<A, E, R>) =>
  Effect.sync(() => mutations.addSidebarOverlay(entry)).pipe(
    Effect.andThen(
      Semaphore.withPermit(
        lock,
        change.pipe(Effect.timeoutOrElse({ duration: SERVER_TIMEOUT, orElse: () => stalled })),
      ),
    ),
    Effect.ensuring(Effect.sync(() => mutations.dropSidebarOverlay(entry))),
  );

/**
 * Lands an organising reply's rows: for a row the sidebar has, only the organisation fields (the
 * reply may be older than a sync event with newer counts); a row it lacks lands whole, through
 * the reducer the `sidebar.row.upserted` event uses, unless the sync path changed (or removed) it
 * after `since`, the `sidebarRowClock()` taken before the request.
 */
const landRows = Effect.fn("organize.landRows")(function* (
  rows: readonly SidebarRow[],
  since: number,
) {
  const state = store.getState();
  const viewerId = state.me?.user.id ?? state.boot?.user.id ?? 0;

  // Local events: `seq` only matters to the sync cursor, which never sees these.
  const fresh = rows.flatMap((row) =>
    state.sidebar.rows[row.room.id] === undefined
      ? [{ seq: 0, topic: `user:${viewerId}`, type: "sidebar.row.upserted" as const, data: row }]
      : [],
  );

  mutations.mergeOrganization(rows);

  if (fresh.length > 0) {
    mutations.landReplyRows(fresh, yield* Clock.currentTimeMillis, since);
  }
});

/** The server's copy of a row, without pending changes: what the next call starts from. */
const serverRow = (roomId: number) => store.getState().sidebar.rows[roomId];

/**
 * Loads the sidebar again (after a 409, the list the client sent was stale). Rows the sync path
 * changed while it was on its way are newer than it, so they stay.
 */
const refetchSidebar = Effect.suspend(() => {
  const since = sidebarRowClock();

  return fetchSidebar().pipe(
    Effect.tap((sidebar) => Effect.sync(() => mutations.loadSidebar(sidebar, since))),
  );
}).pipe(
  Effect.catch((error) => Effect.logWarning("organize: sidebar refetch failed", error.message)),
);

/** A change the server didn't answer in time: refetch in the background, and fail with why. */
const stalled = Effect.forkDetach(refetchSidebar).pipe(
  Effect.andThen(
    Effect.fail(
      new NetworkError({
        message: "The server didn't answer in time, so the change was undone. Try it again.",
      }),
    ),
  ),
);

/** The category a room ends up in once it leaves the favourites for `slot`. */
function categoryAfter(slot: RoomSlot, current: number | null): number | null {
  switch (slot.kind) {
    case "category":
      return serverCategoryId(slot.categoryId);
    case "channels":
      return null;
    default:
      return current;
  }
}

/** The calls that take a room from where the server has it to `slot`. */
const placeOnServer = Effect.fn("organize.placeOnServer")(function* (
  roomId: number,
  slot: RoomSlot,
) {
  const row = serverRow(roomId);

  if (row === undefined) {
    return;
  }

  const favorite = row.membership.favoritePosition !== null;

  if (slot.kind === "favorite") {
    if (!favorite) {
      const since = sidebarRowClock();

      yield* landRows([yield* api.favorite(roomId)], since);
    }

    const others = favoriteRows(store.getState().sidebar).filter(
      (candidate) => candidate.room.id !== roomId,
    );

    // A new favourite lands at the end, which may already be where it was dropped.
    if (favorite || slot.index < others.length) {
      const since = sidebarRowClock();

      yield* landRows((yield* api.moveFavorite(roomId, slot.index)).rows, since);
    }

    return;
  }

  if (favorite) {
    const since = sidebarRowClock();

    yield* landRows([yield* api.unfavorite(roomId)], since);
  }

  const categoryId = categoryAfter(slot, row.membership.roomCategoryId);

  if (canCategorize(row) && categoryId !== row.membership.roomCategoryId) {
    const since = sidebarRowClock();

    yield* landRows([yield* api.assignCategory(roomId, categoryId)], since);
  }
});

/**
 * Puts a room in `slot`: among the favourites at an index, in a category, back in Channels, or
 * out of the favourites. It moves at once; a refusal puts it back.
 */
export const moveRoom = Effect.fn("organize.moveRoom")(function* (roomId: number, slot: RoomSlot) {
  const current = view();
  const row = current.rows[roomId];

  if (row === undefined || isInSlot(current, row, slot)) {
    return;
  }

  yield* pending(memberships(placementPatches(current, roomId, slot)), placeOnServer(roomId, slot));
});

/** Stars a room: to the end of the favourites, or at `index`. */
export const favorite = (roomId: number, index?: number) =>
  moveRoom(roomId, { kind: "favorite", index: index ?? favoriteRows(view()).length });

/** Unstars a room; a channel goes back to its category, or to Channels. */
export const unfavorite = (roomId: number) => moveRoom(roomId, { kind: "unfavorite" });

/**
 * Adds a category at the end, showing it at once under a temporary id. With `roomId`, that
 * channel moves into it as well (Move to → New category).
 */
export const createCategory = Effect.fn("organize.createCategory")(function* (
  name: string,
  roomId: number | null = null,
) {
  const temporaryId = nextTemporaryId;
  const held = view().categories;

  nextTemporaryId -= 1;

  const draft: RoomCategory = {
    id: temporaryId,
    name: name.trim(),
    collapsed: false,
    position: held.reduce((highest, category) => Math.max(highest, category.position), 0) + 1,
  };

  const row = roomId === null ? undefined : view().rows[roomId];

  const entry: SidebarOverlay = {
    categories: { [temporaryId]: draft },
    memberships:
      row === undefined
        ? {}
        : placementPatches(view(), row.room.id, { kind: "category", categoryId: temporaryId }),
  };

  return yield* pending(
    entry,
    Effect.gen(function* () {
      const category = yield* api.createCategory({ name: draft.name });
      const slot: RoomSlot = { kind: "category", categoryId: category.id };

      // The real category replaces the draft at once; the room shows in it while it moves.
      const settled = memberships(
        row === undefined ? {} : placementPatches(view(), row.room.id, slot),
      );

      createdIds.set(temporaryId, category.id);
      mutations.landCreatedCategory(category, entry, settled);

      if (row !== undefined) {
        yield* placeOnServer(row.room.id, slot).pipe(
          Effect.ensuring(Effect.sync(() => mutations.dropSidebarOverlay(settled))),
        );
      }

      return category;
    }),
  );
});

/** The category as the viewer sees it now, with `change` applied. */
const changedCategory = (categoryId: number, change: Partial<RoomCategory>) => {
  const category = view().categories.find((candidate) => candidate.id === categoryId);

  return category === undefined ? null : { ...category, ...change };
};

/** Renames a category (the name is trimmed; 1 to 50 characters, or the server says 422). */
export const renameCategory = Effect.fn("organize.renameCategory")(function* (
  categoryId: number,
  name: string,
) {
  const trimmed = name.trim();
  const next = changedCategory(categoryId, { name: trimmed });

  if (next === null) {
    return;
  }

  yield* pending(
    categories({ [categoryId]: next }),
    Effect.gen(function* () {
      mutations.upsertCategory(
        yield* api.updateCategory(serverCategoryId(categoryId), { name: trimmed }),
      );
    }),
  );
});

/** Folds or unfolds a category; the server remembers it, so other tabs follow. */
export const setCollapsed = Effect.fn("organize.setCollapsed")(function* (
  categoryId: number,
  collapsed: boolean,
) {
  const next = changedCategory(categoryId, { collapsed });

  if (next === null) {
    return;
  }

  yield* pending(
    categories({ [categoryId]: next }),
    Effect.gen(function* () {
      mutations.upsertCategory(
        yield* api.updateCategory(serverCategoryId(categoryId), { collapsed }),
      );
    }),
  );
});

/** Deletes a category; its channels go back to Channels. */
export const deleteCategory = Effect.fn("organize.deleteCategory")(function* (categoryId: number) {
  const inside = Object.values(view().rows).filter(
    (row) => row.membership.roomCategoryId === categoryId,
  );

  const entry: SidebarOverlay = {
    categories: { [categoryId]: null },
    memberships: Object.fromEntries(inside.map((row) => [row.room.id, { roomCategoryId: null }])),
  };

  yield* pending(
    entry,
    Effect.gen(function* () {
      const serverId = serverCategoryId(categoryId);

      yield* api.deleteCategory(serverId);
      mutations.removeCategory(serverId);
    }),
  );
});

/**
 * Puts the categories in this order (every one of them, once). When another tab added or removed
 * one meanwhile, the server answers 409: the sidebar is fetched again and the `Conflict` fails
 * the action, so the person can see the fresh list and try again.
 */
export const reorderCategories = Effect.fn("organize.reorderCategories")(function* (
  categoryIds: readonly number[],
) {
  const held = new Map(view().categories.map((category) => [category.id, category]));

  const entry = categories(
    Object.fromEntries(
      categoryIds.flatMap((id, index) => {
        const category = held.get(id);

        return category === undefined ? [] : [[id, { ...category, position: index + 1 }]];
      }),
    ),
  );

  // The ids are mapped when the call's turn comes: a category created just before has its real
  // id by then.
  yield* pending(
    entry,
    Effect.suspend(() => api.reorderCategories(categoryIds.map(serverCategoryId))).pipe(
      Effect.tap((list) => Effect.sync(() => mutations.setCategories(list.categories))),
      Effect.catchTag("Conflict", (conflict) =>
        refetchSidebar.pipe(Effect.andThen(Effect.fail(conflict))),
      ),
    ),
  );
});

/**
 * Sets the viewer's notification level. `invisible` takes the room out of the sidebar, `muted`
 * also marks it read (the server sends both events); the header follows at once.
 */
export const setInvolvement = Effect.fn("organize.setInvolvement")(function* (
  roomId: number,
  involvement: Involvement,
) {
  const patch: MembershipPatch =
    involvement === "muted" ? { involvement, unreadAt: null } : { involvement };

  yield* pending(
    memberships({ [roomId]: patch }),
    Effect.gen(function* () {
      const since = sidebarRowClock();

      mutations.setMembership(yield* api.updateInvolvement(roomId, involvement), since);
    }),
  );
});
