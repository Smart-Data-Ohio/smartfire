import { Schema } from "effect";
import type { MarkUnread as GeneratedMarkUnread } from "../../gen/MarkUnread.ts";
import type { ReadState as GeneratedReadState } from "../../gen/ReadState.ts";
import type { RoomRead as GeneratedRoomRead } from "../../gen/RoomRead.ts";
import type { RoomUnread as GeneratedRoomUnread } from "../../gen/RoomUnread.ts";
import { MessageId, RoomId } from "./ids.ts";
import type { Assert, Pinned } from "./pin.ts";

/** The reply to `POST` / `DELETE /api/v1/rooms/:id/read`. */
export const ReadState = Schema.Struct({
  roomId: RoomId,
  unread: Schema.Boolean,
  firstUnreadMessageId: Schema.NullOr(MessageId),
  /** Root messages from `firstUnreadMessageId` on, as `SidebarRow.unreadCount` counts them. */
  unreadCount: Schema.Int,
});

export type ReadState = typeof ReadState.Type;

export type ReadStatePin = Assert<Pinned<typeof ReadState, GeneratedReadState>>;

/** The body of `DELETE /api/v1/rooms/:id/read`: unread from this message on. */
export const MarkUnread = Schema.Struct({ messageId: MessageId });

export type MarkUnreadPin = Assert<Pinned<typeof MarkUnread, GeneratedMarkUnread>>;

/** The `room.unread` event's data. `messageId` is `null` when the person marked it unread. */
export const RoomUnread = Schema.Struct({
  roomId: RoomId,
  messageId: Schema.NullOr(MessageId),
  mentioned: Schema.Boolean,
});

export type RoomUnread = typeof RoomUnread.Type;

export type RoomUnreadPin = Assert<Pinned<typeof RoomUnread, GeneratedRoomUnread>>;

/** The `room.read` event's data. */
export const RoomRead = Schema.Struct({ roomId: RoomId });

export type RoomReadPin = Assert<Pinned<typeof RoomRead, GeneratedRoomRead>>;
