/**
 * An in-memory Smartfire backend for the SPA's dev mode and integration tests: every S1
 * endpoint and the sync protocol over the real wire contract (camelCase, numeric ids, RFC 3339
 * millisecond timestamps, nulls never omitted). Plain TypeScript with no Node-only APIs, so the
 * same server runs behind the Vite dev server and inside jsdom tests. Never imported by app code.
 */
import type { ApiError } from "../src/gen/ApiError.ts";
import type { Me } from "../src/gen/Me.ts";
import type { MessageDTO } from "../src/gen/MessageDTO.ts";
import type { MessagePage } from "../src/gen/MessagePage.ts";
import type { Presence } from "../src/gen/Presence.ts";
import type { PresenceList } from "../src/gen/PresenceList.ts";
import type { ReadState } from "../src/gen/ReadState.ts";
import type { RoomDetail } from "../src/gen/RoomDetail.ts";
import type { Sidebar } from "../src/gen/Sidebar.ts";
import type { SidebarRow } from "../src/gen/SidebarRow.ts";
import type { UnreadDivider } from "../src/gen/UnreadDivider.ts";
import type { User } from "../src/gen/User.ts";
import type { UserList } from "../src/gen/UserList.ts";
import { booleanField, intField, type Json, type JsonRecord, stringField } from "./json.ts";
import { type Mentionable, mentionsUser, renderMarkdown } from "./markdown.ts";
import { createRandom, type Random } from "./random.ts";
import { realScheduler, type Scheduler } from "./scheduler.ts";
import {
  BOT_ID,
  CATEGORY_IDS,
  ROOM_IDS,
  type RoomRecord,
  seededUuid,
  seedWorld,
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

/** Messages per page, as `Message::PAGE_SIZE`. */
export const PAGE_SIZE = 40;

/** `Message::SOURCE_LIMIT`. */
export const SOURCE_LIMIT = 50_000;

/** The seeded ids, for tests and screenshots. */
export const SEED_IDS = {
  viewer: VIEWER_ID,
  bot: BOT_ID,
  users: USER_IDS,
  rooms: ROOM_IDS,
  categories: CATEGORY_IDS,
} as const;

/** Request headers, names in any case. */
export interface MockHeaders {
  readonly [name: string]: string | undefined;
}

/** An HTTP request as the transports hand it over. */
export interface MockRequest {
  readonly method: string;
  /** The path, e.g. `/api/v1/rooms/1/messages`; a `?query` here is read too. */
  readonly path: string;
  readonly query?: URLSearchParams | string | undefined;
  /** The parsed JSON body, if any. */
  readonly body?: Json | undefined;
  readonly headers?: MockHeaders | undefined;
}

export interface MockResponse {
  readonly status: number;
  readonly json: Json;
}

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
  /** Serves `/api/v1/*` and `/__mock/*`. Held sends resolve when released. */
  handle(request: MockRequest): Promise<MockResponse>;
  /** Opens a sync connection; `drop` is how the hub hangs up abruptly. */
  connect(send: SendFrame, drop?: DropSocket): SyncConnection;
  /** The CSRF token non-GET requests must send as `X-CSRF-Token`. */
  csrfToken(): string;
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

class HttpError extends Error {
  readonly status: number;
  readonly error: ApiError;

  constructor(status: number, error: ApiError) {
    super(error.message);
    this.status = status;
    this.error = error;
  }
}

/** The `ApiError` variants that carry only a message. */
type PlainErrorTag = Exclude<ApiError["_tag"], "Validation" | "RateLimited">;

/**
 * An error body exactly as the Rust server serializes it. This is wire JSON, not an Effect
 * tagged value (Effect stays out of the mock), so the tag is plain data here.
 */
function plainError(status: number, tag: PlainErrorTag, message: string): HttpError {
  return new HttpError(status, { _tag: tag, message });
}

const notFound = (message = "Not found") => plainError(404, "NotFound", message);

const VALIDATION: ApiError["_tag"] = "Validation";

const validation = (field: string, message: string) =>
  new HttpError(422, {
    _tag: VALIDATION,
    message: `Validation failed: ${message}`,
    fields: { [field]: [message] },
  });

function token(random: Random): string {
  const alphabet = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

  return Array.from({ length: 43 }, () => alphabet[random.int(0, alphabet.length - 1)]).join("");
}

function queryOf(request: MockRequest): URLSearchParams {
  const inline = request.path.includes("?") ? request.path.slice(request.path.indexOf("?")) : "";

  if (request.query === undefined) return new URLSearchParams(inline);

  return new URLSearchParams(request.query);
}

function headerOf(request: MockRequest, name: string): string | undefined {
  for (const [key, value] of Object.entries(request.headers ?? {})) {
    if (key.toLowerCase() === name) return value;
  }

  return undefined;
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

  let world: World = seedWorld(now(), createRandom(seed));
  let random = createRandom(seed * 7919 + 17);
  let csrf = token(random);
  let restarts = 0;
  let sentByClientId = new Map<string, MessageDTO>();
  let holding = false;
  let held: (() => void)[] = [];

  const epochFor = () => `${now().toString(36)}-${seed.toString(36)}-${restarts}`;

  const hub = createSyncHub({ scheduler, epoch: epochFor(), viewerId: VIEWER_ID });

  // --- reads over the world ---

  const viewer = (): User => {
    const user = world.users.get(VIEWER_ID);

    if (user === undefined) throw new Error("the viewer is missing from the seed");

    return user;
  };

  const roomOr404 = (roomId: number): RoomRecord => {
    const record = world.rooms.get(roomId);

    if (record === undefined || record.membership.involvement === "invisible") {
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

  const sidebarRow = (record: RoomRecord): SidebarRow => ({
    room: record.room,
    membership: record.membership,
    displayName: displayName(record),
    directMemberIds: directMemberIds(record),
    unreadCount: unreadMessages(record).length,
    mentionCount: record.mentionCount,
  });

  const visibleRooms = (): RoomRecord[] =>
    [...world.rooms.values()]
      .filter((record) => record.membership.involvement !== "invisible")
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
      account: { name: "Smart Data" },
      theme: "system",
      textSize: "default",
      cableUrl: "/cable",
      version: "mock",
      revision: null,
      csrfToken: csrf,
    };
  };

  const me = (): Me => ({
    user: viewer(),
    emailAddress: "riel@smartdata.example",
    preferences: {
      theme: "system",
      textSize: "default",
      timeZone: "America/New_York",
      timeZoneExplicit: false,
      tourCompleted: true,
      voiceMode: "voice_activity",
      pushToTalkKey: "Space",
    },
    presenceSetting: "auto",
    doNotDisturb: { enabled: false, until: null },
    quietHours: null,
    outOfOffice: null,
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

  const roomDetail = (roomId: number): RoomDetail => {
    const record = roomOr404(roomId);
    const direct = directMemberIds(record);
    const preview = record.memberIds.slice(0, 5);

    return {
      room: record.room,
      membership: record.membership,
      displayName: displayName(record),
      memberCount: record.memberIds.length,
      pinsCount: record.pinsCount,
      directMemberIds: direct,
      memberPreviewIds: preview,
      users: usersFor([...direct, ...preview]),
      unread: unreadDivider(record),
    };
  };

  const page = (record: RoomRecord, from: number, to: number): MessagePage => {
    const start = Math.max(0, from);
    const end = Math.min(record.messages.length, to);
    const messages = record.messages.slice(start, end);
    const oldest = messages[0];
    const newest = messages.at(-1);

    return {
      messages,
      users: usersFor(messages.map((message) => message.creatorId)),
      before: oldest !== undefined && start > 0 ? oldest.id : null,
      after: newest !== undefined && end < record.messages.length ? newest.id : null,
      saved: [],
    };
  };

  const messages = (roomId: number, query: URLSearchParams): MessagePage => {
    const record = roomOr404(roomId);
    const cursors = (["before", "after", "around"] as const).filter((key) => query.has(key));

    if (cursors.length > 1) throw validation("base", "Pass at most one of before, after, around");

    const [cursor] = cursors;

    if (cursor === undefined) {
      return page(record, record.messages.length - PAGE_SIZE, record.messages.length);
    }

    const anchorId = Number(query.get(cursor));
    const index = record.messages.findIndex((message) => message.id === anchorId);

    if (index < 0) throw notFound("Message not found");

    switch (cursor) {
      case "before":
        return page(record, index - PAGE_SIZE, index);
      case "after":
        return page(record, index + 1, index + 1 + PAGE_SIZE);
      case "around":
        return page(record, index - PAGE_SIZE, index + 1 + PAGE_SIZE);
    }
  };

  // --- writes ---

  const markReadUpTo = (record: RoomRecord, messageId: number | null) => {
    record.membership = { ...record.membership, unreadAt: null, lastReadMessageId: messageId };
    record.mentionCount = 0;
  };

  /** Everything that happens when someone posts: the message, unreads, sidebar, the bot. */
  const createMessage = (
    record: RoomRecord,
    creatorId: number,
    markdown: string,
    clientMessageId: string,
    replyToMessageId: number | null,
    streaming: boolean,
  ): MessageDTO => {
    const createdAt = timestamp(Math.max(now(), Date.parse(record.room.updatedAt) + 1));

    const message: MessageDTO = {
      id: world.nextMessageId++,
      roomId: record.room.id,
      threadId: null,
      creatorId,
      clientMessageId,
      bodyHtml: renderMarkdown(markdown, mentionables()),
      markdownSource: markdown,
      systemNote: false,
      action: false,
      streaming,
      embedsSuppressed: false,
      replyToMessageId,
      forwardedFromMessageId: null,
      forwardedAt: null,
      forwardNote: null,
      editedAt: null,
      attachment: null,
      reactions: [],
      boosts: [],
      pinned: false,
      thread: null,
      createdAt,
      updatedAt: createdAt,
    };

    record.messages.push(message);
    record.room = { ...record.room, updatedAt: createdAt };

    const events: Outgoing[] = [
      { topic: `room:${record.room.id}`, type: "message.created", data: message },
    ];

    if (creatorId === VIEWER_ID) {
      markReadUpTo(record, message.id);
    } else {
      const mentioned = mentionsUser(message.bodyHtml, VIEWER_ID);
      const quiet = ["muted", "nothing"].includes(record.membership.involvement) && !mentioned;
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

    if (simulate && creatorId === VIEWER_ID && forBot) simulation.botReply(record.room.id);

    return message;
  };

  const updateMessage = (messageId: number, markdown: string, streaming: boolean) => {
    for (const record of world.rooms.values()) {
      const index = record.messages.findIndex((message) => message.id === messageId);
      const current = record.messages[index];

      if (current === undefined) continue;

      const updatedAt = timestamp(Math.max(now(), Date.parse(current.updatedAt) + 1));

      const message: MessageDTO = {
        ...current,
        bodyHtml: renderMarkdown(markdown, mentionables()),
        markdownSource: markdown,
        streaming,
        updatedAt,
      };

      record.messages[index] = message;
      hub.publish([{ topic: `room:${record.room.id}`, type: "message.updated", data: message }]);

      return;
    }
  };

  const createFromClient = (roomId: number, body: Json | undefined): MockResponse => {
    const record = roomOr404(roomId);
    const clientMessageId = stringField(body, "clientMessageId");
    const markdown = stringField(body, "markdownSource");
    const replyTo = intField(body, "replyToMessageId");

    if (clientMessageId === null || clientMessageId === "") {
      throw validation("clientMessageId", "Client message can't be blank");
    }

    const key = `${roomId}:${VIEWER_ID}:${clientMessageId}`;
    const duplicate = sentByClientId.get(key);

    if (duplicate !== undefined) return { status: 200, json: duplicate };

    if (markdown === null || markdown.trim() === "")
      throw validation("body", "Body can't be blank");

    if (markdown.length > SOURCE_LIMIT) {
      throw validation("body", `Body is too long (maximum is ${SOURCE_LIMIT} characters)`);
    }

    if (replyTo !== null && !record.messages.some((message) => message.id === replyTo)) {
      throw validation("replyToMessageId", "Reply to message must be on this timeline");
    }

    const message = createMessage(record, VIEWER_ID, markdown, clientMessageId, replyTo, false);

    sentByClientId.set(key, message);

    return { status: 201, json: message };
  };

  const markRead = (roomId: number): ReadState => {
    const record = roomOr404(roomId);

    markReadUpTo(record, record.messages.at(-1)?.id ?? null);
    hub.publish([{ topic: "user", type: "room.read", data: { roomId } }]);

    return { roomId, unread: false, firstUnreadMessageId: null };
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

    return { roomId, unread: true, firstUnreadMessageId: message.id };
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

    return createMessage(record, userId, markdown, seededUuid(random), null, false);
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
          : createMessage(record, userId, markdown, seededUuid(random), null, streaming);
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

  // --- routing ---

  const api = (request: MockRequest, path: string): MockResponse | Promise<MockResponse> => {
    const method = request.method.toUpperCase();
    const query = queryOf(request);

    if (method !== "GET" && method !== "HEAD") {
      const sent = headerOf(request, "x-csrf-token");

      if (sent !== csrf) {
        throw plainError(422, "InvalidAuthenticityToken", "Can't verify CSRF token authenticity.");
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
      case "POST /rooms/:id/messages": {
        roomOr404(roomId);

        if (!holding) return createFromClient(roomId, request.body);

        return new Promise<MockResponse>((resolve) => {
          held.push(() => resolve(respond(() => createFromClient(roomId, request.body))));
        });
      }

      case "POST /rooms/:id/read":
        return { status: 200, json: markRead(roomId) };
      case "DELETE /rooms/:id/read":
        return { status: 200, json: markUnread(roomId, request.body) };
      default:
        throw notFound(`No route for ${method} /api/v1${path}`);
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
      case "pause":
        server.pause();

        return ok;
      case "resume":
        server.resume();

        return ok;
      case "typing":
        typing(int("roomId"), int("userId"), flag("on", true));

        return ok;
      case "post":
        return { status: 201, json: post(int("roomId"), int("userId"), text("markdown")) };
      case "hold-sends":
        holdSends(flag("on", true));

        return ok;
      case "release-sends":
        release();

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

      case "reset":
        server.reset();

        return ok;
      default:
        throw notFound(`No mock control named ${action}`);
    }
  };

  const state = (): JsonRecord => ({
    epoch: hub.epoch(),
    seq: hub.seq(),
    connections: hub.connectionCount(),
    simulate,
    paused: simulation.paused(),
    holdingSends: holding,
    pendingSends: held.length,
    csrfToken: csrf,
    ids: SEED_IDS,
  });

  const respond = (run: () => MockResponse): MockResponse => {
    try {
      return run();
    } catch (error) {
      if (error instanceof HttpError) return { status: error.status, json: { error: error.error } };

      throw error;
    }
  };

  const server: MockServer = {
    async handle(request) {
      const path = request.path.split("?")[0] ?? "";

      try {
        if (path.startsWith("/api/v1/")) return await api(request, path.slice("/api/v1".length));

        if (path.startsWith("/__mock/")) return control(request, path.slice("/__mock/".length));

        throw notFound(`No route for ${path}`);
      } catch (error) {
        return respond(() => {
          throw error;
        });
      }
    },
    connect: (send, drop) => hub.connect(send, drop),
    csrfToken: () => csrf,
    pause: () => simulation.pause(),
    resume: () => simulation.resume(),
    typing,
    post,
    holdSends,
    releaseSends: release,
    pendingSends: () => held.length,
    rotateCsrf() {
      csrf = token(random);

      return csrf;
    },
    dropConnections: (bye = false) => hub.dropAll(bye),
    resync: () => hub.resyncAll("mock_resync"),
    setPresence,
    reset() {
      release();
      simulation.stop();
      world = seedWorld(now(), createRandom(seed));
      random = createRandom(seed * 7919 + 17);
      csrf = token(random);
      sentByClientId = new Map();
      restarts += 1;
      hub.restart(epochFor());

      if (simulate) {
        simulation.resume();
        simulation.start();
      }
    },
    dispose() {
      release();
      simulation.stop();
      hub.dropAll(false);
    },
    syncState: () => ({ epoch: hub.epoch(), seq: hub.seq(), connections: hub.connectionCount() }),
  };

  return server;
}

/** Whether a request path is one the mock serves. */
export function isMockPath(path: string): boolean {
  return path.startsWith("/api/v1/") || path.startsWith("/__mock/");
}
