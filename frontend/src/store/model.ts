/**
 * The live store's data model: the wire DTOs exactly as ts-rs generates them (src/gen), already
 * validated by the pinned Effect Schemas in src/api, plus the client-only shapes below. Timestamps
 * stay RFC 3339 strings: the server's fixed `YYYY-MM-DDTHH:MM:SS.mmmZ` form sorts as text.
 */
import type { MessageDTO } from "../gen/MessageDTO.ts";
import type { OpenRoomPreview } from "../gen/OpenRoomPreview.ts";
import type { RoomDetail } from "../gen/RoomDetail.ts";
import type { SidebarRow } from "../gen/SidebarRow.ts";
import type { ThreadFilter } from "../gen/ThreadFilter.ts";
import type { ThreadPermissions } from "../gen/ThreadPermissions.ts";

import type { WorkDetail } from "../gen/WorkDetail.ts";
import type { WorkFacts } from "../gen/WorkFacts.ts";

export type { Me } from "../gen/Me.ts";

export type { Membership } from "../gen/Membership.ts";

export type { MessageDTO } from "../gen/MessageDTO.ts";

export type { MessagePage } from "../gen/MessagePage.ts";

export type { OpenRoomPreview } from "../gen/OpenRoomPreview.ts";

export type { Presence } from "../gen/Presence.ts";

export type { RoomCategory } from "../gen/RoomCategory.ts";

export type { RoomDetail } from "../gen/RoomDetail.ts";

export type { RoomKind } from "../gen/RoomKind.ts";

export type { Sidebar } from "../gen/Sidebar.ts";

export type { SidebarRow } from "../gen/SidebarRow.ts";

export type { SyncEvent } from "../gen/SyncEvent.ts";

export type { Thread } from "../gen/Thread.ts";

export type { ThreadFilter } from "../gen/ThreadFilter.ts";

export type { ThreadMembership } from "../gen/ThreadMembership.ts";

export type { ThreadPermissions } from "../gen/ThreadPermissions.ts";

export type { User } from "../gen/User.ts";

export type { UserPresence } from "../gen/UserPresence.ts";

/**
 * The boot JSON the Rust shell inlines (`crates/spa`'s `Boot`): what the app knows before its
 * first request.
 */
export interface Boot {
  readonly user: { readonly id: number; readonly name: string; readonly avatarUrl: string };
  readonly customStyles: string | null;
  /**
   * The workspace: its name (`null` only before first run), and its logo and banner images when
   * an administrator has uploaded them (`null` falls back to initials and the plain header). An
   * animated image also has a still of its first frame, shown at rest and under reduced motion.
   */
  readonly account: {
    readonly name: string | null;
    readonly logoUrl: string | null;
    readonly logoStillUrl: string | null;
    readonly bannerUrl: string | null;
    readonly bannerStillUrl: string | null;
    readonly uploadLimitBytes: number;
  };
  readonly theme: "system" | "light" | "dark";
  readonly textSize: "smaller" | "small" | "default" | "large" | "larger";
  readonly cableUrl: string;
  readonly serviceWorkerUrl: string | null;
  readonly version: string;
  readonly revision: string | null;
  /**
   * The sign-in or integration callback's notice or alert that sent you here, shown once as a
   * toast; the shell consumes it from the session when it renders the page.
   */
  readonly flash?: { readonly kind: "notice" | "alert"; readonly message: string } | null;
}

/** The sync socket's state, for the connection banner. */
export type ConnectionStatus = "connecting" | "online" | "reconnecting" | "offline";

/** What a pending row shows for the file it's sending (the upload already finished). */
export interface PendingAttachment {
  readonly filename: string;
  readonly contentType: string;
  readonly byteSize: number;
  /** A local `blob:` URL for an image's preview, or `null`. */
  readonly previewUrl: string | null;
}

/** A message written here and not yet confirmed by the server. */
export interface PendingMessage {
  readonly clientMessageId: string;
  readonly roomId: number;
  /** The thread it replies in; `null` on the room's root timeline. */
  readonly threadId: number | null;
  /** A finished direct upload's signed id, posted as the message's file. */
  readonly attachmentSignedId: string | null;
  readonly attachment: PendingAttachment | null;
  /**
   * Several finished uploads posted together as one message (`attachmentSignedIds`), in tray
   * order; absent for none or the legacy single file above.
   */
  readonly attachmentSignedIds?: readonly string[];
  /** What the pending row shows for those files, in the same order. */
  readonly attachments?: readonly PendingAttachment[];
  /** Drive file ids pinned on the pending message; absent when there are none. */
  readonly driveFileIds?: readonly string[];
  /** The message it replies to (inline, on the same timeline), or `null`. */
  readonly replyToMessageId: number | null;
  /** Whether the replied-to author is notified; `null` when it isn't a reply. */
  readonly replyNotifyAuthor: boolean | null;
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
  /**
   * Identity of the latest page request in each direction. A replace bumps both. A page, its
   * error, or its interrupt applies only while the id it captured is still current.
   */
  readonly olderRequest: number;
  readonly newerRequest: number;
  /** Where the unread divider sits for this visit; fixed until the room is left. */
  readonly unreadFromId: number | null;
  readonly unreadCount: number;
  /** Bumped when a resync replaces the window, so the list can jump instead of animating. */
  readonly generation: number;
  /**
   * While a fresh window loads, the live messages that arrived meanwhile (the page may have been
   * read before them); merged in when it lands. `null` when no fresh window is on its way.
   */
  readonly arrived: readonly number[] | null;
}

/** The room view's data: the detail plus its timeline. */
export interface RoomState {
  readonly detail: RoomDetail | null;
  readonly status: LoadStatus;
  readonly error: string | null;
  /** An open room the viewer may join. Set only while they are not a member. */
  readonly preview: OpenRoomPreview | null;
}

export type TimelineMessage = MessageDTO;

/** A thread pane's header data: loaded from `GET /threads/:id`, kept fresh by events. */
export interface ThreadPaneState {
  readonly status: LoadStatus;
  readonly error: string | null;
  readonly permissions: ThreadPermissions | null;
  readonly work: WorkDetail | null;
  /** Facts at the last detail load, for detecting work changes through sync. */
  readonly workFacts: WorkFacts | null;
}

/** One room's thread list (the Threads pane), for one filter. */
export interface RoomThreadList {
  readonly filter: ThreadFilter;
  /** Thread ids, most recently active first. */
  readonly ids: readonly number[];
  readonly status: LoadStatus;
}

export type SidebarRowById = Readonly<Record<number, SidebarRow>>;
