import { describe, expect, it } from "@effect/vitest";
import { DateTime, Effect, Schema } from "effect";
import { ApiErrorResponse, Unavailable } from "../errors.ts";
import {
  HuddleCredentials,
  HuddleDetail,
  HuddleNotice,
  HuddlePresenceList,
  ModerateHuddle,
} from "./huddle.ts";
import {
  ChangeStageRole,
  LowerHand,
  StageDetail,
  StartStageStream,
  StopStageStream,
} from "./stage.ts";
import { ServerFrame } from "./sync.ts";

// The wire JSON below mirrors crates/api_types/src/tests_s5.rs.
const userJson = {
  id: 7,
  name: "Ada Lovelace",
  role: "administrator",
  status: "active",
  bio: null,
  avatarUrl: "/users/7/avatar?v=1700000000",
  hasAvatar: true,
  customStatus: null,
  avatarIcon: null,
  agent: null,
  createdAt: "2026-09-26T12:26:46.848Z",
  updatedAt: "2026-09-26T12:26:46.848Z",
} as const;

const participantJson = {
  userId: 7,
  membershipId: 31,
  identities: ["campfire-participant-0f3a9c", "campfire-participant-77be10"],
  serverMuted: false,
} as const;

const presenceJson = { roomId: 12, participants: [participantJson], live: false } as const;

const stageJson = {
  roomId: 40,
  members: [
    { membershipId: 31, userId: 7, role: "host", handRaisedAt: null, serverMuted: false },
    {
      membershipId: 32,
      userId: 8,
      role: "listener",
      handRaisedAt: "2026-10-06T11:02:03.456Z",
      serverMuted: true,
    },
  ],
  live: {
    id: 5,
    membershipId: 31,
    userId: 7,
    identity: "campfire-participant-0f3a9c",
    quality: "1080p15",
    startedAt: "2026-10-06T11:00:00.000Z",
  },
} as const;

/** Decoding then encoding gives back exactly the wire JSON. */
const roundTrips = <S extends Schema.Codec<unknown, unknown>>(schema: S, wire: S["Encoded"]) =>
  expect(Schema.encodeSync(schema)(Schema.decodeUnknownSync(schema)(wire))).toEqual(wire);

describe("S5 DTO schemas", () => {
  it("round-trip call presence, a room's call and its credentials", () => {
    roundTrips(HuddlePresenceList, { rooms: [presenceJson], users: [userJson] });
    roundTrips(HuddlePresenceList, { rooms: [], users: [] });
    roundTrips(HuddleDetail, {
      roomName: "Grace Hopper",
      presence: presenceJson,
      users: [userJson],
    });
    roundTrips(HuddleCredentials, {
      url: "wss://huddles.example.com",
      token: "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiJ4In0.c2ln",
      identity: "campfire-participant-0f3a9c",
      grantId: 901,
      roomId: 12,
      roomName: "Grace Hopper",
      canPublish: true,
    });
    roundTrips(ModerateHuddle, { membershipId: 32, action: "disconnect" });
    expect(() =>
      Schema.decodeUnknownSync(ModerateHuddle)({ membershipId: 32, action: "ban" }),
    ).toThrow();
  });

  it("decode a stage, its hands and its stream", () => {
    const detail = Schema.decodeUnknownSync(StageDetail)({ stage: stageJson, users: [userJson] });
    const hand = detail.stage.members[1]?.handRaisedAt;

    expect(hand && DateTime.toEpochMillis(hand)).toBe(Date.UTC(2026, 9, 6, 11, 2, 3, 456));
    expect(detail.stage.live?.quality).toBe("1080p15");
    expect(Schema.encodeSync(StageDetail)(detail)).toEqual({ stage: stageJson, users: [userJson] });
    roundTrips(StageDetail, { stage: { ...stageJson, live: null }, users: [] });
  });

  it("round-trip the stage requests", () => {
    roundTrips(ChangeStageRole, { role: "speaker" });
    roundTrips(LowerHand, { membershipId: null });
    roundTrips(LowerHand, { membershipId: 32 });
    roundTrips(StartStageStream, { quality: "720p15" });
    roundTrips(StopStageStream, { streamId: 5 });
    roundTrips(StopStageStream, { streamId: null });
    expect(() => Schema.decodeUnknownSync(ChangeStageRole)({ role: "owner" })).toThrow();
    expect(() => Schema.decodeUnknownSync(StartStageStream)({ quality: "4k60" })).toThrow();
  });

  it("tell the notices apart by kind", () => {
    const joined = Schema.decodeUnknownSync(HuddleNotice)({
      kind: "joined",
      roomId: 12,
      roomName: "general",
      userId: 8,
      userName: "Grace Hopper",
      inCall: true,
      rejoin: false,
    });

    expect(joined.kind === "joined" && joined.inCall).toBe(true);
    roundTrips(HuddleNotice, {
      kind: "left",
      roomId: 12,
      roomName: "general",
      userId: 8,
      userName: "Grace Hopper",
    });
    roundTrips(HuddleNotice, { kind: "ended", roomId: 12 });
  });

  it.effect("decode an unconfigured server's 503", () =>
    Effect.gen(function* () {
      const body = yield* Schema.decodeUnknownEffect(Schema.fromJsonString(ApiErrorResponse))(
        '{"error":{"_tag":"Unavailable","message":"Huddles are not configured"}}',
      );

      expect(body.error).toBeInstanceOf(Unavailable);
      expect(body.error.message).toBe("Huddles are not configured");
    }),
  );
});

describe("S5 sync events", () => {
  it("decode in a batch", () => {
    const frame = Schema.decodeUnknownSync(ServerFrame)({
      t: "batch",
      events: [
        { seq: 1, topic: "user", type: "huddle.presence", data: presenceJson },
        {
          seq: 2,
          topic: "user",
          type: "huddle.role",
          data: { roomId: 40, stageRole: "speaker", serverMuted: false },
        },
        {
          seq: 3,
          topic: "user",
          type: "huddle.role",
          data: { roomId: 12, stageRole: null, serverMuted: true },
        },
        { seq: 4, topic: "user", type: "huddle.notice", data: { kind: "ended", roomId: 12 } },
        {
          seq: 5,
          topic: "user",
          type: "huddle.ring",
          data: {
            activityItemId: 77,
            event: "started",
            state: "unread",
            roomId: 12,
            roomName: "Grace Hopper",
            callerName: "Grace Hopper",
            silent: true,
          },
        },
        {
          seq: 6,
          topic: "user",
          type: "huddle.ring",
          data: {
            activityItemId: null,
            event: "ended",
            state: "handled",
            roomId: 12,
            roomName: "general",
            callerName: "Grace Hopper",
            silent: false,
          },
        },
        { seq: 7, topic: "room:40", type: "stage.updated", data: stageJson },
        { seq: 8, topic: "user", type: "stage.stream.stopped", data: { roomId: 40 } },
      ],
    });

    expect(frame.t === "batch" && frame.events.map((event) => event.type)).toEqual([
      "huddle.presence",
      "huddle.role",
      "huddle.role",
      "huddle.notice",
      "huddle.ring",
      "huddle.ring",
      "stage.updated",
      "stage.stream.stopped",
    ]);
  });
});
