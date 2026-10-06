/**
 * The live store's data model: the wire DTOs exactly as ts-rs generates them (src/gen), already
 * validated by the pinned Effect Schemas in src/api, plus the client-only shapes below. Timestamps
 * stay RFC 3339 strings: the server's fixed `YYYY-MM-DDTHH:MM:SS.mmmZ` form sorts as text.
 */
import type { MessageDTO } from "../gen/MessageDTO.ts";
import type { RoomDetail } from "../gen/RoomDetail.ts";
import type { SidebarRow } from "../gen/SidebarRow.ts";

export type { Me } from "../gen/Me.ts";

export type { MessageDTO } from "../gen/MessageDTO.ts";

export type { MessagePage } from "../gen/MessagePage.ts";

export type { Presence } from "../gen/Presence.ts";

export type { RoomCategory } from "../gen/RoomCategory.ts";

export type { RoomDetail } from "../gen/RoomDetail.ts";

export type { RoomKind } from "../gen/RoomKind.ts";

export type { Sidebar } from "../gen/Sidebar.ts";

export type { SidebarRow } from "../gen/SidebarRow.ts";

export type { SyncEvent } from "../gen/SyncEvent.ts";

export type { User } from "../gen/User.ts";

export type { UserPresence } from "../gen/UserPresence.ts";

/**
 * The boot JSON the Rust shell inlines (`crates/spa`'s `Boot`): what the app knows before its
 * first request.
 */
export interface Boot {
  readonly user: { readonly id: number; readonly name: string; readonly avatarUrl: string };
  readonly account: { readonly name: string | null };
  readonly theme: "system" | "light" | "dark";
  readonly textSize: "smaller" | "small" | "default" | "large" | "larger";
  readonly cableUrl: string;
  readonly version: string;
  readonly revision: string | null;
}

/** The sync socket's state, for the connection banner. */
export type ConnectionStatus = "connecting" | "online" | "reconnecting" | "offline";

/** A message written here and not yet confirmed by the server. */
export interface PendingMessage {
  readonly clientMessageId: string;
  readonly roomId: number;
  readonly creatorId: number;
  readonly markdownSource: string;
  /** Local clock, RFC 3339: pending rows sort after every confirmed row by this. */
  readonly createdAt: string;
  readonly state: "sending" | "failed";
  /** Why it failed, for the "Couldn't send" line. */
  readonly error: string | null;
}

export type LoadStatus = "idle" | "loading" | "ready" | "error";

/** One room's loaded window of its root timeline. */
export interface Timeline {
  /** Confirmed message ids, ordered by `(createdAt, id)`. */
  readonly ids: readonly number[];
  /** Cursor for the next older page; `null` at the start of the room. */
  readonly before: number | null;
  /** Cursor for the next newer page; `null` when the window reaches the present. */
  readonly after: number | null;
  readonly status: LoadStatus;
  /** A page in one direction is on its way. */
  readonly loadingOlder: boolean;
  readonly loadingNewer: boolean;
  /** Where the unread divider sits for this visit; fixed until the room is left. */
  readonly unreadFromId: number | null;
  readonly unreadCount: number;
  /** Bumped when a resync replaces the window, so the list can jump instead of animating. */
  readonly generation: number;
}

/** The room view's data: the detail plus its timeline. */
export interface RoomState {
  readonly detail: RoomDetail | null;
  readonly status: LoadStatus;
  readonly error: string | null;
}

export type TimelineMessage = MessageDTO;

export type SidebarRowById = Readonly<Record<number, SidebarRow>>;
