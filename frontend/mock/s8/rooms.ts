import type { RoomForm } from "../../src/gen/RoomForm.ts";
import type { RoomKind } from "../../src/gen/RoomKind.ts";
import type { RoomMutation } from "../../src/gen/RoomMutation.ts";
import type { User } from "../../src/gen/User.ts";
import { conflict, forbidden, HttpError, notFound, ok, validation } from "../http.ts";
import { field, isNumber, isRecord, isString, type Json } from "../json.ts";
import type { AdminModule } from "../s2/admin.ts";
import { firstId, type Route, route, type S2Context } from "../s2/context.ts";
import { lookupIcon } from "../s2/emoji.ts";
import type { Huddles } from "../s5/huddles.ts";
import { type RoomRecord, timestamp, VIEWER_ID } from "../seed.ts";

const TYPES: readonly RoomKind[] = ["open", "closed", "direct", "voice", "stage", "board"];

const DEFAULT_NAMES = {
  open: "New room",
  closed: "New room",
  direct: null,
  voice: "New voice channel",
  stage: "New stage channel",
  board: "New board",
} satisfies Record<RoomKind, string | null>;

function roomType(raw: Json | undefined): RoomKind {
  const type = TYPES.find((type) => type === raw);

  if (type === undefined) throw validation("type", "is not a supported room type");

  return type;
}

function nullableString(
  body: Json | undefined,
  key: string,
  required: boolean,
): string | null | undefined {
  const value = field(body, key);

  if (value === undefined && !required) return undefined;

  if (value === null || isString(value)) return value;

  if (value === undefined && required) return null;

  throw validation(key, "must be a string or null");
}

interface RoomManagement {
  readonly routes: readonly Route[];
}

function roomInvalid(field: string, detail: string, message: string): HttpError {
  const invalid = validation(field, detail);

  return new HttpError(422, { ...invalid.error, message });
}

/** Form/API room management on the explicit membership semantics of the classic writes. */
export function createRoomManagement(
  ctx: S2Context,
  admin: Pick<AdminModule, "roomCreationRestricted" | "hasIcon">,
  huddles: Pick<Huddles, "roomRoles" | "reviseRoom" | "removeRoom">,
): RoomManagement {
  const creationKeys = new WeakMap<ReturnType<S2Context["world"]>, Map<string, RoomRecord>>();
  const viewer = () => ctx.world().users.get(VIEWER_ID);
  const isAdmin = () => viewer()?.role === "administrator";
  const canCreate = () => isAdmin() || !admin.roomCreationRestricted();
  const canEdit = (record: RoomRecord) => isAdmin() || record.room.creatorId === VIEWER_ID;

  const groupCapable = (record: RoomRecord) =>
    record.room.kind === "direct" &&
    (record.memberIds.length > 2 || (record.room.name ?? "").trim() !== "");

  const canDelete = (record: RoomRecord) =>
    record.room.kind === "direct" ? !groupCapable(record) || isAdmin() : canEdit(record);

  const allowedTypes = () => (canCreate() ? [...TYPES] : ["direct" as const]);

  const activeUsers = (): User[] =>
    [...ctx.world().users.values()]
      .filter((user) => user.status === "active")
      .sort((a, b) => a.name.toLowerCase().localeCompare(b.name.toLowerCase()) || a.id - b.id);

  const form = (type: RoomKind, record: RoomRecord | null): RoomForm => {
    const active = activeUsers();
    const members = record?.memberIds ?? [];
    const direct = type === "direct";

    const selected =
      type === "open"
        ? active.map((user) => user.id)
        : record === null
          ? direct
            ? []
            : [VIEWER_ID]
          : active.filter((user) => members.includes(user.id)).map((user) => user.id);

    const others = members.filter((id) => id !== VIEWER_ID);

    const candidates = direct
      ? active.filter((user) => user.id !== VIEWER_ID && !members.includes(user.id))
      : active;

    const capable = record === null ? false : groupCapable(record);

    if (record === null && direct)
      candidates.sort(
        (a, b) => Number(ctx.world().stars.has(b.id)) - Number(ctx.world().stars.has(a.id)),
      );

    const roles =
      record === null
        ? new Map<number, "host">(type === "stage" ? [[VIEWER_ID, "host"]] : [])
        : huddles.roomRoles(record);

    return {
      type,
      roomId: record?.room.id ?? null,
      name: record === null ? DEFAULT_NAMES[type] : record.room.name,
      iconName: record?.room.iconName ?? null,
      displayName: record === null ? (DEFAULT_NAMES[type] ?? "") : ctx.displayName(record),
      userIds: selected,
      memberIds: [...members],
      candidateIds: candidates.map((user) => user.id),
      displayMemberIds:
        direct && record !== null ? (others.length === 0 ? [VIEWER_ID] : others) : [],
      users: ctx.usersFor([...candidates.map((user) => user.id), ...members]),
      allowedTypes: allowedTypes(),
      conversionTypes:
        record !== null && canEdit(record) && (type === "open" || type === "closed")
          ? ["open", "closed"]
          : [],
      canSubmit: record === null ? direct || canCreate() : direct ? capable : canEdit(record),
      canDelete: record !== null && canDelete(record),
      canLeave: record !== null && direct,
      groupCapable: capable,
      defaultInvolvement: direct ? "everything" : "mentions",
      stageRoles: [...roles].map(([userId, role]) => ({ userId, role })),
      topic: record?.room.topic ?? null,
    };
  };

  const parse = (body: Json | undefined, creating: boolean) => {
    if (!isRecord(body)) throw validation("base", "Invalid room body");
    const type = roomType(field(body, "type"));

    if (type === "direct") throw validation("type", "Use the direct message endpoint");

    const keys =
      type === "open" ? ["type", "name", "iconName"] : ["type", "name", "iconName", "userIds"];

    if (creating) keys.push("clientRoomId");
    else keys.push("topic");

    if (Object.keys(body).some((key) => !keys.includes(key)))
      throw validation("base", "Unknown room field");

    const raw = field(body, "userIds");
    const ids: number[] = [];

    if (type !== "open") {
      if (!Array.isArray(raw)) throw validation("userIds", "must be an array of numeric ids");

      for (const value of raw) {
        if (!isNumber(value) || !Number.isInteger(value))
          throw validation("userIds", "must be an array of numeric ids");

        if (ctx.world().users.has(value) && !ids.includes(value)) ids.push(value);
      }
    }

    return {
      type,
      ids,
      hasRemainingIds: Array.isArray(raw) && raw.length > 0,
      name: nullableString(body, "name", creating),
      iconName: nullableString(body, "iconName", creating),
      topic: creating ? undefined : nullableString(body, "topic", false),
    };
  };

  const normalizeIcon = (name: string | null | undefined): string | null =>
    name
      ?.trim()
      .replace(/^:+|:+$/g, "")
      .trim()
      .toLowerCase() || null;

  const icon = (name: string | null | undefined, previous: string | null): string | null => {
    if (name === undefined) return previous;

    const normalized = normalizeIcon(name);

    if (
      normalized !== null &&
      normalized !== previous &&
      lookupIcon(normalized) === null &&
      !admin.hasIcon(normalized)
    ) {
      throw roomInvalid("iconName", "is not a known icon", "Icon name is not a known icon");
    }

    return normalized;
  };

  const publish = (record: RoomRecord) => {
    if (!record.memberIds.includes(VIEWER_ID) || record.membership.involvement === "invisible") {
      ctx.publish([
        {
          topic: "user",
          type: "sidebar.row.removed",
          data: { roomId: record.room.id, refreshRoom: true },
        },
      ]);
    } else {
      ctx.publish([
        {
          topic: "user",
          type: "sidebar.row.upserted",
          data: { ...ctx.sidebarRow(record), refreshRoom: true },
        },
      ]);
    }
  };

  const result = (record: RoomRecord): RoomMutation => ({
    room: record.room,
    detail: record.memberIds.includes(VIEWER_ID) ? ctx.roomDetail(record.room.id) : null,
    row:
      record.memberIds.includes(VIEWER_ID) && record.membership.involvement !== "invisible"
        ? ctx.sidebarRow(record)
        : null,
  });

  const create = (body: Json | undefined) => {
    if (!canCreate()) throw forbidden();
    const input = parse(body, true);
    const rawKey = field(body, "clientRoomId");

    if (!isString(rawKey) || rawKey.trim() === "")
      throw validation("clientRoomId", "can't be blank");

    const key = `${VIEWER_ID}:${rawKey.trim()}`;
    const world = ctx.world();
    const keys = creationKeys.get(world) ?? new Map<string, RoomRecord>();
    const replay = keys.get(key);

    const memberIds = input.type === "open" ? activeUsers().map((user) => user.id) : input.ids;

    if (input.type === "stage" && !memberIds.includes(VIEWER_ID)) memberIds.unshift(VIEWER_ID);

    const normalizedIcon = normalizeIcon(input.iconName);

    // Like the API's room-backed key, edits can make a replay conflict with the current room.
    if (replay !== undefined) {
      const requested = new Set(memberIds);
      const existing = new Set(replay.memberIds);

      if (
        replay.room.kind !== input.type ||
        replay.room.name !== (input.name ?? null) ||
        replay.room.iconName !== normalizedIcon ||
        (input.type !== "open" &&
          (requested.size !== existing.size || [...requested].some((id) => !existing.has(id))))
      )
        throw conflict("clientRoomId was already used with different room parameters");

      return ok(result(replay));
    }

    const iconName = icon(input.iconName, null);
    const id = world.nextRoomId++;
    const createdAt = timestamp(ctx.now());

    const record: RoomRecord = {
      room: {
        id,
        kind: input.type,
        name: input.name ?? null,
        iconName,
        creatorId: VIEWER_ID,
        createdAt,
        updatedAt: createdAt,
        topic: null,
      },
      memberIds,
      membership: {
        id: world.nextMembershipId++,
        roomId: id,
        userId: VIEWER_ID,
        involvement: "mentions",
        unreadAt: null,
        lastReadMessageId: null,
        roomCategoryId: null,
        favoritePosition: null,
        stageRole: input.type === "stage" ? "host" : null,
      },
      messages: [],
      mentionCount: 0,
    };

    world.rooms.set(id, record);
    keys.set(key, record);
    creationKeys.set(world, keys);
    huddles.reviseRoom(record, []);
    publish(record);

    return ok(result(record), 201);
  };

  const update = (roomId: number, body: Json | undefined) => {
    const record = ctx.roomOr404(roomId);

    if (record.room.kind === "direct") throw notFound();

    if (!canEdit(record)) throw forbidden();
    const input = parse(body, false);
    const oldType = record.room.kind;
    const topic = input.topic === undefined ? record.room.topic : input.topic?.trim() || null;

    if (topic !== null && Array.from(topic).length > 1024)
      throw validation("topic", "is too long (maximum is 1024 characters)");

    if (
      !(
        oldType === input.type ||
        ((oldType === "open" || oldType === "closed") &&
          (input.type === "open" || input.type === "closed"))
      )
    )
      throw roomInvalid(
        "type",
        "cannot convert this room to that type",
        "Type cannot convert this room to that type",
      );

    // Numeric [] follows the classic empty-array path, not its [""] sentinel.
    if (input.type === "stage" && input.hasRemainingIds) {
      const hosts = [...huddles.roomRoles(record)].flatMap(([id, role]) =>
        role === "host" ? [id] : [],
      );

      if (hosts.length > 0 && !hosts.some((id) => input.ids.includes(id))) {
        const host = ctx.world().users.get(hosts[0] ?? 0);

        const message = `Promote another host before removing ${host?.name ?? "Unknown"}`;

        throw roomInvalid("userIds", message, message);
      }
    }

    const iconName = icon(input.iconName, record.room.iconName);

    const previousMembers = [...record.memberIds];

    record.room = {
      ...record.room,
      kind: input.type,
      name: input.name === undefined ? record.room.name : input.name,
      iconName,
      updatedAt: timestamp(Math.max(ctx.now(), Date.parse(record.room.updatedAt) + 1)),
      topic,
    };
    record.memberIds =
      input.type === "open"
        ? oldType === "open"
          ? record.memberIds
          : [...new Set([...record.memberIds, ...activeUsers().map((user) => user.id)])]
        : input.ids;
    huddles.reviseRoom(record, previousMembers);
    publish(record);

    return ok(result(record));
  };

  const remove = (roomId: number) => {
    const record = ctx.roomOr404(roomId);

    if (!canDelete(record)) throw forbidden();
    huddles.removeRoom(record);
    ctx.world().rooms.delete(roomId);
    ctx.publish([
      { topic: "user", type: "sidebar.row.removed", data: { roomId, refreshRoom: true } },
    ]);

    return ok({ roomId, deleted: true });
  };

  const leave = (roomId: number) => {
    const record = ctx.roomOr404(roomId);

    if (record.room.kind !== "direct") throw notFound();
    const previous = [...record.memberIds];

    record.memberIds = record.memberIds.filter((id) => id !== VIEWER_ID);
    huddles.reviseRoom(record, previous);
    const deleted = record.memberIds.length === 0;

    if (deleted) {
      huddles.removeRoom(record);
      ctx.world().rooms.delete(roomId);
    }

    ctx.publish([
      { topic: "user", type: "sidebar.row.removed", data: { roomId, refreshRoom: true } },
    ]);

    return ok({ roomId, deleted });
  };

  return {
    routes: [
      route("GET", /^\/rooms\/new$/, ({ query }) => {
        const type = roomType(query.get("type") ?? "open");

        if (type !== "direct" && !canCreate()) throw forbidden();

        return ok(form(type, null));
      }),
      route("GET", /^\/rooms\/(\d+)\/edit$/, (request) => {
        const record = ctx.roomOr404(firstId(request));

        return ok(form(record.room.kind, record));
      }),
      route("POST", /^\/rooms$/, ({ body }) => create(body)),
      route("PATCH", /^\/rooms\/(\d+)$/, (request) => update(firstId(request), request.body)),
      route("DELETE", /^\/rooms\/(\d+)$/, (request) => remove(firstId(request))),
      route("DELETE", /^\/rooms\/(\d+)\/membership$/, (request) => leave(firstId(request))),
    ],
  };
}
