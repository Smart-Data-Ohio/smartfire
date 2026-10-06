/**
 * The room's side panes (members and files), starring people, and the quick switcher's
 * one-shot index.
 */
import type { FileList } from "../../src/gen/FileList.ts";
import type { FileType } from "../../src/gen/FileType.ts";
import type { MemberList } from "../../src/gen/MemberList.ts";
import type { RoomFile } from "../../src/gen/RoomFile.ts";
import type { StarState } from "../../src/gen/StarState.ts";
import type { Switcher } from "../../src/gen/Switcher.ts";
import type { SwitcherRoomKind } from "../../src/gen/SwitcherRoomKind.ts";
import type { User } from "../../src/gen/User.ts";
import { notFound, ok, validation } from "../http.ts";
import { type RoomRecord, VIEWER_ID } from "../seed.ts";
import { firstId, type Route, route, type S2Context } from "./context.ts";

/** Files per page (`RoomFiles::PER_PAGE`). */
export const FILES_PER_PAGE = 30;

/** The files pane stops at this page. */
export const MAX_FILE_PAGE = 20;

/** The switcher lists this many recent threads. */
export const SWITCHER_THREADS = 15;

const FILE_TYPES: readonly FileType[] = ["all", "images", "videos", "documents", "other"];

/** `RoomFiles`'s documents group. */
function isDocument(contentType: string): boolean {
  return (
    contentType === "application/pdf" ||
    contentType.startsWith("text/") ||
    contentType === "application/msword" ||
    contentType.startsWith("application/vnd.ms-") ||
    contentType.includes("openxmlformats") ||
    contentType.startsWith("application/vnd.oasis.opendocument")
  );
}

/** Which `FileType` group a content type falls in (never `all`). */
export function fileGroup(contentType: string): FileType {
  if (contentType.startsWith("image/")) return "images";

  if (contentType.startsWith("video/")) return "videos";

  return isDocument(contentType) ? "documents" : "other";
}

const byName = (a: User, b: User) => {
  const [left, right] = [a.name.toLowerCase(), b.name.toLowerCase()];

  return left < right ? -1 : left > right ? 1 : a.id - b.id;
};

/** `Array#to_sentence`, as direct room names are joined. */
function sentence(words: readonly string[]): string {
  if (words.length <= 2) return words.join(" and ");

  return `${words.slice(0, -1).join(", ")}, and ${words.at(-1) ?? ""}`;
}

/** The panes module. */
export interface Panes {
  readonly routes: readonly Route[];
}

/** Creates the panes module. */
export function createPanes(ctx: S2Context): Panes {
  const members = (roomId: number): MemberList => {
    const world = ctx.world();
    const record = ctx.roomOr404(roomId);

    const users = ctx
      .usersFor(record.memberIds)
      .filter((user) => user.status === "active")
      .sort(byName);

    return {
      members: users.map((user) => {
        const presence = world.presence.get(user.id);

        return {
          userId: user.id,
          presence: presence?.presence ?? (user.role === "bot" ? "online" : "offline"),
          statusText: presence?.statusText ?? (user.role === "bot" ? "Idle" : null),
          starred: user.id !== VIEWER_ID && world.stars.has(user.id),
        };
      }),
      users,
    };
  };

  const star = (userId: number, starred: boolean): StarState => {
    const world = ctx.world();
    const user = world.users.get(userId);

    if (user === undefined) throw notFound("User not found");

    if (userId === VIEWER_ID) throw validation("userId", "You can't star yourself");

    if (user.status !== "active" || user.role === "bot") {
      throw validation("userId", "Only active people can be starred");
    }

    if (starred) {
      world.stars.add(userId);
    } else {
      world.stars.delete(userId);
    }

    return { userId, starred };
  };

  const files = (roomId: number, query: URLSearchParams): FileList => {
    const world = ctx.world();
    const record = ctx.roomOr404(roomId);
    const rawType = query.get("type") ?? "all";
    const type = FILE_TYPES.find((candidate) => candidate === rawType);

    if (type === undefined) throw validation("type", "Type is not included in the list");

    const needle = (query.get("filename") ?? "").trim().toLowerCase();
    const requested = Number.parseInt(query.get("page") ?? "1", 10);
    const page = Math.min(MAX_FILE_PAGE, Math.max(1, Number.isNaN(requested) ? 1 : requested));
    const all: RoomFile[] = [];

    const replies = [...world.threads.values()]
      .filter((thread) => thread.roomId === roomId)
      .flatMap((thread) => thread.messages);

    for (const message of [...record.messages, ...replies]) {
      const attachment = message.attachment;

      if (attachment === null) continue;

      if (type !== "all" && fileGroup(attachment.contentType) !== type) continue;

      if (needle !== "" && !attachment.filename.toLowerCase().includes(needle)) continue;

      all.push({
        messageId: message.id,
        threadId: message.threadId,
        creatorId: message.creatorId,
        attachment,
        createdAt: message.createdAt,
      });
    }

    all.sort(
      (a, b) => Date.parse(b.createdAt) - Date.parse(a.createdAt) || b.messageId - a.messageId,
    );

    const start = (page - 1) * FILES_PER_PAGE;
    const rows = all.slice(start, start + FILES_PER_PAGE);
    const more = start + FILES_PER_PAGE < all.length && page < MAX_FILE_PAGE;

    return {
      files: rows,
      users: ctx.usersFor(rows.map((row) => row.creatorId)),
      nextPage: more ? page + 1 : null,
    };
  };

  const switcherKind = (record: RoomRecord): SwitcherRoomKind => {
    switch (record.room.kind) {
      case "open":
      case "closed":
        return "channel";
      case "direct":
        return record.memberIds.length > 2 ? "group" : "dm";
      default:
        return record.room.kind;
    }
  };

  const switcherName = (record: RoomRecord): string => {
    if (record.room.name !== null) return record.room.name;

    const others = ctx
      .usersFor(record.memberIds.filter((id) => id !== VIEWER_ID))
      .sort(byName)
      .map((user) => user.name);

    return others.length === 0 ? (ctx.world().users.get(VIEWER_ID)?.name ?? "") : sentence(others);
  };

  const switcher = (): Switcher => {
    const world = ctx.world();

    const mine = [...world.rooms.values()]
      .filter((record) => record.membership.involvement !== "invisible")
      .sort((a, b) => {
        // `LOWER(rooms.name)`: unnamed direct rooms sort first, as NULL does.
        const [left, right] = [
          (a.room.name ?? "").toLowerCase(),
          (b.room.name ?? "").toLowerCase(),
        ];

        return left < right ? -1 : left > right ? 1 : a.room.id - b.room.id;
      });

    const people = [...world.users.values()]
      .filter((user) => user.status === "active" && user.role !== "bot" && user.id !== VIEWER_ID)
      .sort(byName);

    const directWith = (userId: number) =>
      mine.find(
        (record) =>
          record.room.kind === "direct" &&
          record.memberIds.length === 2 &&
          record.memberIds.includes(userId),
      )?.room.id ?? null;

    const roomIds = new Set(mine.map((record) => record.room.id));

    const threads = [...world.threads.values()]
      .filter((thread) => roomIds.has(thread.roomId))
      .sort((a, b) => Date.parse(b.lastActivityAt) - Date.parse(a.lastActivityAt) || b.id - a.id)
      .slice(0, SWITCHER_THREADS);

    return {
      rooms: mine.map((record) => ({
        roomId: record.room.id,
        name: switcherName(record),
        kind: switcherKind(record),
        iconName: record.room.iconName,
        unread: record.membership.unreadAt !== null,
        muted: record.membership.involvement === "muted",
        favorite: record.membership.favoritePosition !== null,
      })),
      people: people.map((user) => ({ userId: user.id, directRoomId: directWith(user.id) })),
      threads: threads.map((thread) => ({
        threadId: thread.id,
        name: thread.name,
        roomId: thread.roomId,
        roomName: world.rooms.get(thread.roomId)?.room.name ?? null,
      })),
      users: people,
    };
  };

  return {
    routes: [
      route("GET", /^\/rooms\/(\d+)\/members$/, (request) => ok(members(firstId(request)))),
      route("PUT", /^\/users\/(\d+)\/star$/, (request) => ok(star(firstId(request), true))),
      route("DELETE", /^\/users\/(\d+)\/star$/, (request) => ok(star(firstId(request), false))),
      route("GET", /^\/rooms\/(\d+)\/files$/, (request) =>
        ok(files(firstId(request), request.query)),
      ),
      route("GET", /^\/switcher$/, () => ok(switcher())),
    ],
  };
}
