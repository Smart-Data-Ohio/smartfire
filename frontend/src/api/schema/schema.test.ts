import { describe, expect, it } from "@effect/vitest";
import { DateTime, Effect, Exit, Schema } from "effect";
import { ApiErrorResponse, NotFound, RateLimited, Validation } from "../errors.ts";
import { Me } from "./me.ts";
import { CreateMessage, MessageDTO, MessagePage } from "./message.ts";
import { PresenceList } from "./presence.ts";
import { ReadState } from "./read.ts";
import { Membership, Room, RoomDetail } from "./room.ts";
import { Sidebar } from "./sidebar.ts";
import { ClientFrame, ServerFrame } from "./sync.ts";
import { User, UserList } from "./user.ts";

// The wire JSON below mirrors crates/api_types/src/tests.rs.
const userJson = {
  id: 7,
  name: "Ada Lovelace",
  role: "administrator",
  status: "active",
  bio: null,
  avatarUrl: "/users/7/avatar?v=1700000000",
  hasAvatar: true,
  customStatus: { emoji: "🌴", text: "On a beach", expiresAt: null },
  avatarIcon: null,
  agent: null,
  createdAt: "2026-09-26T12:26:46.848Z",
  updatedAt: "2026-09-26T12:26:46.848000Z",
  accountName: "Ada Lovelace",
  pronouns: null,
} as const;

const messageJson = {
  id: 9001,
  roomId: 12,
  threadId: null,
  creatorId: 7,
  clientMessageId: "4f1c7a0e-5b0e-4c55-9d0a-6f3b2d1e8c11",
  sound: null,
  bodyHtml: "<p>Hello <strong>there</strong></p>",
  markdownSource: "Hello **there**",
  systemNote: false,
  action: false,
  streaming: false,
  embedsSuppressed: false,
  replyToMessageId: 8999,
  forwardedFromMessageId: null,
  forwardedAt: null,
  forwardNote: null,
  editedAt: null,
  attachment: null,
  reactions: [],
  boosts: [],
  pinned: false,
  thread: null,
  poll: null,
  cards: [],
  cardsAsOf: "2026-10-06T09:15:00.200Z",
  steps: [],
  createdAt: "2026-10-06T09:15:00.123Z",
  updatedAt: "2026-10-06T09:15:00.123Z",
} as const;

const roomJson = {
  id: 12,
  kind: "open",
  name: "general",
  iconName: null,
  creatorId: 7,
  createdAt: "2026-01-01T00:00:00.000Z",
  updatedAt: "2026-10-06T09:15:00.123Z",
  topic: null,
} as const;

const membershipJson = {
  id: 40,
  roomId: 12,
  userId: 7,
  involvement: "mentions",
  unreadAt: "2026-10-06T09:15:00.123Z",
  lastReadMessageId: 8999,
  roomCategoryId: null,
  favoritePosition: 0,
  stageRole: "host",
} as const;

const sidebarRowJson = {
  revision: 0,
  evaluatedAt: "2026-10-10T12:00:00.000000000Z",
  room: roomJson,
  membership: membershipJson,
  displayName: "general",
  directMemberIds: [],
  unreadCount: 4,
  mentionCount: 1,
  notificationCount: 1,
  threadNotificationCount: 0,
} as const;

/** Decoding then encoding gives back exactly the wire JSON. */
const roundTrips = <S extends Schema.Codec<unknown, unknown>>(schema: S, wire: S["Encoded"]) =>
  expect(Schema.encodeSync(schema)(Schema.decodeUnknownSync(schema)(wire))).toEqual(wire);

describe("DTO schemas", () => {
  it("decodes a message to branded ids and DateTime values and encodes it back unchanged", () => {
    const message = Schema.decodeUnknownSync(MessageDTO)(messageJson);

    expect(message.id).toBe(9001);
    expect(DateTime.toEpochMillis(message.createdAt)).toBe(Date.UTC(2026, 9, 6, 9, 15, 0, 123));
    expect(Schema.encodeSync(MessageDTO)(message)).toEqual(messageJson);
  });

  it("round-trips me, rooms and memberships", () => {
    const meJson = {
      user: userJson,
      emailAddress: "ada@example.com",
      preferences: {
        settingsRevision: 4,
        theme: "system",
        textSize: "default",
        appearancePreferences: null,
        timeZone: "America/New_York",
        timeZoneExplicit: false,
        tourCompleted: true,
        voiceMode: "push_to_talk",
        pushToTalkKey: "`",
      },
      presenceSetting: "dnd",
      doNotDisturb: { enabled: true, until: "2026-10-06T17:00:00.000Z" },
      quietHours: { startMinute: 1320, endMinute: 420 },
      chatSounds: { muted: true, quietHours: null, timeZone: "UTC", quietWindows: [] },
      outOfOffice: null,
      lastRoomId: 12,
    };

    expect(Schema.encodeSync(Me)(Schema.decodeUnknownSync(Me)(meJson))).toEqual(meJson);
    expect(Schema.encodeSync(Room)(Schema.decodeUnknownSync(Room)(roomJson))).toEqual(roomJson);
    expect(
      Schema.encodeSync(Membership)(Schema.decodeUnknownSync(Membership)(membershipJson)),
    ).toEqual(membershipJson);
  });

  it("round-trips the S1 shapes: room detail, message page, sidebar, reads and presence", () => {
    roundTrips(RoomDetail, {
      room: roomJson,
      membership: membershipJson,
      displayName: "general",
      memberCount: 23,
      pinsCount: 2,
      directMemberIds: [],
      memberPreviewIds: [7],
      users: [userJson],
      unread: { firstUnreadMessageId: 9000, count: 4 },
    });
    roundTrips(MessagePage, {
      messages: [messageJson],
      users: [userJson],
      before: 9001,
      after: null,
      saved: [{ messageId: 9001, savedItemId: 31 }],
    });
    roundTrips(CreateMessage, {
      clientMessageId: "0192f0c4-7e8a-7b3c-9d0a-6f3b2d1e8c11",
      markdownSource: "Ship it",
      replyToMessageId: null,
      replyNotifyAuthor: null,
      attachmentSignedId: null,
    });
    roundTrips(Sidebar, {
      rows: [sidebarRowJson],
      categories: [{ id: 3, name: "Projects", collapsed: false, position: 0 }],
      users: [userJson],
      directPlaceholderUserIds: [7],
      canCreateRooms: true,
    });
    roundTrips(ReadState, { roomId: 12, unread: true, firstUnreadMessageId: 9000, unreadCount: 4 });
    roundTrips(UserList, { users: [userJson] });
    roundTrips(PresenceList, {
      presences: [{ userId: 7, presence: "dnd", statusText: "In a meeting" }],
    });
  });

  it("rejects an omitted nullable key, a fractional id and an unknown literal", () => {
    const { bio: _bio, ...withoutBio } = userJson;

    expect(Schema.decodeUnknownSync(User)(userJson).bio).toBeNull();
    expect(() => Schema.decodeUnknownSync(User)(withoutBio)).toThrowError();
    expect(() => Schema.decodeUnknownSync(MessageDTO)({ ...messageJson, id: 1.5 })).toThrowError();
    expect(() =>
      Schema.decodeUnknownSync(Room)({
        id: 1,
        kind: "lobby",
        name: null,
        iconName: null,
        creatorId: 1,
        createdAt: "2026-01-01T00:00:00.000Z",
        updatedAt: "2026-01-01T00:00:00.000Z",
      }),
    ).toThrowError();
  });
});

describe("sync frames", () => {
  it("decodes the protocol's client frames", () => {
    const hello = {
      t: "hello",
      v: 1,
      resume: { epoch: "b7c1", seq: 48211 },
      topics: ["room:12", "thread:88"],
    };

    expect(Schema.decodeUnknownSync(ClientFrame)(hello)).toEqual(hello);
    expect(Schema.decodeUnknownSync(ClientFrame)({ t: "hb", active: true })).toEqual({
      t: "hb",
      active: true,
    });
    expect(() => Schema.decodeUnknownSync(ClientFrame)({ t: "present" })).toThrowError();
  });

  it("decodes a batch with each event's data", () => {
    const frame = Schema.decodeUnknownSync(ServerFrame)({
      t: "batch",
      events: [
        { seq: 48212, topic: "room:12", type: "message.created", data: messageJson },
        { seq: 48213, topic: "room:12", type: "typing", data: { userId: 7, on: false } },
        {
          seq: 48214,
          topic: "user",
          type: "room.unread",
          data: { roomId: 12, messageId: 9001, mentioned: true },
        },
        { seq: 48215, topic: "user", type: "room.read", data: { roomId: 12 } },
        {
          seq: 48216,
          topic: "user",
          type: "sidebar.row.upserted",
          data: sidebarRowJson,
        },
        { seq: 48217, topic: "user", type: "sidebar.row.removed", data: { roomId: 12 } },
        {
          seq: 48218,
          topic: "user",
          type: "presence",
          data: { userId: 7, presence: "idle", statusText: null },
        },
      ],
    });

    expect(frame.t).toBe("batch");
    expect(frame.t === "batch" && frame.events.map((event) => event.type)).toEqual([
      "message.created",
      "typing",
      "room.unread",
      "room.read",
      "sidebar.row.upserted",
      "sidebar.row.removed",
      "presence",
    ]);
    expect(Schema.decodeUnknownSync(ServerFrame)({ t: "ping" })).toEqual({ t: "ping" });
  });
});

describe("API errors", () => {
  // Response bodies as the server sends them; the client decodes the text.
  const decodeBody = Schema.decodeUnknownEffect(Schema.fromJsonString(ApiErrorResponse));

  it.effect("decodes each tagged error into its class", () =>
    Effect.gen(function* () {
      const notFound = yield* decodeBody('{"error":{"_tag":"NotFound","message":"Not found"}}');

      const validation = yield* decodeBody(
        '{"error":{"_tag":"Validation","message":"Name can\'t be blank","fields":{"name":["can\'t be blank"]}}}',
      );

      const limited = yield* decodeBody(
        '{"error":{"_tag":"RateLimited","message":"Slow down","retryAfter":30}}',
      );

      expect(notFound.error).toBeInstanceOf(NotFound);
      expect(validation.error).toBeInstanceOf(Validation);
      expect(limited.error).toBeInstanceOf(RateLimited);
      expect(limited.error).toMatchObject({ message: "Slow down", retryAfter: 30 });
    }),
  );

  it.effect("rejects an unknown tag", () =>
    Effect.gen(function* () {
      const result = yield* Effect.exit(decodeBody('{"error":{"_tag":"Teapot","message":"418"}}'));

      expect(Exit.isFailure(result)).toBe(true);
    }),
  );
});
