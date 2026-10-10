/**
 * Direct messages: the people the viewer can message, opening (or reusing) the room with a
 * set of people, growing a group and renaming it. Every change reaches the viewer's sidebar as
 * `sidebar.row.upserted`; growing and renaming also leave a system note on the room.
 */
import type { DirectCandidateList } from "../../src/gen/DirectCandidateList.ts";
import type { RoomDetail } from "../../src/gen/RoomDetail.ts";
import type { User } from "../../src/gen/User.ts";
import { type MockResponse, ok, validation } from "../http.ts";
import { field, type Json, stringField } from "../json.ts";
import { type RoomRecord, VIEWER_ID } from "../seed.ts";
import { firstId, type Route, route, type S2Context } from "./context.ts";
import { iso, plainDraft, THREAD_NAME_LIMIT } from "./model.ts";

/** `DirectRoom::MAX_MEMBERS`: a direct room holds at most this many people, the viewer included. */
export const MAX_DIRECT_MEMBERS = 10;

/** A direct room's name is at most this long (the room name limit). */
export const DIRECT_NAME_LIMIT = THREAD_NAME_LIMIT;

/** `Array#to_sentence`. */
function sentence(words: readonly string[]): string {
  if (words.length <= 2) return words.join(" and ");

  return `${words.slice(0, -1).join(", ")}, and ${words.at(-1) ?? ""}`;
}

/** The directs module. */
export interface Directs {
  readonly routes: readonly Route[];
}

/** Creates the directs module. */
export function createDirects(ctx: S2Context): Directs {
  const byName = (a: User, b: User) => {
    const [left, right] = [a.name.toLowerCase(), b.name.toLowerCase()];

    return left < right ? -1 : left > right ? 1 : a.id - b.id;
  };

  const candidates = (): DirectCandidateList => {
    const world = ctx.world();

    const users = [...world.users.values()]
      .filter((user) => user.status === "active" && user.id !== VIEWER_ID)
      .sort(
        (a, b) => Number(world.stars.has(b.id)) - Number(world.stars.has(a.id)) || byName(a, b),
      );

    return {
      candidates: users.map((user) => ({
        userId: user.id,
        agent: user.role === "bot",
        starred: world.stars.has(user.id),
      })),
      users,
    };
  };

  /** The active people among `ids`, the viewer left out, once each, in the order given. */
  const activeOthers = (body: Json | undefined): number[] => {
    const raw = field(body, "userIds");
    const ids = Array.isArray(raw) ? raw.filter((id): id is number => Number.isInteger(id)) : [];
    const users = ctx.world().users;

    return [...new Set(ids)].filter((id) => id !== VIEWER_ID && users.get(id)?.status === "active");
  };

  const sameMembers = (record: RoomRecord, members: ReadonlySet<number>) =>
    record.memberIds.length === members.size && record.memberIds.every((id) => members.has(id));

  const upserted = (record: RoomRecord) => {
    ctx.publish([
      {
        topic: "user",
        type: "sidebar.row.upserted",
        data: { ...ctx.sidebarRow(record), refreshRoom: true },
      },
    ]);
  };

  const create = (body: Json | undefined): MockResponse => {
    const world = ctx.world();
    const others = activeOthers(body);

    if (others.length > MAX_DIRECT_MEMBERS - 1) {
      throw validation("userIds", `A direct message can have at most ${MAX_DIRECT_MEMBERS} people`);
    }

    const members = new Set([VIEWER_ID, ...others]);

    for (const record of world.rooms.values()) {
      if (record.room.kind !== "direct" || !sameMembers(record, members)) continue;

      if (record.membership.involvement === "invisible") {
        record.membership = { ...record.membership, involvement: "everything" };
        upserted(record);
      }

      return ok(ctx.sidebarRow(record));
    }

    const id = world.nextRoomId++;
    const createdAt = iso(ctx.now());

    const record: RoomRecord = {
      room: {
        id,
        kind: "direct",
        name: null,
        iconName: null,
        creatorId: VIEWER_ID,
        createdAt,
        updatedAt: createdAt,
        topic: null,
      },
      memberIds: [VIEWER_ID, ...others],
      membership: {
        id: world.nextMembershipId++,
        roomId: id,
        userId: VIEWER_ID,
        involvement: "everything",
        unreadAt: null,
        lastReadMessageId: null,
        roomCategoryId: null,
        favoritePosition: null,
        stageRole: null,
      },
      messages: [],
      mentionCount: 0,
    };

    world.rooms.set(id, record);
    upserted(record);

    return ok(ctx.sidebarRow(record), 201);
  };

  const groupOr422 = (roomId: number): RoomRecord => {
    const record = ctx.roomOr404(roomId);
    const capable = record.memberIds.length > 2 || (record.room.name ?? "").trim() !== "";

    if (record.room.kind !== "direct" || !capable) {
      throw validation("base", "Only a group conversation can be renamed or gain people");
    }

    return record;
  };

  const note = (record: RoomRecord, text: string) => {
    ctx.postToRoom(record, { ...plainDraft(VIEWER_ID, text, ctx.uuid()), systemNote: true });
  };

  const addMembers = (roomId: number, body: Json | undefined): RoomDetail => {
    const world = ctx.world();
    const record = groupOr422(roomId);
    const fresh = activeOthers(body).filter((id) => !record.memberIds.includes(id));

    // Nobody new is the model's no-op: the room as it is, and no event.
    if (fresh.length === 0) return ctx.roomDetail(roomId);

    if (record.memberIds.length + fresh.length > MAX_DIRECT_MEMBERS) {
      throw validation("userIds", `A group can have at most ${MAX_DIRECT_MEMBERS} people`);
    }

    record.memberIds.push(...fresh);

    const names = fresh.map((id) => world.users.get(id)?.name ?? "Someone");

    note(record, `added ${sentence(names)} to the group`);
    upserted(record);

    return ctx.roomDetail(roomId);
  };

  const rename = (roomId: number, body: Json | undefined): RoomDetail => {
    const record = groupOr422(roomId);
    const name = (stringField(body, "name") ?? "").trim();

    if ([...name].length > DIRECT_NAME_LIMIT) {
      throw validation("name", `Name is too long (maximum is ${DIRECT_NAME_LIMIT} characters)`);
    }

    const updatedAt = iso(Math.max(ctx.now(), Date.parse(record.room.updatedAt) + 1));

    record.room = { ...record.room, name: name === "" ? null : name, updatedAt };
    note(record, name === "" ? "cleared the group name" : `renamed the group to ${name}`);
    upserted(record);

    return ctx.roomDetail(roomId);
  };

  return {
    routes: [
      route("GET", /^\/directs\/candidates$/, () => ok(candidates())),
      route("POST", /^\/directs$/, (request) => create(request.body)),
      route("POST", /^\/directs\/(\d+)\/members$/, (request) =>
        ok(addMembers(firstId(request), request.body)),
      ),
      route("PATCH", /^\/directs\/(\d+)$/, (request) => ok(rename(firstId(request), request.body))),
    ],
  };
}
