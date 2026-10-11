/**
 * The shared workspace layout (`crates/api/src/organize.rs`, `WorkspaceCategory` in
 * `crates/db/src/models/workspace_category.rs`): categories and room positions administrators
 * set for everyone, with the server's rules. Category positions count from 0 and are compacted
 * after a delete; a move (to a position, or before a room of the target group) renumbers its
 * source and target groups densely; a stale category order
 * is a 409; direct messages can't be placed; every write publishes `workspace.layout.updated`.
 * `/__mock/workspace-layout` stands in for another administrator's change.
 */
import type { WorkspaceCategory } from "../../src/gen/WorkspaceCategory.ts";
import type { WorkspaceLayout } from "../../src/gen/WorkspaceLayout.ts";
import type { WorkspaceRoomPosition } from "../../src/gen/WorkspaceRoomPosition.ts";
import {
  conflict,
  forbidden,
  type MockResponse,
  noContent,
  notFound,
  ok,
  validation,
} from "../http.ts";
import {
  field,
  intField,
  isNumber,
  isRecord,
  type Json,
  stringArrayField,
  stringField,
} from "../json.ts";
import { type Route, route, type S2Context } from "../s2/context.ts";
import { type RoomRecord, VIEWER_ID, type World } from "../seed.ts";
import { CATEGORY_NAME_LIMIT } from "./organize.ts";

function byPosition(left: WorkspaceCategory, right: WorkspaceCategory): number {
  return left.position - right.position || left.id - right.id;
}

/** A room the shared layout can place: anything but a direct message. */
function placeable(record: RoomRecord): boolean {
  return record.room.kind !== "direct";
}

/** The rooms of one group (`null`: uncategorized) in the server's order. */
function groupIds(world: World, categoryId: number | null): number[] {
  return [...world.rooms.values()]
    .filter(
      (record) =>
        placeable(record) &&
        (world.workspacePlacements.get(record.room.id)?.categoryId ?? null) === categoryId,
    )
    .sort((left, right) => {
      const a = world.workspacePlacements.get(left.room.id)?.position ?? null;
      const b = world.workspacePlacements.get(right.room.id)?.position ?? null;

      if (a !== b) {
        return a === null ? 1 : b === null ? -1 : a - b;
      }

      return (
        (left.room.name ?? "").toLowerCase().localeCompare((right.room.name ?? "").toLowerCase()) ||
        left.room.id - right.room.id
      );
    })
    .map((record) => record.room.id);
}

/** Numbers a group's rooms 0, 1, … in the order given (`write_rooms`). */
function writeGroup(world: World, categoryId: number | null, ids: readonly number[]): void {
  ids.forEach((roomId, position) => {
    world.workspacePlacements.set(roomId, { categoryId, position });
  });
}

/**
 * The viewer's layout, as `GET /sidebar` and `workspace.layout.updated` carry it: the visible,
 * non-direct rooms they belong to, and every category for an administrator, only the categories
 * holding one of those rooms for anyone else.
 */
export function workspaceLayoutOf(world: World): WorkspaceLayout {
  const admin = world.users.get(VIEWER_ID)?.role === "administrator";

  const rooms: WorkspaceRoomPosition[] = [...world.rooms.values()]
    .flatMap((record) => {
      if (
        !placeable(record) ||
        !record.memberIds.includes(VIEWER_ID) ||
        record.membership.involvement === "invisible"
      ) {
        return [];
      }

      const placement = world.workspacePlacements.get(record.room.id);

      return [
        {
          roomId: record.room.id,
          workspaceCategoryId: placement?.categoryId ?? null,
          position: placement?.position ?? null,
          name: (record.room.name ?? "").toLowerCase(),
        },
      ];
    })
    .sort(
      (left, right) =>
        (left.workspaceCategoryId ?? -1) - (right.workspaceCategoryId ?? -1) ||
        (left.position ?? Number.MAX_SAFE_INTEGER) - (right.position ?? Number.MAX_SAFE_INTEGER) ||
        left.name.localeCompare(right.name) ||
        left.roomId - right.roomId,
    )
    .map(({ roomId, workspaceCategoryId, position }) => ({
      roomId,
      workspaceCategoryId,
      position,
    }));

  const held = new Set(rooms.map((room) => room.workspaceCategoryId));

  return {
    categories: world.workspaceCategories
      .filter((category) => admin || held.has(category.id))
      .sort(byPosition),
    rooms,
  };
}

export interface WorkspaceLayoutModule {
  readonly routes: readonly Route[];
  /**
   * `/__mock/workspace-layout`: another administrator's change, published to the viewer. Takes
   * `categories` (names, in order, replacing every shared category) and `rooms` (`{ roomId,
   * category }`, the category by its index in `categories`, `null` for none, in order).
   */
  readonly control: (body: Json | undefined) => WorkspaceLayout;
}

export function createWorkspaceLayout(ctx: S2Context): WorkspaceLayoutModule {
  const world = () => ctx.world();

  const publish = () =>
    ctx.publish([
      { topic: "user", type: "workspace.layout.updated", data: workspaceLayoutOf(world()) },
    ]);

  const requireAdministrator = () => {
    if (world().users.get(VIEWER_ID)?.role !== "administrator") {
      throw forbidden("Only administrators can organize the workspace");
    }
  };

  const validName = (raw: string | null): string => {
    if (raw === null || raw.trim() === "") throw validation("name", "Name can't be blank");

    if ([...raw].length > CATEGORY_NAME_LIMIT) {
      throw validation("name", `Name is too long (maximum is ${CATEGORY_NAME_LIMIT} characters)`);
    }

    return raw;
  };

  const categoryOr404 = (categoryId: number): WorkspaceCategory => {
    const category = world().workspaceCategories.find((held) => held.id === categoryId);

    if (category === undefined) throw notFound("Workspace category not found");

    return category;
  };

  const ordered = () => world().workspaceCategories.toSorted(byPosition);

  const renumber = (categories: readonly WorkspaceCategory[]) => {
    world().workspaceCategories = categories.map((category, position) => ({
      ...category,
      position,
    }));
  };

  const create = (body: Json | undefined): MockResponse => {
    requireAdministrator();

    const name = validName(stringField(body, "name"));
    const held = world().workspaceCategories;

    const category: WorkspaceCategory = {
      id: held.reduce((highest, candidate) => Math.max(highest, candidate.id), 0) + 1,
      name,
      position: held.reduce((highest, candidate) => Math.max(highest, candidate.position), -1) + 1,
    };

    world().workspaceCategories = [...held, category];
    publish();

    return ok(category, 201);
  };

  const rename = (categoryId: number, body: Json | undefined): MockResponse => {
    requireAdministrator();

    const current = categoryOr404(categoryId);
    const category = { ...current, name: validName(stringField(body, "name")) };

    world().workspaceCategories = world().workspaceCategories.map((held) =>
      held.id === categoryId ? category : held,
    );

    if (category.name !== current.name) publish();

    return ok(category);
  };

  const destroy = (categoryId: number): MockResponse => {
    requireAdministrator();
    categoryOr404(categoryId);

    const ids = [...groupIds(world(), null), ...groupIds(world(), categoryId)];

    renumber(ordered().filter((category) => category.id !== categoryId));
    writeGroup(world(), null, ids);
    publish();

    return noContent();
  };

  const reorder = (body: Json | undefined): MockResponse => {
    requireAdministrator();

    const raw = field(body, "categoryIds");
    const ids = Array.isArray(raw) ? raw.filter(isNumber) : [];
    const held = ordered();

    const matches =
      Array.isArray(raw) &&
      ids.length === raw.length &&
      ids.toSorted((a, b) => a - b).join() ===
        held
          .map((category) => category.id)
          .sort((a, b) => a - b)
          .join();

    if (!matches) {
      throw conflict("Workspace categories changed. Refresh and try again.");
    }

    renumber(ids.map(categoryOr404));
    publish();

    return ok(workspaceLayoutOf(world()));
  };

  const move = (roomId: number, body: Json | undefined): MockResponse => {
    requireAdministrator();

    const record = ctx.roomOr404(roomId);
    const raw = field(body, "workspaceCategoryId");
    const categoryId = intField(body, "workspaceCategoryId");
    const position = intField(body, "position");
    const beforeRoomId = intField(body, "beforeRoomId");

    if (raw !== null && categoryId === null) {
      throw validation("workspaceCategoryId", "Workspace category must be a number or null");
    }

    if (position === null) throw validation("position", "Position must be a number");

    if (!placeable(record)) {
      throw validation("workspaceCategoryId", "Cannot categorize direct messages");
    }

    if (categoryId !== null) categoryOr404(categoryId);

    const source = world().workspacePlacements.get(roomId)?.categoryId ?? null;
    const target = groupIds(world(), categoryId).filter((id) => id !== roomId);

    const at =
      beforeRoomId === null
        ? Math.min(Math.max(position, 0), target.length)
        : target.indexOf(beforeRoomId);

    if (at === -1) {
      throw validation("beforeRoomId", "Before room must be a room in the target category");
    }

    target.splice(at, 0, roomId);

    if (source !== categoryId) {
      writeGroup(
        world(),
        source,
        groupIds(world(), source).filter((id) => id !== roomId),
      );
    }

    writeGroup(world(), categoryId, target);
    publish();

    return ok(workspaceLayoutOf(world()));
  };

  const control = (body: Json | undefined): WorkspaceLayout => {
    const names = stringArrayField(body, "categories");
    const rooms = field(body, "rooms");

    if (names !== null) {
      world().workspaceCategories = names.map((name, position) => ({
        id: position + 1,
        name,
        position,
      }));
      world().workspacePlacements.clear();
    }

    if (Array.isArray(rooms)) {
      const groups = new Map<number | null, number[]>();

      for (const entry of rooms) {
        const roomId = isRecord(entry) ? intField(entry, "roomId") : null;
        const index = isRecord(entry) ? intField(entry, "category") : null;
        const categoryId = index === null ? null : (world().workspaceCategories[index]?.id ?? null);

        if (roomId !== null) groups.set(categoryId, [...(groups.get(categoryId) ?? []), roomId]);
      }

      for (const [categoryId, ids] of groups) writeGroup(world(), categoryId, ids);
    }

    publish();

    return workspaceLayoutOf(world());
  };

  const id = (request: { readonly ids: readonly number[] }) => request.ids[0] ?? 0;

  return {
    control,
    routes: [
      route("POST", /^\/workspace_categories$/, (request) => create(request.body)),
      route("PUT", /^\/workspace_categories\/order$/, (request) => reorder(request.body)),
      route("PATCH", /^\/workspace_categories\/(\d+)$/, (request) =>
        rename(id(request), request.body),
      ),
      route("DELETE", /^\/workspace_categories\/(\d+)$/, (request) => destroy(id(request))),
      route("PUT", /^\/rooms\/(\d+)\/workspace_category$/, (request) =>
        move(id(request), request.body),
      ),
    ],
  };
}
