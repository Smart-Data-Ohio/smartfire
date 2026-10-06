import { describe, expect, it } from "@effect/vitest";
import { DateTime, Effect, Exit, Schema } from "effect";
import { ApiErrorResponse, NotFound, RateLimited, Validation } from "../errors.ts";
import { Me } from "./me.ts";
import { MessageDTO } from "./message.ts";
import { Membership, Room } from "./room.ts";
import { ClientFrame, ServerFrame } from "./sync.ts";
import { User } from "./user.ts";

// The wire JSON below mirrors crates/api_types/src/tests.rs.
const userJson = {
  id: 7,
  name: "Ada Lovelace",
  role: "administrator",
  status: "active",
  bio: null,
  avatarUrl: "/users/7/avatar?v=1700000000",
  customStatus: { emoji: "🌴", text: "On a beach", expiresAt: null },
  createdAt: "2026-09-26T12:26:46.848Z",
};

const messageJson = {
  id: 9001,
  roomId: 12,
  threadId: null,
  creatorId: 7,
  clientMessageId: "4f1c7a0e-5b0e-4c55-9d0a-6f3b2d1e8c11",
  bodyHtml: "<p>Hello <strong>there</strong></p>",
  markdownSource: "Hello **there**",
  systemNote: false,
  action: false,
  streaming: false,
  embedsSuppressed: false,
  replyToMessageId: 8999,
  forwardedFromMessageId: null,
  editedAt: null,
  createdAt: "2026-10-06T09:15:00.123Z",
  updatedAt: "2026-10-06T09:15:00.123Z",
};

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
        theme: "system",
        textSize: "default",
        timeZone: "America/New_York",
        timeZoneExplicit: false,
        tourCompleted: true,
        voiceMode: "push_to_talk",
        pushToTalkKey: "`",
      },
      presenceSetting: "dnd",
      doNotDisturb: { enabled: true, until: "2026-10-06T17:00:00.000Z" },
      quietHours: { startMinute: 1320, endMinute: 420 },
      outOfOffice: null,
    };

    const roomJson = {
      id: 12,
      kind: "open",
      name: "general",
      iconName: null,
      creatorId: 7,
      createdAt: "2026-01-01T00:00:00.000Z",
      updatedAt: "2026-10-06T09:15:00.123Z",
    };

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
    };

    expect(Schema.encodeSync(Me)(Schema.decodeUnknownSync(Me)(meJson))).toEqual(meJson);
    expect(Schema.encodeSync(Room)(Schema.decodeUnknownSync(Room)(roomJson))).toEqual(roomJson);
    expect(
      Schema.encodeSync(Membership)(Schema.decodeUnknownSync(Membership)(membershipJson)),
    ).toEqual(membershipJson);
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
    expect(Schema.decodeUnknownSync(ClientFrame)({ t: "hb" })).toEqual({ t: "hb" });
    expect(() => Schema.decodeUnknownSync(ClientFrame)({ t: "present" })).toThrowError();
  });

  it("decodes a batch with each event's data", () => {
    const frame = Schema.decodeUnknownSync(ServerFrame)({
      t: "batch",
      events: [
        { seq: 48212, topic: "room:12", type: "message.created", data: messageJson },
        { seq: 48213, topic: "room:12", type: "typing", data: { userId: 7, on: false } },
      ],
    });

    expect(frame.t).toBe("batch");
    expect(frame.t === "batch" && frame.events.map((event) => event.type)).toEqual([
      "message.created",
      "typing",
    ]);
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
