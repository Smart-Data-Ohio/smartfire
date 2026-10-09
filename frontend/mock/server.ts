/**
 * An in-memory Smartfire backend for the SPA's dev mode and integration tests: every S1 and S2
 * endpoint and the sync protocol over the real wire contract (camelCase, numeric ids, RFC 3339
 * millisecond timestamps, nulls never omitted). Plain TypeScript with no Node-only APIs, so the
 * same server runs behind the Vite dev server and inside jsdom tests. Never imported by app code.
 *
 * The S1 routes live here; the S2 ones are modules under s2/, each handed an `S2Context` and
 * reached through `dispatch` when no S1 route matches.
 */
import type { Me } from "../src/gen/Me.ts";
import type { MessageDTO } from "../src/gen/MessageDTO.ts";
import type { MessageReactions } from "../src/gen/MessageReactions.ts";
import type { PinState } from "../src/gen/PinState.ts";
import type { Presence } from "../src/gen/Presence.ts";
import type { PresenceList } from "../src/gen/PresenceList.ts";
import type { ReadState } from "../src/gen/ReadState.ts";
import type { RoomDetail } from "../src/gen/RoomDetail.ts";
import type { Sidebar } from "../src/gen/Sidebar.ts";
import type { SidebarRow } from "../src/gen/SidebarRow.ts";
import type { UnreadDivider } from "../src/gen/UnreadDivider.ts";
import type { User } from "../src/gen/User.ts";
import type { UserList } from "../src/gen/UserList.ts";
import {
  forbidden,
  headerOf,
  type MockBinaryRequest,
  type MockBinaryResponse,
  type MockRequest,
  type MockResponse,
  noContent,
  notFound,
  plainError,
  queryOf,
  respond,
  validation,
} from "./http.ts";
import { booleanField, field, intField, type Json, type JsonRecord, stringField } from "./json.ts";
import { type Mentionable, mentionsUser, renderMarkdown } from "./markdown.ts";
import { createRandom, type Random } from "./random.ts";
import { createAccount } from "./s2/account.ts";
import { createAdmin } from "./s2/admin.ts";
import { createAmbient } from "./s2/ambient.ts";
import { createBots } from "./s2/bots.ts";
import { createComposer, VIEWER_TIME_ZONE } from "./s2/composer.ts";
import { dispatch, type S2Context } from "./s2/context.ts";
import { createDirects } from "./s2/directs.ts";
import { createMessages } from "./s2/messages.ts";
import {
  buildMessage,
  locate,
  type MessageDraft,
  pageOf,
  pinCount,
  plainDraft,
  threadStatus,
} from "./s2/model.ts";
import { createPanes } from "./s2/panes.ts";
import { createPeople } from "./s2/people.ts";
import { clientMessageIdOf, parseMessage } from "./s2/posting.ts";
import { MESSAGE_IDS, SCHEDULED_IDS, THREAD_IDS } from "./s2/seed.ts";
import { createSettings } from "./s2/settings.ts";
import { createSlack } from "./s2/slack.ts";
import { createThreads } from "./s2/threads.ts";
import { createUploads, isBinaryPath } from "./s2/uploads.ts";
import { createActivity, scheduledInboxHooks } from "./s3/activity.ts";
import { createServerInboxAmbient } from "./s3/ambient.ts";
import { CARD_IDS, createCards } from "./s3/cards.ts";
import { createOrganize } from "./s3/organize.ts";
import { createSaved } from "./s3/saved.ts";
import { createSearch } from "./s3/search.ts";
import {
  buildWorld,
  DUE_REMINDER_DELAY_MS,
  S3_MESSAGE_IDS,
  S3_ROOM_IDS,
  S3_SCHEDULED_IDS,
  S3_THREAD_IDS,
} from "./s3/seed.ts";
import { AGENT_IDS, createAgents, HIDDEN_ROOM, statusControl } from "./s4/agents.ts";
import { createApprovals } from "./s4/approvals.ts";
import { createLedger, UNKNOWN_LEDGER_TYPE } from "./s4/ledger.ts";
import { S4_BOARD, S4_BOARD_POST_IDS, S4_WORK_IDS, seedWork } from "./s4/seed.ts";
import { createWork } from "./s4/work.ts";
import { WORK_STATUSES } from "./s4/work-model.ts";
import { createHuddles } from "./s5/huddles.ts";
import { createBoards } from "./s6/boards.ts";
import { BOARD_POST_IDS, BOARD_ROOM_ID } from "./s6/seed.ts";
import { createWorkLinks } from "./s6/work-links.ts";
import { createEvents, EVENT_IDS } from "./s8/events.ts";
import { createFizzy } from "./s8/fizzy.ts";
import { createRoomIntegrations } from "./s8/room-integrations.ts";
import { createRoomManagement } from "./s8/rooms.ts";
import { realScheduler, type Scheduler } from "./scheduler.ts";
import {
  BOT_ID,
  CATEGORY_IDS,
  JOINABLE_HISTORY_LENGTH,
  JOINABLE_OLDEST_MESSAGE_ID,
  JOINABLE_OPEN_ROOM,
  ROOM_IDS,
  type RoomRecord,
  seededUuid,
  timestamp,
  USER_IDS,
  VIEWER_ID,
  type World,
} from "./seed.ts";
import { createSimulation, type Simulation } from "./simulation.ts";
import {
  createSyncHub,
  type DropSocket,
  type Outgoing,
  type SendFrame,
  type SyncConnection,
} from "./sync.ts";

export type {
  MockBinaryRequest,
  MockBinaryResponse,
  MockHeaders,
  MockRequest,
  MockResponse,
} from "./http.ts";

export { PAGE_SIZE, SOURCE_LIMIT } from "./s2/model.ts";

export { isBinaryPath } from "./s2/uploads.ts";

/** The seeded ids, for tests and screenshots. */
export const SEED_IDS = {
  viewer: VIEWER_ID,
  bot: BOT_ID,
  users: USER_IDS,
  rooms: ROOM_IDS,
  categories: CATEGORY_IDS,
  threads: THREAD_IDS,
  messages: MESSAGE_IDS,
  scheduled: SCHEDULED_IDS,
  s3: {
    rooms: S3_ROOM_IDS,
    threads: S3_THREAD_IDS,
    scheduled: S3_SCHEDULED_IDS,
    messages: S3_MESSAGE_IDS,
    dueReminderDelayMs: DUE_REMINDER_DELAY_MS,
  },
  s4: {
    work: S4_WORK_IDS,
    board: S4_BOARD,
    boardPosts: S4_BOARD_POST_IDS,
    agents: AGENT_IDS,
    hiddenRoom: HIDDEN_ROOM,
    unknownLedgerType: UNKNOWN_LEDGER_TYPE,
  },
  cards: CARD_IDS,
  boards: { roomId: BOARD_ROOM_ID, posts: BOARD_POST_IDS },
  events: EVENT_IDS,
} as const;

export interface MockServerOptions {
  /** The clock (ms since the epoch); seed data is placed relative to it. */
  readonly now?: () => number;
  /** Seeds the PRNG behind the data, ids and simulation. */
  readonly seed?: number;
  /** Ambient chatter, presence drift and bot replies. Off by default (tests). */
  readonly simulate?: boolean;
  /** Timers for the simulation, bot streaming and pings. Defaults to `setTimeout`. */
  readonly scheduler?: Scheduler;
}

export interface MockServer {
  /**
   * Serves `/api/v1/*`, `/__mock/*` and classic's tour stamp (`/users/me/tour`). Held sends
   * resolve when released.
   */
  handle(request: MockRequest): Promise<MockResponse>;
  /** Serves the byte routes: the upload `PUT`, blob downloads and icon images. */
  handleBinary(request: MockBinaryRequest): Promise<MockBinaryResponse>;
  /** Opens a sync connection; `drop` is how the hub hangs up abruptly. */
  connect(send: SendFrame, drop?: DropSocket): SyncConnection;
  /** The CSRF token non-GET requests must send as `X-CSRF-Token`. */
  csrfToken(): string;
  /**
   * The boot JSON the Rust shell inlines in `<script type="application/json" id="boot">` (boot
   * without its CSRF token, which the meta tag carries), escaped for a script element.
   */
  inlineBoot(): string;
  /** Stops the ambient simulation (the bot still answers). */
  pause(): void;
  resume(): void;
  /** `userId` starts or stops typing in a room (fanned out to `room:<id>`). */
  typing(roomId: number, userId: number, on: boolean): void;
  /** `userId` posts Markdown in a room, as if from their own client. */
  post(roomId: number, userId: number, markdown: string): MessageDTO;
  /** While on, `POST .../messages` requests wait until released. Turning it off releases them. */
  holdSends(on: boolean): void;
  /** Lets every held send through. */
  releaseSends(): void;
  /** Held sends still waiting. */
  pendingSends(): number;
  /** While on, upload `PUT`s wait until released. Turning it off releases them. */
  holdUploads(on: boolean): void;
  /** Lets every held upload through. */
  releaseUploads(): void;
  /** Delays each upload `PUT`'s answer by `ms` on the scheduler (0 turns it off). */
  throttleUploads(ms: number): void;
  /** Held uploads still waiting. */
  pendingUploads(): number;
  /** `userId` replies in a thread, as if from their own client. */
  threadPost(threadId: number, userId: number, markdown: string): MessageDTO;
  /** `userId` starts or stops typing in a thread (fanned out to `thread:<id>`). */
  threadTyping(threadId: number, userId: number, on: boolean): void;
  /** `userId` reacts (toggling) or boosts, as `POST /messages/:id/boosts` would for them. */
  react(messageId: number, userId: number, content: string): MessageReactions;
  /** `userId` pins (or, with `pinned: false`, unpins) a message. */
  pin(messageId: number, userId: number, pinned?: boolean): PinState;
  /**
   * Posts the scheduled messages that are due now (all pending ones with `all`). Their timers
   * also fire on their own, exactly at `sendAt`.
   */
  fireScheduled(all?: boolean): number;
  /** A new CSRF token; the old one now gets 422 InvalidAuthenticityToken. */
  rotateCsrf(): string;
  /** Hangs up every sync connection (abruptly, or after `bye{reconnect:true}`). */
  dropConnections(bye?: boolean): void;
  /** Sends `resync` for every topic to every connection. */
  resync(): void;
  /** Sets someone's presence and publishes it. */
  setPresence(userId: number, presence: Presence, statusText?: string | null): void;
  /** Back to the seed: data, ids, a new epoch; connections are dropped with `bye`. */
  reset(): void;
  /** Stops timers and drops connections. */
  dispose(): void;
  /** The sync epoch and last sequence number. */
  syncState(): { readonly epoch: string; readonly seq: number; readonly connections: number };
}

function token(random: Random): string {
  const alphabet = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

  return Array.from({ length: 43 }, () => alphabet[random.int(0, alphabet.length - 1)]).join("");
}

function idsParam(query: URLSearchParams): number[] {
  const ids = (query.get("ids") ?? "")
    .split(",")
    .map((part) => Number(part.trim()))
    .filter((id) => Number.isInteger(id) && id > 0);

  return [...new Set(ids)].sort((a, b) => a - b).slice(0, 100);
}

/** Creates a mock server. Deterministic for a given `seed` and `now`. */
export function createMockServer(options: MockServerOptions = {}): MockServer {
  const now = options.now ?? Date.now;
  const seed = options.seed ?? 1;
  const scheduler = options.scheduler ?? realScheduler();
  const simulate = options.simulate ?? false;

  let world: World = seedWork(buildWorld(now(), seed), now());
  let random = createRandom(seed * 7919 + 17);
  let csrf = token(random);
  let restarts = 0;
  let holding = false;
  let held: (() => void)[] = [];
  let holdingJoins = false;
  let heldJoins: (() => void)[] = [];
  /**
   * The viewer's `tour_completed_at`: set, so the product tour stays out of every spec's way;
   * `/__mock/tour` clears it for the tour's own specs. `tourStamps` counts the stamps sent.
   */
  let tourCompleted = true;
  let tourStamps = 0;

  const epochFor = () => `${now().toString(36)}-${seed.toString(36)}-${restarts}`;

  const hub = createSyncHub({
    scheduler,
    epoch: epochFor(),
    viewerId: VIEWER_ID,
    memberOf: (roomId) => world.rooms.get(roomId)?.memberIds.includes(VIEWER_ID) ?? false,
  });

  // --- reads over the world ---

  const viewer = (): User => {
    const user = world.users.get(VIEWER_ID);

    if (user === undefined) throw new Error("the viewer is missing from the seed");

    return user;
  };

  const roomOr404 = (roomId: number): RoomRecord => {
    const record = world.rooms.get(roomId);

    if (record === undefined || !record.memberIds.includes(VIEWER_ID)) {
      throw notFound("Room not found");
    }

    return record;
  };

  const usersFor = (ids: Iterable<number>): User[] => {
    const users: User[] = [];

    for (const id of [...new Set(ids)].sort((a, b) => a - b)) {
      const user = world.users.get(id);

      if (user !== undefined) users.push(user);
    }

    return users;
  };

  const mentionables = (): Mentionable[] => {
    const people: Mentionable[] = [];

    for (const user of world.users.values()) {
      if (user.status === "active") people.push({ id: user.id, name: user.name });
    }

    return people;
  };

  const directMemberIds = (record: RoomRecord): number[] => {
    if (record.room.kind !== "direct") return [];

    const others = record.memberIds.filter((id) => id !== VIEWER_ID);

    return others.length > 0 ? others : [VIEWER_ID];
  };

  const displayName = (record: RoomRecord): string => {
    if (record.room.name !== null) return record.room.name;

    return usersFor(directMemberIds(record))
      .map((user) => user.name)
      .join(", ");
  };

  const unreadMessages = (record: RoomRecord): MessageDTO[] => {
    if (record.membership.unreadAt === null) return [];

    const lastRead = record.membership.lastReadMessageId ?? 0;

    return record.messages.filter((message) => message.id > lastRead && !message.systemNote);
  };

  const unreadDivider = (record: RoomRecord): UnreadDivider | null => {
    const unread = unreadMessages(record);
    const first = unread[0];

    return first === undefined ? null : { firstUnreadMessageId: first.id, count: unread.length };
  };

  /**
   * The server's `notificationCount` (crates/api/src/dto.rs `notification_count`), from what the
   * mock tracks: every unread root message in an `everything` room, the mentions in a `mentions`
   * or `muted` one, none in a `nothing` one.
   */
  const notificationCount = (record: RoomRecord, unreadCount: number): number => {
    switch (record.membership.involvement) {
      case "everything":
        return unreadCount;
      case "mentions":
      case "muted":
        return record.mentionCount;
      default:
        return 0;
    }
  };

  /** A direct room's newest root message as its row previews it (`last_direct_message`). */
  const lastMessage = (record: RoomRecord): SidebarRow["lastMessage"] => {
    const last = record.messages.findLast(
      (message) => message.threadId === null && !message.systemNote,
    );

    if (record.room.kind !== "direct" || last === undefined) return undefined;

    const text = last.bodyHtml
      .replace(/<[^>]*>/g, " ")
      .replace(/&lt;/g, "<")
      .replace(/&gt;/g, ">")
      .replace(/&quot;/g, '"')
      .replace(/&#39;/g, "'")
      .replace(/&amp;/g, "&")
      .replace(/\s+/g, " ")
      .trim();

    const plain = text === "" ? (last.attachment?.filename ?? "") : text;

    return {
      creatorId: last.creatorId,
      excerpt: plain.length > 140 ? `${plain.slice(0, 139)}…` : plain,
      createdAt: last.createdAt,
    };
  };

  const sidebarRow = (record: RoomRecord): SidebarRow => {
    const unreadCount = unreadMessages(record).length;

    const row: SidebarRow = {
      room: record.room.kind === "direct" ? { ...record.room, name: null } : record.room,
      membership: record.membership,
      displayName: displayName(record),
      directMemberIds: directMemberIds(record),
      unreadCount,
      mentionCount: record.mentionCount,
      notificationCount: notificationCount(record, unreadCount),
      threadNotificationCount: 0,
    };

    const last = lastMessage(record);

    if (last !== undefined) {
      row.lastMessage = last;
    }

    return row;
  };

  const visibleRooms = (): RoomRecord[] =>
    [...world.rooms.values()]
      .filter(
        (record) =>
          record.memberIds.includes(VIEWER_ID) && record.membership.involvement !== "invisible",
      )
      .sort((a, b) => {
        const left = (a.room.name ?? "").toLowerCase();
        const right = (b.room.name ?? "").toLowerCase();

        return left < right ? -1 : left > right ? 1 : a.room.id - b.room.id;
      });

  // --- endpoints ---

  const boot = (): JsonRecord => {
    const user = viewer();

    return {
      user: { id: user.id, name: user.name, avatarUrl: user.avatarUrl },
      account: admin.branding(),
      ...settings.appearance(),
      cableUrl: "/cable",
      serviceWorkerUrl: null,
      version: "mock",
      revision: null,
      csrfToken: csrf,
    };
  };

  const me = (): Me => ({
    user: viewer(),
    emailAddress: "riel@smartdata.example",
    preferences: {
      ...settings.appearance(),
      timeZone: VIEWER_TIME_ZONE,
      timeZoneExplicit: false,
      tourCompleted,
      voiceMode: "voice_activity",
      pushToTalkKey: "Space",
    },
    presenceSetting: "auto",
    doNotDisturb: world.doNotDisturb,
    quietHours: null,
    outOfOffice: world.outOfOffice,
    lastRoomId: ROOM_IDS.general,
  });

  const sidebar = (): Sidebar => {
    const rows = visibleRooms().map(sidebarRow);
    const withDirect = new Set<number>();

    for (const record of world.rooms.values()) {
      if (record.room.kind === "direct" && record.memberIds.length === 2) {
        for (const id of record.memberIds) withDirect.add(id);
      }
    }

    const placeholders = [...world.users.values()]
      .filter(
        (user) =>
          user.status === "active" &&
          user.role !== "bot" &&
          user.id !== VIEWER_ID &&
          !withDirect.has(user.id),
      )
      .sort((a, b) => (a.createdAt < b.createdAt ? -1 : a.createdAt > b.createdAt ? 1 : 0))
      .slice(0, 20)
      .map((user) => user.id);

    return {
      rows,
      categories: [...world.categories],
      users: usersFor([...rows.flatMap((row) => row.directMemberIds), ...placeholders]),
      directPlaceholderUserIds: placeholders,
      canCreateRooms: true,
    };
  };

  const materialiseJoinable = (): RoomRecord => {
    const createdAt = timestamp(now() - 30 * 24 * 60 * 60 * 1000);

    const messages = Array.from({ length: JOINABLE_HISTORY_LENGTH }, (_, index) => {
      const postedAt = timestamp(now() - (JOINABLE_HISTORY_LENGTH - index) * 60_000);

      const markdown =
        index === 0
          ? "campfire-oldest"
          : index === JOINABLE_HISTORY_LENGTH - 1
            ? "campfire-newest"
            : `campfire note ${index}`;

      return buildMessage(
        JOINABLE_OLDEST_MESSAGE_ID + index,
        JOINABLE_OPEN_ROOM.id,
        null,
        plainDraft(USER_IDS.maya, markdown, `campfire-${index}`),
        postedAt,
        mentionables(),
      );
    });

    const record: RoomRecord = {
      room: {
        id: JOINABLE_OPEN_ROOM.id,
        kind: "open",
        name: JOINABLE_OPEN_ROOM.name,
        iconName: null,
        creatorId: USER_IDS.priya,
        createdAt,
        updatedAt: createdAt,
      },
      memberIds: [...JOINABLE_OPEN_ROOM.memberIds, VIEWER_ID],
      membership: {
        id: 9_000 + JOINABLE_OPEN_ROOM.id,
        roomId: JOINABLE_OPEN_ROOM.id,
        userId: VIEWER_ID,
        involvement: "mentions",
        unreadAt: null,
        lastReadMessageId: null,
        roomCategoryId: null,
        favoritePosition: null,
        stageRole: null,
      },
      messages,
      mentionCount: 0,
    };

    world.rooms.set(record.room.id, record);

    return record;
  };

  /** Alive open rooms only. The catalog room isn't in the world until someone joins it. */
  const openPreview = (roomId: number): { id: number; name: string } | null => {
    const record = world.rooms.get(roomId);

    if (record !== undefined) {
      if (record.room.kind !== "open") return null;

      return { id: record.room.id, name: record.room.name ?? "" };
    }

    if (roomId !== JOINABLE_OPEN_ROOM.id) return null;

    return { id: JOINABLE_OPEN_ROOM.id, name: JOINABLE_OPEN_ROOM.name };
  };

  const publishJoined = (record: RoomRecord) => {
    hub.publish([
      {
        topic: "user",
        type: "sidebar.row.upserted",
        data: { ...sidebarRow(record), refreshRoom: true },
      },
    ]);
  };

  const joinOpen = (roomId: number) => {
    const existing = world.rooms.get(roomId);

    if (existing !== undefined) {
      if (existing.room.kind !== "open") throw notFound("Room not found");

      if (!existing.memberIds.includes(VIEWER_ID)) {
        existing.memberIds.push(VIEWER_ID);
        existing.membership = {
          id: 9_000 + existing.room.id,
          roomId: existing.room.id,
          userId: VIEWER_ID,
          involvement: "mentions",
          unreadAt: null,
          lastReadMessageId: null,
          roomCategoryId: null,
          favoritePosition: null,
          stageRole: null,
        };
        publishJoined(existing);
      }

      return { detail: roomDetail(roomId), row: sidebarRow(existing) };
    }

    if (roomId !== JOINABLE_OPEN_ROOM.id) throw notFound("Room not found");

    const record = materialiseJoinable();

    publishJoined(record);

    return { detail: roomDetail(roomId), row: sidebarRow(record) };
  };

  const roomDetail = (roomId: number): RoomDetail => {
    const record = roomOr404(roomId);
    const direct = directMemberIds(record);
    const preview = record.memberIds.slice(0, 5);

    return {
      room: record.room.kind === "direct" ? { ...record.room, name: null } : record.room,
      membership: record.membership,
      displayName: displayName(record),
      memberCount: record.memberIds.length,
      pinsCount: pinCount(world, record.room.id),
      directMemberIds: direct,
      memberPreviewIds: preview,
      users: usersFor([...direct, ...preview]),
      unread: unreadDivider(record),
    };
  };

  const messages = (roomId: number, query: URLSearchParams) =>
    pageOf(roomOr404(roomId).messages, query, { usersFor, saved: world.saved });

  // --- writes ---

  const markReadUpTo = (record: RoomRecord, messageId: number | null) => {
    record.membership = { ...record.membership, unreadAt: null, lastReadMessageId: messageId };
    record.mentionCount = 0;
  };

  /** Everything that happens when someone posts: the message, unreads, sidebar, the bot. */
  const createMessage = (record: RoomRecord, draft: MessageDraft): MessageDTO => {
    const createdAt = timestamp(Math.max(now(), Date.parse(record.room.updatedAt) + 1));
    const creatorId = draft.creatorId;

    const message = buildMessage(
      world.nextMessageId++,
      record.room.id,
      null,
      draft,
      createdAt,
      mentionables(),
    );

    record.messages.push(message);
    record.room = { ...record.room, updatedAt: createdAt };

    const events: Outgoing[] = [
      { topic: `room:${record.room.id}`, type: "message.created", data: message },
    ];

    if (creatorId === VIEWER_ID) {
      markReadUpTo(record, message.id);
    } else if (!message.systemNote) {
      const mentioned = mentionsUser(message.bodyHtml, VIEWER_ID);
      // `Room#unread_memberships`: a muted room goes unread only for a mention; every other
      // visible one (a "nothing" room too) goes unread for any message.
      const quiet = record.membership.involvement === "muted" && !mentioned;
      const viewing = hub.presentRooms().has(record.room.id);

      if (viewing && record.membership.unreadAt === null) {
        markReadUpTo(record, message.id);
      } else if (!viewing && !quiet) {
        if (record.membership.unreadAt === null) {
          record.membership = { ...record.membership, unreadAt: createdAt };
        }

        if (mentioned) record.mentionCount += 1;

        events.push({
          topic: "user",
          type: "room.unread",
          data: { roomId: record.room.id, messageId: message.id, mentioned },
        });
      }
    }

    if (record.memberIds.includes(VIEWER_ID)) {
      events.push({ topic: "user", type: "sidebar.row.upserted", data: sidebarRow(record) });
    }

    hub.publish(events);

    const forBot = record.room.id === ROOM_IDS.dmEmber || mentionsUser(message.bodyHtml, BOT_ID);

    if (simulate && creatorId === VIEWER_ID && forBot && !message.systemNote) {
      simulation.botReply(record.room.id);
    }

    return message;
  };

  const updateMessage = (messageId: number, markdown: string, streaming: boolean) => {
    const location = locate(world, messageId);

    if (location === null) return;

    const current = location.message;
    const updatedAt = timestamp(Math.max(now(), Date.parse(current.updatedAt) + 1));

    const message: MessageDTO = {
      ...current,
      bodyHtml: renderMarkdown(markdown, mentionables()),
      markdownSource: markdown,
      streaming,
      updatedAt,
    };

    location.list[location.index] = message;
    hub.publish([
      { topic: `room:${location.room.room.id}`, type: "message.updated", data: message },
    ]);
  };

  const createFromClient = (roomId: number, body: Json | undefined): MockResponse => {
    const record = roomOr404(roomId);
    const clientMessageId = clientMessageIdOf(body);
    const key = `${roomId}:${VIEWER_ID}:${clientMessageId}`;
    const duplicate = world.sentByClientId.get(key);

    if (duplicate !== undefined) return { status: 200, json: duplicate };

    const parsed = parseMessage(body, uploads.attachment);
    const replyTo = parsed.replyToMessageId;

    if (replyTo !== null && !record.messages.some((message) => message.id === replyTo)) {
      throw validation("replyToMessageId", "Reply to message must be on this timeline");
    }

    const message = createMessage(record, {
      ...plainDraft(VIEWER_ID, parsed.markdown, clientMessageId),
      replyToMessageId: replyTo,
      attachment: parsed.attachment,
    });

    world.sentByClientId.set(key, message);

    return { status: 201, json: message };
  };

  const markRead = (roomId: number): ReadState => {
    const record = roomOr404(roomId);

    markReadUpTo(record, record.messages.at(-1)?.id ?? null);
    hub.publish([{ topic: "user", type: "room.read", data: { roomId } }]);

    return { roomId, unread: false, firstUnreadMessageId: null, unreadCount: 0 };
  };

  const markUnread = (roomId: number, body: Json | undefined): ReadState => {
    const record = roomOr404(roomId);
    const messageId = intField(body, "messageId");
    const index = record.messages.findIndex((message) => message.id === messageId);
    const message = record.messages[index];

    if (messageId === null || message === undefined) throw notFound("Message not found");

    record.membership = {
      ...record.membership,
      unreadAt: message.createdAt,
      lastReadMessageId: record.messages[index - 1]?.id ?? null,
    };
    hub.publish([
      { topic: "user", type: "room.unread", data: { roomId, messageId: null, mentioned: false } },
    ]);

    return {
      roomId,
      unread: true,
      firstUnreadMessageId: message.id,
      unreadCount: unreadMessages(record).length,
    };
  };

  const userList = (query: URLSearchParams): UserList => ({ users: usersFor(idsParam(query)) });

  const presenceList = (query: URLSearchParams): PresenceList => {
    const presences = [];

    for (const id of idsParam(query)) {
      const presence = world.presence.get(id);

      if (presence !== undefined) presences.push(presence);
    }

    return { presences };
  };

  const setPresence = (userId: number, presence: Presence, statusText?: string | null) => {
    const current = world.presence.get(userId);

    if (current === undefined) return;

    const next = { ...current, presence, statusText: statusText ?? current.statusText };

    world.presence.set(userId, next);
    hub.publish([{ topic: "user", type: "presence", data: next }]);
  };

  const typing = (roomId: number, userId: number, on: boolean) => {
    hub.publish([{ topic: `room:${roomId}`, type: "typing", data: { userId, on } }]);
  };

  const post = (roomId: number, userId: number, markdown: string): MessageDTO => {
    const record = world.rooms.get(roomId);

    if (record === undefined) throw notFound("Room not found");

    return createMessage(record, plainDraft(userId, markdown, seededUuid(random)));
  };

  // --- the simulation ---

  const simulation: Simulation = createSimulation({
    scheduler,
    random: createRandom(seed * 104_729 + 3),
    botId: BOT_ID,
    host: {
      typing,
      post(roomId, userId, markdown, streaming) {
        const record = world.rooms.get(roomId);

        return record === undefined
          ? null
          : createMessage(record, {
              ...plainDraft(userId, markdown, seededUuid(random)),
              streaming,
            });
      },
      update: updateMessage,
      setPresence: (userId, presence) => setPresence(userId, presence),
      subscribedRooms: () => hub.subscribedRooms(),
      liveRooms: () => [
        ROOM_IDS.general,
        ROOM_IDS.design,
        ROOM_IDS.engineering,
        ROOM_IDS.random,
        ROOM_IDS.launchPlanning,
        ROOM_IDS.dmMaya,
        ROOM_IDS.groupDm,
      ],
      posters: (roomId) =>
        (world.rooms.get(roomId)?.memberIds ?? []).filter(
          (id) => id !== VIEWER_ID && id !== BOT_ID,
        ),
      firstNames: (roomId, exceptUserId) =>
        usersFor(world.rooms.get(roomId)?.memberIds ?? [])
          .filter((user) => user.id !== exceptUserId && user.role !== "bot")
          .map((user) => user.name.split(/\s+/)[0] ?? user.name),
      presencePeople: () => [...world.presence.keys()].filter((id) => id !== VIEWER_ID),
    },
  });

  if (simulate) simulation.start();

  // --- holding sends ---

  const release = () => {
    const waiting = held;

    held = [];

    for (const run of waiting) run();
  };

  const holdSends = (on: boolean) => {
    holding = on;

    if (!on) release();
  };

  /** Runs a posting request now, or once released while sends are held. */
  const whenReleased = (run: () => MockResponse): MockResponse | Promise<MockResponse> => {
    if (!holding) return run();

    return new Promise<MockResponse>((resolve) => {
      held.push(() => resolve(respond(run)));
    });
  };

  // --- the S2 modules ---

  const ctx: S2Context = {
    world: () => world,
    now,
    scheduler,
    publish: (events) => hub.publish(events),
    uuid: () => seededUuid(random),
    hex: (length) => Array.from({ length }, () => random.int(0, 15).toString(16)).join(""),
    roomOr404,
    usersFor,
    mentionables,
    sidebarRow,
    roomDetail,
    displayName,
    postToRoom: createMessage,
  };

  const uploads = createUploads(ctx);
  const admin = createAdmin(ctx, uploads);
  // Boot and `/me` (above) read the saved theme and text size from here, once requests arrive.
  const settings = createSettings(ctx, uploads, admin.requireSudo);
  const threads = createThreads(ctx, uploads, whenReleased);
  const activity = createActivity(ctx);
  const saved = createSaved(ctx, activity);
  const messageActions = createMessages(ctx, threads, saved.savedChanged);
  const work = createWork(ctx, threads);
  const boards = createBoards(ctx, threads, uploads, work);

  const composer = createComposer(
    ctx,
    threads,
    (messageId, remindAt) => {
      messageActions.save(messageId, remindAt);
    },
    scheduledInboxHooks(ctx, activity),
  );

  const agents = createAgents(ctx, createRandom(seed * 49_979_687 + 3), () => simulation.paused());

  const approvals = createApprovals(
    ctx,
    agents,
    activity,
    createRandom(seed * 67_867_967 + 11),
    () => simulation.paused(),
  );

  const ledger = createLedger(ctx, agents);

  agents.seed();
  approvals.seed();
  ledger.seed();

  const huddles = createHuddles(ctx, simulate);
  const events = createEvents(ctx);
  const cards = createCards(ctx, events);
  const fizzy = createFizzy(ctx, threads);

  const routes = [
    ...agents.routes,
    ...approvals.routes,
    ...ledger.routes,
    ...createRoomManagement(ctx, admin, huddles).routes,
    ...createRoomIntegrations(ctx).routes,
    ...events.routes,
    ...huddles.routes,
    ...cards.routes,
    ...fizzy.routes,
    ...uploads.routes,
    ...threads.routes,
    ...boards.routes,
    ...createWorkLinks(ctx, threads).routes,
    ...messageActions.routes,
    ...composer.routes,
    ...createDirects(ctx).routes,
    ...createPanes(ctx).routes,
    ...activity.routes,
    ...saved.routes,
    ...work.routes,
    ...settings.routes,
    ...createAccount(ctx).routes,
    ...admin.routes,
    ...createPeople(ctx, admin.requireSudo, agents).routes,
    ...createBots(ctx, uploads, admin.requireSudo).routes,
    ...createSlack(ctx, admin.requireSudo).routes,
    ...createOrganize(ctx).routes,
  ];

  composer.arm();
  saved.arm();

  const threadPost = (threadId: number, userId: number, markdown: string): MessageDTO => {
    const thread = world.threads.get(threadId);

    if (thread === undefined) throw notFound("Thread not found");

    if (thread.locked) throw forbidden("This thread is locked");

    return threads.postReply(thread, plainDraft(userId, markdown, seededUuid(random)));
  };

  const ambient = createAmbient(
    {
      reactable: () => {
        const targets = [];

        for (const roomId of hub.subscribedRooms()) {
          const record = world.rooms.get(roomId);

          if (record === undefined || record.room.kind === "direct") continue;

          const people = record.memberIds.filter((id) => id !== VIEWER_ID && id !== BOT_ID);

          for (const message of record.messages.slice(-5)) {
            if (!message.systemNote) targets.push({ messageId: message.id, people });
          }
        }

        return targets;
      },
      threads: () =>
        [...world.threads.values()].flatMap((thread) =>
          threadStatus(thread, now()) === "active"
            ? [
                {
                  threadId: thread.id,
                  people: [...thread.memberIds].filter((id) => id !== VIEWER_ID && id !== BOT_ID),
                },
              ]
            : [],
        ),
      react: (messageId, userId, content) => {
        messageActions.react(messageId, userId, content);
      },
      typing: (threadId, userId, on) => threads.typing(threadId, userId, on),
      reply: (threadId, userId, markdown) => {
        threadPost(threadId, userId, markdown);
      },
      paused: () => simulation.paused(),
    },
    scheduler,
    createRandom(seed * 15_485_863 + 7),
  );

  const inboxAmbient = createServerInboxAmbient(
    ctx,
    // The inbox's approval requests are real requests (S4), so deciding one updates its item.
    {
      ...activity,
      record: (draft) =>
        draft.eventType === "agent_approval_request"
          ? approvals.requestFromInbox(draft)
          : activity.record(draft),
    },
    () => simulation.paused(),
    createRandom(seed * 32_452_843 + 5),
  );

  if (simulate) {
    ambient.start();
    inboxAmbient.start();
    agents.start();
    approvals.start();
  }

  /** Global search (S3), after the other modules' routes. */
  const searchRoutes = createSearch(ctx).routes;

  // --- routing ---

  const api = (request: MockRequest, path: string): MockResponse | Promise<MockResponse> => {
    const method = request.method.toUpperCase();
    const query = queryOf(request);

    if (method !== "GET" && method !== "HEAD") {
      const sent = headerOf(request.headers, "x-csrf-token");

      if (sent !== csrf) {
        throw plainError(422, "InvalidAuthenticityToken", "Can't verify CSRF token authenticity.");
      }
    }

    const joining = /^\/rooms\/(\d+)\/(preview|join)$/.exec(path);

    if (joining !== null) {
      const id = Number(joining[1]);

      if (method === "GET" && joining[2] === "preview") {
        const preview = openPreview(id);

        if (preview === null) throw notFound("Room not found");

        return { status: 200, json: preview };
      }

      if (method === "POST" && joining[2] === "join") {
        // Membership (and its sidebar broadcast) exist before a held response is released, so a
        // rename or a return can land while the client is still waiting on this body.
        const created = joinOpen(id);

        if (!holdingJoins) return { status: 200, json: created };

        return new Promise<MockResponse>((resolve) => {
          heldJoins.push(() => resolve({ status: 200, json: created }));
        });
      }
    }

    const room = /^\/rooms\/(\d+)(\/messages|\/read)?$/.exec(path);
    const route = `${method} ${room === null ? path : `/rooms/:id${room[2] ?? ""}`}`;
    const roomId = Number(room?.[1]);

    switch (route) {
      case "GET /boot":
        return { status: 200, json: boot() };
      case "GET /me":
        return { status: 200, json: me() };
      case "GET /sidebar":
        return { status: 200, json: sidebar() };
      case "GET /users":
        return { status: 200, json: userList(query) };
      case "GET /presence":
        return { status: 200, json: presenceList(query) };
      case "GET /rooms/:id":
        return { status: 200, json: roomDetail(roomId) };
      case "GET /rooms/:id/messages":
        return { status: 200, json: messages(roomId, query) };
      case "POST /rooms/:id/messages":
        roomOr404(roomId);

        return whenReleased(() => createFromClient(roomId, request.body));
      case "POST /rooms/:id/read":
        return { status: 200, json: markRead(roomId) };
      case "DELETE /rooms/:id/read":
        return { status: 200, json: markUnread(roomId, request.body) };
      default: {
        const handler = dispatch([...routes, ...searchRoutes], method, path, query, request.body);

        if (handler === null) throw notFound(`No route for ${method} /api/v1${path}`);

        return handler();
      }
    }
  };

  const control = (request: MockRequest, action: string): MockResponse => {
    const query = queryOf(request);
    const body = request.body;

    const int = (key: string): number => {
      const value = intField(body, key) ?? Number(query.get(key) ?? Number.NaN);

      if (!Number.isInteger(value)) throw validation(key, `${key} is required`);

      return value;
    };

    const text = (key: string): string => {
      const value = stringField(body, key) ?? query.get(key);

      if (value === null) throw validation(key, `${key} is required`);

      return value;
    };

    const flag = (key: string, fallback: boolean): boolean => {
      const value = booleanField(body, key);

      if (value !== null) return value;

      const raw = query.get(key);

      return raw === null ? fallback : raw === "1" || raw === "true";
    };

    const ok = { status: 200, json: { ok: true } };

    switch (action) {
      case "state":
        return { status: 200, json: state() };
      case "tour":
        tourCompleted = flag("completed", false);

        return ok;
      case "pause":
        server.pause();

        return ok;
      case "resume":
        server.resume();

        return ok;
      case "typing":
        typing(int("roomId"), int("userId"), flag("on", true));

        return ok;
      case "thread-typing":
        threads.typing(int("threadId"), int("userId"), flag("on", true));

        return ok;
      case "post":
        return { status: 201, json: post(int("roomId"), int("userId"), text("markdown")) };
      case "thread-post":
        return { status: 201, json: threadPost(int("threadId"), int("userId"), text("markdown")) };
      case "react":
        return {
          status: 200,
          json: messageActions.react(int("messageId"), int("userId"), text("content")),
        };
      case "pin":
        return {
          status: 200,
          json: messageActions.pin(int("messageId"), int("userId"), flag("pinned", true)),
        };
      case "schedule-due":
        return { status: 200, json: { sent: composer.fireScheduled(flag("all", false)).length } };
      case "schedule-sending":
        return { status: 200, json: composer.markSending(int("id"), flag("on", true)) };
      case "remind-due":
        return { status: 200, json: { reminded: saved.remindDue(flag("all", false)) } };
      case "activity-arrival":
        inboxAmbient.arrive();

        return ok;
      case "hold-sends":
        holdSends(flag("on", true));

        return ok;
      case "hold-join":
        holdingJoins = flag("on", true);

        if (!holdingJoins) {
          const waiting = heldJoins;

          heldJoins = [];

          for (const run of waiting) run();
        }

        return ok;
      case "restart":
        restarts += 1;
        hub.restart(epochFor());

        return ok;
      case "release-sends":
        release();

        return ok;
      case "lapse-sudo":
        admin.lapseSudo(flag("on", true));

        return ok;
      case "hold-uploads":
        uploads.hold(flag("on", true));

        return ok;
      case "release-uploads":
        uploads.release();

        return ok;
      case "throttle-uploads":
        uploads.throttle(int("ms"));

        return ok;
      case "rotate-csrf":
        return { status: 200, json: { csrfToken: server.rotateCsrf() } };
      case "drop-connections":
        hub.dropAll(flag("bye", false));

        return ok;
      case "resync":
        hub.resyncAll("mock_resync");

        return ok;
      case "presence": {
        const presence = text("presence");

        if (!["online", "idle", "offline", "dnd"].includes(presence)) {
          throw validation("presence", "presence must be online, idle, offline or dnd");
        }

        // SAFETY: checked against the four `Presence` literals just above.
        setPresence(int("userId"), presence as Presence, stringField(body, "statusText"));

        return ok;
      }

      case "work-status": {
        const status = text("status");

        return {
          status: 200,
          json: work.setStatusAs(
            int("threadId"),
            WORK_STATUSES.find((candidate) => candidate === status) ?? null,
            intField(body, "actorId") ?? USER_IDS.maya,
          ),
        };
      }

      case "agent-status":
        return {
          status: 200,
          json: agents.setStatus(int("agentId"), {
            ...statusControl(
              query.get("status") ?? stringField(body, "status"),
              booleanField(body, "suspended"),
              stringField(body, "presence") ?? query.get("presence"),
            ),
            statusNote:
              field(body, "statusNote") === undefined ? undefined : stringField(body, "statusNote"),
          }),
        };

      case "agent-steps": {
        const messageId =
          intField(body, "messageId") ??
          agents.latestBy(int("roomId"), intField(body, "userId") ?? BOT_ID)?.id ??
          0;

        return { status: 200, json: { steps: [...agents.setSteps(messageId, int("stage"))] } };
      }

      case "viewer-role": {
        const role = text("role");
        const viewer = world.users.get(VIEWER_ID);

        if (role !== "member" && role !== "administrator") {
          throw validation("role", "role must be member or administrator");
        }

        if (viewer !== undefined) world.users.set(VIEWER_ID, { ...viewer, role });

        return ok;
      }

      case "approval-request":
        return {
          status: 201,
          json: approvals.request(
            intField(body, "agentId") ?? AGENT_IDS.ember,
            stringField(body, "action") ?? "messages.post",
            text("summary"),
            intField(body, "roomId") ?? ROOM_IDS.engineering,
          ),
        };

      case "approval-settle": {
        const status = text("status");

        const known = (["pending", "approved", "denied", "cancelled", "expired"] as const).find(
          (candidate) => candidate === status,
        );

        if (known === undefined) {
          throw validation("status", "status must be an approval status");
        }

        return {
          status: 200,
          json: approvals.settle(int("id"), known, intField(body, "deciderId") ?? USER_IDS.priya),
        };
      }

      case "fizzy":
        return { status: 200, json: fizzy.control(body) };
      case "cards":
        return { status: 200, json: cards.control(body) };
      case "reset":
        server.reset();

        return ok;
      default: {
        const boardControl = boards.control(action, body);

        if (boardControl !== null) return boardControl;

        const handled = huddles.control(action, {
          int,
          text,
          flag,
          optionalText: (key) => stringField(body, key) ?? query.get(key),
        });

        if (handled === null) throw notFound(`No mock control named ${action}`);

        return handled;
      }
    }
  };

  /** Classic's users/tours#update: skipping and finishing both stamp the tour done; 204. */
  const stampTour = (request: MockRequest): MockResponse => {
    const method = request.method.toUpperCase();

    if (method !== "PATCH" && method !== "PUT")
      throw notFound(`No route for ${method} ${request.path}`);

    if (headerOf(request.headers, "x-csrf-token") !== csrf) {
      throw plainError(422, "InvalidAuthenticityToken", "Can't verify CSRF token authenticity.");
    }

    tourCompleted = true;
    tourStamps += 1;

    return noContent();
  };

  const state = (): JsonRecord => ({
    epoch: hub.epoch(),
    seq: hub.seq(),
    connections: hub.connectionCount(),
    simulate,
    paused: simulation.paused(),
    holdingSends: holding,
    pendingSends: held.length,
    pendingJoins: heldJoins.length,
    presentRoomIds: [...hub.presentRooms()].sort((left, right) => left - right),
    pendingUploads: uploads.pending(),
    csrfToken: csrf,
    tourCompleted,
    tourStamps,
    ids: SEED_IDS,
  });

  const server: MockServer = {
    async handle(request) {
      const path = request.path.split("?")[0] ?? "";

      try {
        if (path.startsWith("/api/v1/")) return await api(request, path.slice("/api/v1".length));

        if (path.startsWith("/__mock/")) return control(request, path.slice("/__mock/".length));

        if (TOUR_PATHS.has(path)) return stampTour(request);

        throw notFound(`No route for ${path}`);
      } catch (error) {
        return respond(() => {
          throw error;
        });
      }
    },
    handleBinary: (request) => uploads.handleBinary(request),
    connect: (send, drop) => hub.connect(send, drop),
    csrfToken: () => csrf,
    inlineBoot: () => {
      const { csrfToken: _meta, ...inline } = boot();

      return JSON.stringify(inline).replaceAll("<", "\\u003c");
    },
    pause: () => simulation.pause(),
    resume: () => simulation.resume(),
    typing,
    post,
    holdSends,
    releaseSends: release,
    pendingSends: () => held.length,
    holdUploads: (on) => uploads.hold(on),
    releaseUploads: () => uploads.release(),
    throttleUploads: (ms) => uploads.throttle(ms),
    pendingUploads: () => uploads.pending(),
    threadPost,
    threadTyping: (threadId, userId, on) => threads.typing(threadId, userId, on),
    react: (messageId, userId, content) => messageActions.react(messageId, userId, content),
    pin: (messageId, userId, pinned = true) => messageActions.pin(messageId, userId, pinned),
    fireScheduled: (all = false) => composer.fireScheduled(all).length,
    rotateCsrf() {
      csrf = token(random);

      return csrf;
    },
    dropConnections: (bye = false) => hub.dropAll(bye),
    resync: () => hub.resyncAll("mock_resync"),
    setPresence,
    reset() {
      release();
      holdingJoins = false;
      tourCompleted = true;
      tourStamps = 0;

      const waitingJoins = heldJoins;

      heldJoins = [];

      for (const run of waitingJoins) run();

      uploads.reset();
      admin.lapseSudo(false);
      composer.stop();
      saved.stop();
      simulation.stop();
      ambient.stop();
      inboxAmbient.stop();
      agents.stop();
      approvals.stop();
      world = seedWork(buildWorld(now(), seed), now());
      agents.seed();
      approvals.seed();
      ledger.seed();
      random = createRandom(seed * 7919 + 17);
      csrf = token(random);
      restarts += 1;
      hub.restart(epochFor());
      composer.arm();
      saved.arm();

      if (simulate) {
        simulation.resume();
        simulation.start();
        ambient.start();
        inboxAmbient.start();
        agents.start();
        approvals.start();
      }
    },
    dispose() {
      agents.stop();
      approvals.stop();
      release();
      uploads.reset();
      composer.stop();
      saved.stop();
      simulation.stop();
      ambient.stop();
      inboxAmbient.stop();
      hub.dropAll(false);
    },
    syncState: () => ({ epoch: hub.epoch(), seq: hub.seq(), connections: hub.connectionCount() }),
  };

  return server;
}

/** Classic's tour stamp, which the SPA calls outside `/api/v1`. */
const TOUR_PATHS = new Set(["/users/me/tour", "/users/me/tour.json"]);

/** Whether a request path is one the mock serves as JSON. */
export function isMockPath(path: string): boolean {
  return path.startsWith("/api/v1/") || path.startsWith("/__mock/") || TOUR_PATHS.has(path);
}

/** Whether a request path is one the mock serves at all, as JSON or bytes. */
export function isAnyMockPath(path: string): boolean {
  return isMockPath(path) || isBinaryPath(path);
}
