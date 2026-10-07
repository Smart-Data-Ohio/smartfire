/**
 * What the mock keeps for S2 beyond the S1 rooms and messages (threads, pins, saved items,
 * stars, scheduled messages, uploaded blobs) and the pure helpers that turn it into wire DTOs.
 */
import type { Attachment } from "../../src/gen/Attachment.ts";
import type { DoNotDisturb } from "../../src/gen/DoNotDisturb.ts";
import type { MessageDTO } from "../../src/gen/MessageDTO.ts";
import type { MessagePage } from "../../src/gen/MessagePage.ts";
import type { OutOfOffice } from "../../src/gen/OutOfOffice.ts";
import type { Pin } from "../../src/gen/Pin.ts";
import type { SavedItem } from "../../src/gen/SavedItem.ts";
import type { ScheduledMessage } from "../../src/gen/ScheduledMessage.ts";
import type { Thread } from "../../src/gen/Thread.ts";
import type { ThreadIndicator } from "../../src/gen/ThreadIndicator.ts";
import type { ThreadMembership } from "../../src/gen/ThreadMembership.ts";
import type { ThreadStatus } from "../../src/gen/ThreadStatus.ts";
import type { User } from "../../src/gen/User.ts";
import { notFound, validation } from "../http.ts";
import { escapeHtml, type Mentionable, renderMarkdown } from "../markdown.ts";
import type { RoomRecord, World } from "../seed.ts";

/** Messages per page, as `Message::PAGE_SIZE`. */
export const PAGE_SIZE = 40;

/** `Message::SOURCE_LIMIT`. */
export const SOURCE_LIMIT = 50_000;

/** `MessagePin::MAX_PER_ROOM`. */
export const MAX_PINS_PER_ROOM = 50;

/** `ChannelThread::NAME_LIMIT`. */
export const THREAD_NAME_LIMIT = 100;

/** New threads go quiet after a day without replies. */
export const DEFAULT_AUTO_ARCHIVE_MINUTES = 1440;

/** A channel thread and the replies in it. */
export interface ThreadRecord {
  readonly id: number;
  readonly roomId: number;
  /** `null` once the parent message is deleted. */
  parentMessageId: number | null;
  readonly creatorId: number;
  name: string;
  /** `closed_at` is set. */
  closed: boolean;
  /** `locked_at` is set. */
  locked: boolean;
  lastActivityAt: string;
  autoArchiveAfterMinutes: number;
  readonly createdAt: string;
  /** The replies, ascending by id. */
  readonly messages: MessageDTO[];
  /** Everyone with a thread membership, the viewer included when they have one. */
  readonly memberIds: Set<number>;
  /** The viewer's membership; `null` when they never touched the thread. */
  viewerMembership: ThreadMembership | null;
}

/** One stored blob: what `POST /uploads` declared and, once `PUT`, the bytes. */
export interface BlobRecord {
  readonly id: number;
  readonly signedId: string;
  /** The `/rails/active_storage/disk/:token` segment. */
  readonly token: string;
  readonly filename: string;
  readonly contentType: string;
  readonly byteSize: number;
  /** Base64 MD5, checked on `PUT`. */
  readonly checksum: string;
  bytes: Uint8Array | null;
  width: number | null;
  height: number | null;
  /** The upload URL stops working at this instant (ms). */
  readonly uploadExpiresAt: number;
}

/** A `message_pins` row with the room it counts against. */
export interface PinRecord {
  readonly roomId: number;
  readonly pin: Pin;
}

/** An agent command registered in a room (`agent_slash_commands`). */
export interface AgentCommand {
  readonly name: string;
  readonly description: string | null;
  readonly agentName: string;
}

/** The S2 part of the world, mutated in place by the S2 modules. */
export interface S2World {
  readonly threads: Map<number, ThreadRecord>;
  nextThreadId: number;
  /** By message id. */
  readonly pins: Map<number, PinRecord>;
  /** The viewer's saved items, by message id. */
  readonly saved: Map<number, SavedItem>;
  nextSavedId: number;
  /** People the viewer starred. */
  readonly stars: Set<number>;
  readonly scheduled: Map<number, ScheduledMessage>;
  nextScheduledId: number;
  /** By signed id. */
  readonly blobs: Map<string, BlobRecord>;
  nextBlobId: number;
  nextBoostId: number;
  nextRoomId: number;
  nextMembershipId: number;
  /** By room id. */
  readonly agentCommands: Map<number, readonly AgentCommand[]>;
  /** The room-scoped `clientMessageId` index (`Message::find_duplicate`). */
  readonly sentByClientId: Map<string, MessageDTO>;
  /** The viewer's Do Not Disturb, as `/dnd` leaves it. */
  doNotDisturb: DoNotDisturb;
  /** The viewer's out of office, as `/ooo` leaves it. */
  outOfOffice: OutOfOffice | null;
}

/** A fresh, empty S2 world. */
export function emptyS2World(): S2World {
  return {
    threads: new Map(),
    nextThreadId: 1,
    pins: new Map(),
    saved: new Map(),
    nextSavedId: 1,
    stars: new Set(),
    scheduled: new Map(),
    nextScheduledId: 1,
    blobs: new Map(),
    nextBlobId: 1,
    nextBoostId: 1,
    nextRoomId: 100,
    nextMembershipId: 1000,
    agentCommands: new Map(),
    sentByClientId: new Map(),
    doNotDisturb: { enabled: false, until: null },
    outOfOffice: null,
  };
}

/** Where a forwarded message came from. */
export interface ForwardOrigin {
  readonly messageId: number;
  readonly at: string;
  readonly note: string | null;
  /** The frozen copy of the source's body. */
  readonly bodyHtml: string;
}

/** Everything needed to make a message, before it has an id. */
export interface MessageDraft {
  readonly creatorId: number;
  readonly markdown: string;
  readonly clientMessageId: string;
  readonly replyToMessageId: number | null;
  readonly streaming: boolean;
  readonly attachment: Attachment | null;
  readonly action: boolean;
  readonly systemNote: boolean;
  readonly forward: ForwardOrigin | null;
}

/** A plain draft: no reply, attachment or forward. */
export function plainDraft(
  creatorId: number,
  markdown: string,
  clientMessageId: string,
): MessageDraft {
  return {
    creatorId,
    markdown,
    clientMessageId,
    replyToMessageId: null,
    streaming: false,
    attachment: null,
    action: false,
    systemNote: false,
    forward: null,
  };
}

/** The wire DTO for a new message. */
export function buildMessage(
  id: number,
  roomId: number,
  threadId: number | null,
  draft: MessageDraft,
  createdAt: string,
  people: readonly Mentionable[],
): MessageDTO {
  return {
    id,
    roomId,
    threadId,
    creatorId: draft.creatorId,
    clientMessageId: draft.clientMessageId,
    bodyHtml: draft.systemNote
      ? escapeHtml(draft.markdown)
      : (draft.forward?.bodyHtml ?? renderMarkdown(draft.markdown, people)),
    // A system note's body is plain escaped text with no Markdown behind it.
    markdownSource: draft.systemNote ? null : draft.markdown,
    systemNote: draft.systemNote,
    action: draft.action,
    streaming: draft.streaming,
    embedsSuppressed: false,
    replyToMessageId: draft.replyToMessageId,
    forwardedFromMessageId: draft.forward?.messageId ?? null,
    forwardedAt: draft.forward?.at ?? null,
    forwardNote: draft.forward?.note ?? null,
    editedAt: null,
    attachment: draft.attachment,
    reactions: [],
    boosts: [],
    pinned: false,
    thread: null,
    poll: null,
    cards: [],
    cardsAsOf: createdAt,
    steps: [],
    createdAt,
    updatedAt: createdAt,
  };
}

/** RFC 3339 with milliseconds and `Z`. */
export function iso(ms: number): string {
  return new Date(ms).toISOString();
}

/** `touch`: now, or a millisecond past the previous stamp, whichever is later. */
export function touched(now: number, previous: string): string {
  return iso(Math.max(now, Date.parse(previous) + 1));
}

/** `ChannelThread#status_in_room`. */
export function threadStatus(thread: ThreadRecord, now: number): ThreadStatus {
  if (thread.locked) return "locked";

  const archiveAt = Date.parse(thread.lastActivityAt) + thread.autoArchiveAfterMinutes * 60_000;

  return thread.closed || archiveAt <= now ? "closed" : "active";
}

/** Replies that count: not system notes, not still streaming. */
export function countedReplies(thread: ThreadRecord): MessageDTO[] {
  return thread.messages.filter((message) => !message.systemNote && !message.streaming);
}

/** The thread's wire DTO. */
export function threadDto(thread: ThreadRecord, now: number): Thread {
  return {
    id: thread.id,
    roomId: thread.roomId,
    parentMessageId: thread.parentMessageId,
    creatorId: thread.creatorId,
    name: thread.name,
    status: threadStatus(thread, now),
    replyCount: countedReplies(thread).length,
    lastActivityAt: thread.lastActivityAt,
    autoArchiveAfterMinutes: thread.autoArchiveAfterMinutes,
    createdAt: thread.createdAt,
  };
}

/** Up to 3 distinct authors of the newest replies, newest first. */
export function replierIds(thread: ThreadRecord): number[] {
  const ids: number[] = [];

  for (const message of countedReplies(thread).reverse()) {
    if (!ids.includes(message.creatorId)) ids.push(message.creatorId);

    if (ids.length === 3) break;
  }

  return ids;
}

/** The parent's reply indicator. */
export function indicatorOf(thread: ThreadRecord): ThreadIndicator {
  return {
    threadId: thread.id,
    replyCount: countedReplies(thread).length,
    lastReplyAt: thread.lastActivityAt,
    replierIds: replierIds(thread),
  };
}

/** The first line of a message's text, for a thread's default name. */
export function defaultThreadName(parent: MessageDTO | null): string {
  const text = (parent?.markdownSource ?? "")
    .replace(/[*_`]/g, "")
    .replace(/\[([^\]]*)\]\([^)]*\)/g, "$1")
    .replace(/@\[([^\]]+)\]/g, "@$1");

  const firstLine = (text.split("\n")[0] ?? "").trim();
  const chars = [...firstLine];

  if (chars.length === 0) return "New thread";

  return chars.length > THREAD_NAME_LIMIT
    ? `${chars.slice(0, THREAD_NAME_LIMIT - 1).join("")}…`
    : firstLine;
}

/** Where a message lives. */
export interface MessageLocation {
  readonly room: RoomRecord;
  readonly thread: ThreadRecord | null;
  /** The array that holds it: the room's root timeline or the thread's replies. */
  readonly list: MessageDTO[];
  readonly index: number;
  readonly message: MessageDTO;
}

/** Finds a message anywhere in the world; `null` when there's none with that id. */
export function locate(world: World, messageId: number): MessageLocation | null {
  for (const room of world.rooms.values()) {
    const index = room.messages.findIndex((message) => message.id === messageId);
    const message = room.messages[index];

    if (message !== undefined) return { room, thread: null, list: room.messages, index, message };
  }

  for (const thread of world.threads.values()) {
    const index = thread.messages.findIndex((message) => message.id === messageId);
    const message = thread.messages[index];
    const room = world.rooms.get(thread.roomId);

    if (message !== undefined && room !== undefined) {
      return { room, thread, list: thread.messages, index, message };
    }
  }

  return null;
}

/** The message's conversation topic: `thread:<id>` for a reply, else `room:<id>`. */
export function topicOf(message: MessageDTO): string {
  return message.threadId === null ? `room:${message.roomId}` : `thread:${message.threadId}`;
}

/** The room's pin count (`MessagePin::count_for_room`). */
export function pinCount(world: World, roomId: number): number {
  let count = 0;

  for (const record of world.pins.values()) {
    if (record.roomId === roomId) count += 1;
  }

  return count;
}

/** Who and what a page refers to beyond its messages. */
export interface PageLookups {
  readonly usersFor: (ids: Iterable<number>) => User[];
  readonly saved: ReadonlyMap<number, SavedItem>;
}

function slice(
  list: readonly MessageDTO[],
  from: number,
  to: number,
  lookups: PageLookups,
): MessagePage {
  const start = Math.max(0, from);
  const end = Math.min(list.length, to);
  const messages = list.slice(start, end);
  const oldest = messages[0];
  const newest = messages.at(-1);
  const saved = [];

  for (const message of messages) {
    const item = lookups.saved.get(message.id);

    if (item !== undefined) saved.push({ messageId: message.id, savedItemId: item.id });
  }

  return {
    messages,
    users: lookups.usersFor(messages.map((message) => message.creatorId)),
    before: oldest !== undefined && start > 0 ? oldest.id : null,
    after: newest !== undefined && end < list.length ? newest.id : null,
    saved,
  };
}

/**
 * One page of a timeline (a room's root messages or a thread's replies): `before`, `after` or
 * `around` a message on it, or the newest page.
 */
export function pageOf(
  list: readonly MessageDTO[],
  query: URLSearchParams,
  lookups: PageLookups,
): MessagePage {
  const cursors = (["before", "after", "around"] as const).filter((key) => query.has(key));

  if (cursors.length > 1) throw validation("base", "Pass at most one of before, after, around");

  const [cursor] = cursors;

  if (cursor === undefined) return slice(list, list.length - PAGE_SIZE, list.length, lookups);

  const anchorId = Number(query.get(cursor));
  const index = list.findIndex((message) => message.id === anchorId);

  if (index < 0) throw notFound("Message not found");

  switch (cursor) {
    case "before":
      return slice(list, index - PAGE_SIZE, index, lookups);
    case "after":
      return slice(list, index + 1, index + 1 + PAGE_SIZE, lookups);
    case "around":
      return slice(list, index - PAGE_SIZE, index + 1 + PAGE_SIZE, lookups);
  }
}
