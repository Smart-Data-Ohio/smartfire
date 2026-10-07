/**
 * Sidebar organisation (S3, `crates/api_types/src/organize.rs`): the viewer's categories,
 * favourites and notification levels, with the server's rules: category positions from 1, a
 * 50-character name limit, a 409 for a stale category order, favourites appended at
 * `max + 1`, unfavourites leaving gaps, a move renumbering every favourite 0, 1, …, and the
 * sync events each change publishes on the viewer's `user` topic.
 */
import type { FavoriteList } from "../../src/gen/FavoriteList.ts";
import type { Involvement } from "../../src/gen/Involvement.ts";
import type { Membership } from "../../src/gen/Membership.ts";
import type { RoomCategory } from "../../src/gen/RoomCategory.ts";
import type { RoomCategoryList } from "../../src/gen/RoomCategoryList.ts";
import type { SidebarRow } from "../../src/gen/SidebarRow.ts";
import { conflict, type MockResponse, noContent, notFound, ok, validation } from "../http.ts";
import { booleanField, field, intField, isNumber, type Json, stringField } from "../json.ts";
import { type Route, route, type S2Context } from "../s2/context.ts";
import type { RoomRecord } from "../seed.ts";
import type { Outgoing } from "../sync.ts";

/** `RoomCategory::NAME_LIMIT`. */
export const CATEGORY_NAME_LIMIT = 50;

const INVOLVEMENTS: readonly Involvement[] = [
  "invisible",
  "nothing",
  "muted",
  "mentions",
  "everything",
];

export interface Organize {
  readonly routes: readonly Route[];
}

function byPosition(left: RoomCategory, right: RoomCategory): number {
  return left.position - right.position || left.id - right.id;
}

function byFavorite(left: RoomRecord, right: RoomRecord): number {
  return (
    (left.membership.favoritePosition ?? 0) - (right.membership.favoritePosition ?? 0) ||
    left.membership.id - right.membership.id
  );
}

/** Whether the room has a sidebar row: a hidden (`invisible`) membership has none. */
function visible(record: RoomRecord): boolean {
  return record.membership.involvement !== "invisible";
}

/** A category name as the model validates it: present (not just spaces) and at most 50 long. */
function validName(raw: string | null): string {
  if (raw === null || raw.trim() === "") throw validation("name", "Name can't be blank");

  if ([...raw].length > CATEGORY_NAME_LIMIT) {
    throw validation("name", `Name is too long (maximum is ${CATEGORY_NAME_LIMIT} characters)`);
  }

  return raw;
}

/** The organisation routes, over the server's world. */
export function createOrganize(ctx: S2Context): Organize {
  const world = () => ctx.world();

  const rowEvent = (record: RoomRecord): Outgoing => ({
    topic: "user",
    type: "sidebar.row.upserted",
    data: ctx.sidebarRow(record),
  });

  const categoryEvent = (category: RoomCategory): Outgoing => ({
    topic: "user",
    type: "sidebar.category.upserted",
    data: category,
  });

  const categoryOr404 = (categoryId: number): RoomCategory => {
    const category = world().categories.find((candidate) => candidate.id === categoryId);

    if (category === undefined) throw notFound("Room category not found");

    return category;
  };

  const replaceCategory = (category: RoomCategory) => {
    world().categories = [
      ...world().categories.filter((held) => held.id !== category.id),
      category,
    ].sort(byPosition);
  };

  /** Every favourite, hidden ones included (`favorites_for_user`). */
  const allFavorites = (): RoomRecord[] =>
    [...world().rooms.values()]
      .filter((record) => record.membership.favoritePosition !== null)
      .sort(byFavorite);

  /** The favourites the sidebar shows, in its order. */
  const favorites = (): RoomRecord[] => allFavorites().filter(visible);

  /**
   * A room the viewer is a member of, hidden or not: as in the classic controllers, organising
   * calls accept a hidden room, which stays hidden.
   */
  const memberRoomOr404 = (roomId: number): RoomRecord => {
    const record = world().rooms.get(roomId);

    if (record === undefined) throw notFound("Room not found");

    return record;
  };

  /** A change to a hidden room's membership publishes nothing: it has no sidebar row. */
  const rowEvents = (records: readonly RoomRecord[]): Outgoing[] =>
    records.flatMap((record) => (visible(record) ? [rowEvent(record)] : []));

  const setMembership = (record: RoomRecord, change: Partial<Membership>) => {
    record.membership = { ...record.membership, ...change };
  };

  // --- categories ---

  const createCategory = (body: Json | undefined): MockResponse => {
    const name = validName(stringField(body, "name"));
    const held = world().categories;

    const category: RoomCategory = {
      id: held.reduce((highest, candidate) => Math.max(highest, candidate.id), 0) + 1,
      name,
      collapsed: booleanField(body, "collapsed") ?? false,
      position: held.reduce((highest, candidate) => Math.max(highest, candidate.position), 0) + 1,
    };

    replaceCategory(category);
    ctx.publish([categoryEvent(category)]);

    return ok(category, 201);
  };

  const updateCategory = (categoryId: number, body: Json | undefined): MockResponse => {
    const current = categoryOr404(categoryId);
    const rawName = stringField(body, "name");

    const category: RoomCategory = {
      ...current,
      name: rawName === null ? current.name : validName(rawName),
      collapsed: booleanField(body, "collapsed") ?? current.collapsed,
    };

    replaceCategory(category);
    ctx.publish([categoryEvent(category)]);

    return ok(category);
  };

  const deleteCategory = (categoryId: number): MockResponse => {
    categoryOr404(categoryId);

    const events: Outgoing[] = [];

    for (const record of world().rooms.values()) {
      if (record.membership.roomCategoryId !== categoryId) continue;

      setMembership(record, { roomCategoryId: null });

      if (record.membership.involvement !== "invisible") events.push(rowEvent(record));
    }

    world().categories = world().categories.filter((category) => category.id !== categoryId);
    events.push({ topic: "user", type: "sidebar.category.removed", data: { id: categoryId } });
    ctx.publish(events);

    return noContent();
  };

  const reorderCategories = (body: Json | undefined): MockResponse => {
    const raw = field(body, "categoryIds");
    const ids = Array.isArray(raw) ? raw.filter(isNumber) : [];
    const held = world().categories;
    const known = new Set(held.map((category) => category.id));

    const matches =
      Array.isArray(raw) &&
      ids.length === raw.length &&
      ids.length === held.length &&
      new Set(ids).size === ids.length &&
      ids.every((id) => known.has(id));

    if (!matches) {
      throw conflict("The categories changed since they were loaded. Reload and try again.");
    }

    const events: Outgoing[] = [];

    const reordered = ids.map((id, index) => {
      const category = categoryOr404(id);
      const moved = { ...category, position: index + 1 };

      if (moved.position !== category.position) events.push(categoryEvent(moved));

      return moved;
    });

    world().categories = reordered;
    ctx.publish(events);

    const list: RoomCategoryList = { categories: reordered };

    return ok(list);
  };

  // --- placement ---

  const assignCategory = (roomId: number, body: Json | undefined): MockResponse => {
    const record = memberRoomOr404(roomId);
    const categoryId = intField(body, "roomCategoryId");

    if (field(body, "roomCategoryId") !== null && categoryId === null) {
      throw validation("roomCategoryId", "Room category must be a number or null");
    }

    if (record.room.kind !== "open" && record.room.kind !== "closed") {
      throw validation("room", "Only channels can be put in a category");
    }

    if (categoryId !== null) categoryOr404(categoryId);

    setMembership(record, { roomCategoryId: categoryId });
    ctx.publish(rowEvents([record]));

    const row: SidebarRow = ctx.sidebarRow(record);

    return ok(row);
  };

  const favorite = (roomId: number): MockResponse => {
    const record = memberRoomOr404(roomId);

    if (record.membership.favoritePosition === null) {
      const highest = allFavorites().reduce(
        (max, held) => Math.max(max, held.membership.favoritePosition ?? -1),
        -1,
      );

      setMembership(record, { favoritePosition: highest + 1 });
      ctx.publish(rowEvents([record]));
    }

    return ok(ctx.sidebarRow(record));
  };

  const unfavorite = (roomId: number): MockResponse => {
    const record = memberRoomOr404(roomId);

    if (record.membership.favoritePosition !== null) {
      // The others keep their positions: favourites can have gaps.
      setMembership(record, { favoritePosition: null });
      ctx.publish(rowEvents([record]));
    }

    return ok(ctx.sidebarRow(record));
  };

  const moveFavorite = (roomId: number, body: Json | undefined): MockResponse => {
    const record = memberRoomOr404(roomId);
    const position = intField(body, "position");

    if (position === null) throw validation("position", "Position must be a number");

    if (record.membership.favoritePosition !== null) {
      // `position` is an index among the favourites the sidebar shows: the room lands just before
      // the shown favourite now at that index (or after every favourite). Hidden favourites keep
      // their places, and every favourite is renumbered from 0.
      const others = allFavorites().filter((held) => held !== record);
      const shown = others.filter(visible);
      const anchor = shown[Math.min(Math.max(position, 0), shown.length)];
      const at = anchor === undefined ? others.length : others.indexOf(anchor);
      const order = [...others.slice(0, at), record, ...others.slice(at)];

      const changed: RoomRecord[] = [];

      order.forEach((held, renumbered) => {
        if (held.membership.favoritePosition === renumbered) return;

        setMembership(held, { favoritePosition: renumbered });
        changed.push(held);
      });

      ctx.publish(rowEvents(changed));
    }

    const list: FavoriteList = { rows: favorites().map(ctx.sidebarRow) };

    return ok(list);
  };

  // --- involvement ---

  const updateInvolvement = (roomId: number, body: Json | undefined): MockResponse => {
    // An invisible room is still the viewer's: it can come back from here.
    const record = world().rooms.get(roomId);

    if (record === undefined) throw notFound("Room not found");

    const raw = stringField(body, "involvement");
    const involvement = INVOLVEMENTS.find((candidate) => candidate === raw);

    if (involvement === undefined) {
      throw validation("involvement", "Involvement is not included in the list");
    }

    const before = record.membership.involvement;
    const events: Outgoing[] = [];

    setMembership(record, { involvement });

    if (involvement === "muted" && before !== "muted") {
      // Muting marks the room read; from then on only a mention makes it unread.
      setMembership(record, {
        unreadAt: null,
        lastReadMessageId: record.messages.at(-1)?.id ?? record.membership.lastReadMessageId,
      });
      record.mentionCount = 0;
      events.push({ topic: "user", type: "room.read", data: { roomId } });
    }

    if (involvement === "invisible") {
      if (before !== "invisible") {
        events.push({ topic: "user", type: "sidebar.row.removed", data: { roomId } });
      }
    } else if (involvement !== before) {
      events.push(rowEvent(record));
    }

    ctx.publish(events);

    const membership: Membership = record.membership;

    return ok(membership);
  };

  const id = (request: { readonly ids: readonly number[] }) => request.ids[0] ?? 0;

  return {
    routes: [
      route("POST", /^\/room_categories$/, (request) => createCategory(request.body)),
      route("PUT", /^\/room_categories\/order$/, (request) => reorderCategories(request.body)),
      route("PATCH", /^\/room_categories\/(\d+)$/, (request) =>
        updateCategory(id(request), request.body),
      ),
      route("DELETE", /^\/room_categories\/(\d+)$/, (request) => deleteCategory(id(request))),
      route("PUT", /^\/rooms\/(\d+)\/category$/, (request) =>
        assignCategory(id(request), request.body),
      ),
      route("POST", /^\/rooms\/(\d+)\/favorite$/, (request) => favorite(id(request))),
      route("DELETE", /^\/rooms\/(\d+)\/favorite$/, (request) => unfavorite(id(request))),
      route("PATCH", /^\/rooms\/(\d+)\/favorite$/, (request) =>
        moveFavorite(id(request), request.body),
      ),
      route("PUT", /^\/rooms\/(\d+)\/involvement$/, (request) =>
        updateInvolvement(id(request), request.body),
      ),
    ],
  };
}
