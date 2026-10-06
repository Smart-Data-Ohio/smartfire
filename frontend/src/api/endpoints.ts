/**
 * One function per `/api/v1` endpoint. Each validates the reply with its pinned schema and returns
 * the wire value typed as the generated DTO (what the store keeps).
 */
import { Effect } from "effect";
import type { CreateMessage } from "../gen/CreateMessage.ts";
import type { Me } from "../gen/Me.ts";
import type { MessageDTO } from "../gen/MessageDTO.ts";
import type { MessagePage } from "../gen/MessagePage.ts";
import type { PresenceList } from "../gen/PresenceList.ts";
import type { ReadState } from "../gen/ReadState.ts";
import type { RoomDetail } from "../gen/RoomDetail.ts";
import type { Sidebar } from "../gen/Sidebar.ts";
import type { UserList } from "../gen/UserList.ts";
import { ApiClient, type ApiRequest } from "./client.ts";
import { BootReply } from "./schema/boot.ts";
import { Me as MeSchema } from "./schema/me.ts";
import { MessagePage as MessagePageSchema, MessageDTO as MessageSchema } from "./schema/message.ts";
import { PresenceList as PresenceListSchema } from "./schema/presence.ts";
import { ReadState as ReadStateSchema } from "./schema/read.ts";
import { RoomDetail as RoomDetailSchema } from "./schema/room.ts";
import { Sidebar as SidebarSchema } from "./schema/sidebar.ts";
import { UserList as UserListSchema } from "./schema/user.ts";
import { wire } from "./wire.ts";

/** Which page of a room's timeline: older than, newer than, or centred on a message; `null` is the newest page. */
export type PageCursor =
  | { readonly before: number }
  | { readonly after: number }
  | { readonly around: number }
  | null;

const get = (path: string, query?: Readonly<Record<string, string>>): ApiRequest =>
  query === undefined ? { method: "GET", path } : { method: "GET", path, query };

/** `GET /boot` (sign-in required): the boot JSON plus a fresh CSRF token. */
export const boot = Effect.fn("api.boot")(function* () {
  const client = yield* ApiClient;

  return yield* client.execute(get("/boot"), wire<BootReply>(BootReply));
});

/** `GET /me`: the signed-in person and their settings. */
export const me = Effect.fn("api.me")(function* () {
  const client = yield* ApiClient;

  return yield* client.execute(get("/me"), wire<Me>(MeSchema));
});

/** `GET /sidebar`: every room row, categories, and the people the rows name. */
export const sidebar = Effect.fn("api.sidebar")(function* () {
  const client = yield* ApiClient;

  return yield* client.execute(get("/sidebar"), wire<Sidebar>(SidebarSchema));
});

/** `GET /rooms/:id`: the room's header data and unread divider. */
export const room = Effect.fn("api.room")(function* (roomId: number) {
  const client = yield* ApiClient;

  return yield* client.execute(get(`/rooms/${roomId}`), wire<RoomDetail>(RoomDetailSchema));
});

export function cursorQuery(cursor: PageCursor): Readonly<Record<string, string>> | undefined {
  if (cursor === null) {
    return undefined;
  }

  if ("before" in cursor) {
    return { before: String(cursor.before) };
  }

  if ("after" in cursor) {
    return { after: String(cursor.after) };
  }

  return { around: String(cursor.around) };
}

/** `GET /rooms/:id/messages`: 40 root messages, oldest first. */
export const messages = Effect.fn("api.messages")(function* (roomId: number, cursor: PageCursor) {
  const client = yield* ApiClient;

  return yield* client.execute(
    get(`/rooms/${roomId}/messages`, cursorQuery(cursor)),
    wire<MessagePage>(MessagePageSchema),
  );
});

/** `POST /rooms/:id/messages`: idempotent on `clientMessageId`. */
export const createMessage = Effect.fn("api.createMessage")(function* (
  roomId: number,
  body: CreateMessage,
) {
  const client = yield* ApiClient;

  return yield* client.execute(
    { method: "POST", path: `/rooms/${roomId}/messages`, body },
    wire<MessageDTO>(MessageSchema),
  );
});

/** `POST /rooms/:id/read`: the room is read up to its newest message. */
export const markRead = Effect.fn("api.markRead")(function* (roomId: number) {
  const client = yield* ApiClient;

  return yield* client.execute(
    { method: "POST", path: `/rooms/${roomId}/read` },
    wire<ReadState>(ReadStateSchema),
  );
});

/** `DELETE /rooms/:id/read`: unread from `messageId` on. */
export const markUnread = Effect.fn("api.markUnread")(function* (
  roomId: number,
  messageId: number,
) {
  const client = yield* ApiClient;

  return yield* client.execute(
    { method: "DELETE", path: `/rooms/${roomId}/read`, body: { messageId } },
    wire<ReadState>(ReadStateSchema),
  );
});

/** `GET /users?ids=`: the named people. */
export const users = Effect.fn("api.users")(function* (ids: readonly number[]) {
  const client = yield* ApiClient;

  return yield* client.execute(
    get("/users", { ids: ids.join(",") }),
    wire<UserList>(UserListSchema),
  );
});

/** `GET /presence?ids=`: the named people's presence and status lines. */
export const presence = Effect.fn("api.presence")(function* (ids: readonly number[]) {
  const client = yield* ApiClient;

  return yield* client.execute(
    get("/presence", { ids: ids.join(",") }),
    wire<PresenceList>(PresenceListSchema),
  );
});
