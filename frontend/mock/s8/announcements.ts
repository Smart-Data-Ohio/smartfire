/**
 * Announcement rooms: the flag, follows and publishing. Rules match the API: the creator and
 * administrators flag a room (never a direct one) and manage what it follows; the author or an
 * administrator publishes a message in an announcement room, which copies it into every room
 * that follows. Copies are delivered at once here (the server does it in a job) and aren't kept
 * in step with later edits.
 */
import type { AnnouncementFollow } from "../../src/gen/AnnouncementFollow.ts";
import type { AnnouncementFollows } from "../../src/gen/AnnouncementFollows.ts";
import type { MessageDTO } from "../../src/gen/MessageDTO.ts";
import { forbidden, noContent, notFound, ok, validation } from "../http.ts";
import { field, intField, isBoolean, isRecord, type Json } from "../json.ts";
import { firstId, type Route, route, type S2Context } from "../s2/context.ts";
import { locate, topicOf } from "../s2/model.ts";
import { type RoomRecord, timestamp, VIEWER_ID, type World } from "../seed.ts";

const follows = new WeakMap<World, AnnouncementFollow[]>();

function followsFor(world: World): AnnouncementFollow[] {
  const existing = follows.get(world);

  if (existing !== undefined) return existing;

  const created: AnnouncementFollow[] = [];

  follows.set(world, created);

  return created;
}

/** A 422 as `record_invalid` words it: the humanized field, then the reason. */
function invalid(fieldName: string, human: string, reason: string) {
  return validation(fieldName, reason, `${human} ${reason}`);
}

interface Announcements {
  readonly routes: readonly Route[];
}

/** Routes for `/rooms/:id/announcement`, `/rooms/:id/announcement_follows` and publishing. */
export function createAnnouncements(ctx: S2Context): Announcements {
  const isAdmin = () => ctx.world().users.get(VIEWER_ID)?.role === "administrator";
  const canAdminister = (record: RoomRecord) => isAdmin() || record.room.creatorId === VIEWER_ID;
  const all = () => followsFor(ctx.world());

  const visible = (roomId: number) => {
    const record = ctx.world().rooms.get(roomId);

    return record?.memberIds.includes(VIEWER_ID) === true;
  };

  const listed = (record: RoomRecord): AnnouncementFollows => {
    const roomId = record.room.id;

    return {
      followers: canAdminister(record)
        ? all().filter((each) => each.sourceRoomId === roomId && visible(each.targetRoomId))
        : [],
      following: all().filter((each) => each.targetRoomId === roomId && visible(each.sourceRoomId)),
    };
  };

  /** The room this room's follows land in, if the viewer may manage it. */
  const managedTarget = (targetId: number) => {
    const record = ctx.roomOr404(targetId);

    if (!canAdminister(record)) throw forbidden("Not allowed");

    return record;
  };

  const setFlag = (roomId: number, body: Json | undefined) => {
    const record = ctx.roomOr404(roomId);

    if (!canAdminister(record)) throw forbidden("Not allowed");

    const value = field(body, "announcement");

    if (!isRecord(body) || !isBoolean(value)) {
      throw invalid("announcement", "Announcement", "must be true or false");
    }

    if (record.room.kind === "direct") {
      throw invalid("announcement", "Announcement", "is not supported for direct messages");
    }

    if (record.room.announcement !== value) {
      record.room = { ...record.room, announcement: value };
    }

    ctx.publish([
      {
        topic: "user",
        type: "sidebar.row.upserted",
        data: { ...ctx.sidebarRow(record), refreshRoom: true },
      },
    ]);

    return ok(record.room);
  };

  // Would following `source` from `target` close a loop: does `source` already hear from `target`?
  const cycle = (source: number, target: number) => {
    const seen = new Set<number>([target]);
    const queue = [target];

    for (let current = queue.shift(); current !== undefined; current = queue.shift()) {
      if (current === source) return true;

      for (const each of all()) {
        if (each.sourceRoomId === current && !seen.has(each.targetRoomId)) {
          seen.add(each.targetRoomId);
          queue.push(each.targetRoomId);
        }
      }
    }

    return false;
  };

  const follow = (sourceId: number, body: Json | undefined) => {
    const source = ctx.roomOr404(sourceId);
    const targetId = intField(body, "targetRoomId");

    if (targetId === null) throw notFound();

    const target = managedTarget(targetId);

    if (source.room.announcement !== true) {
      throw invalid("sourceRoomId", "Source room", "must be an announcement room");
    }

    if (target.room.kind === "direct") {
      throw invalid("targetRoomId", "Target room", "cannot be a direct message");
    }

    if (target.room.kind === "board") {
      throw invalid(
        "targetRoomId",
        "Target room",
        "must show plain timeline messages; boards are not supported",
      );
    }

    if (cycle(source.room.id, target.room.id)) {
      throw invalid("targetRoomId", "Target room", "would create an announcement cycle");
    }

    const exists = all().some(
      (each) => each.sourceRoomId === source.room.id && each.targetRoomId === target.room.id,
    );

    if (!exists) {
      all().push({
        sourceRoomId: source.room.id,
        targetRoomId: target.room.id,
        creatorId: VIEWER_ID,
        createdAt: timestamp(ctx.now()),
      });
    }

    // Like the server, the reply is the source room's list.
    return ok(listed(source));
  };

  const unfollow = (sourceId: number, targetId: number) => {
    managedTarget(targetId);

    const list = all();

    const index = list.findIndex(
      (each) => each.sourceRoomId === sourceId && each.targetRoomId === targetId,
    );

    if (index !== -1) list.splice(index, 1);

    return noContent();
  };

  const deliver = (source: MessageDTO, record: RoomRecord) => {
    const markdown =
      source.markdownSource ?? source.bodyHtml.replace(/<br\s*\/?>/g, "\n").replace(/<[^>]+>/g, "");

    const copy = ctx.postToRoom(record, {
      creatorId: source.creatorId,
      markdown,
      clientMessageId: ctx.uuid(),
      replyToMessageId: null,
      streaming: false,
      attachment: null,
      action: false,
      systemNote: false,
      forward: null,
    });

    const sourceRoom = ctx.world().rooms.get(source.roomId);
    const index = record.messages.findIndex((message) => message.id === copy.id);

    const attributed: MessageDTO = {
      ...copy,
      announcementSource: {
        roomId: source.roomId,
        messageId: source.id,
        roomName: sourceRoom?.room.name ?? null,
      },
    };

    if (index !== -1) record.messages[index] = attributed;

    ctx.publish([{ topic: topicOf(attributed), type: "message.updated", data: attributed }]);
  };

  const publish = (messageId: number) => {
    const location = locate(ctx.world(), messageId);

    if (location === null || !location.room.memberIds.includes(VIEWER_ID)) {
      throw notFound("Message not found");
    }

    const message = location.message;

    if (message.creatorId !== VIEWER_ID && !isAdmin()) {
      throw forbidden("Only the author or a workspace administrator can publish");
    }

    if (message.published === true) return ok(message);

    if (location.room.room.announcement !== true) {
      throw invalid("message", "Message", "must be in an announcement room");
    }

    if (message.systemNote || message.streaming || message.announcementSource != null) {
      throw invalid(
        "message",
        "Message",
        "cannot publish a copy, system note or unfinished message",
      );
    }

    const published: MessageDTO = { ...message, published: true };

    location.list[location.index] = published;
    ctx.publish([{ topic: topicOf(published), type: "message.updated", data: published }]);

    for (const each of all()) {
      if (each.sourceRoomId !== location.room.room.id) continue;

      const target = ctx.world().rooms.get(each.targetRoomId);

      if (target === undefined) continue;

      if (target.room.kind === "direct" || target.room.kind === "board") continue;

      deliver(published, target);
    }

    return ok(published);
  };

  return {
    routes: [
      route("PUT", /^\/rooms\/(\d+)\/announcement$/, (request) =>
        setFlag(firstId(request), request.body),
      ),
      route("GET", /^\/rooms\/(\d+)\/announcement_follows$/, (request) =>
        ok(listed(ctx.roomOr404(firstId(request)))),
      ),
      route("POST", /^\/rooms\/(\d+)\/announcement_follows$/, (request) =>
        follow(firstId(request), request.body),
      ),
      route("DELETE", /^\/rooms\/(\d+)\/announcement_follows\/(\d+)$/, (request) =>
        unfollow(request.ids[0] ?? 0, request.ids[1] ?? 0),
      ),
      route("POST", /^\/messages\/(\d+)\/publish$/, (request) => publish(firstId(request))),
    ],
  };
}
