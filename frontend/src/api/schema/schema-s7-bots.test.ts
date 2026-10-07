import { describe, expect, it } from "@effect/vitest";
import { DateTime, Schema } from "effect";
import {
  BotChange,
  BotKey,
  BotList,
  BotRemoved,
  ConnectGithub,
  CreateBot,
  CreateCredential,
  CreateGrant,
  CredentialCreated,
  CredentialState,
  GrantList,
  UpdateBot,
} from "./bots.ts";

// The wire JSON below mirrors crates/api_types/src/tests_s7_bots.rs.
const agent = {
  id: 11,
  provider: "anthropic",
  runtime: null,
  description: "Bends things",
  dailyMessageCap: 50,
  dailyBoardPostCap: null,
  dailyExternalActionCap: 5,
  usage: "2/50 messages · 0 board posts · 0/5 external actions",
  suspended: false,
  ledgerUrl: "/agents/11/events",
  approvalsUrl: "/agents/11/approvals",
};

const roundTrips = <S extends Schema.Codec<unknown, unknown>>(schema: S, wire: S["Encoded"]) =>
  expect(Schema.encodeSync(schema)(Schema.decodeUnknownSync(schema)(wire))).toEqual(wire);

describe("S7 bot schemas", () => {
  it("round-trip the list and a bot", () => {
    roundTrips(BotList, {
      bots: [
        {
          id: 9,
          name: "Bender",
          avatarUrl: "/users/9/avatar?v=1",
          icon: { kind: "emoji", title: "Robot", character: "🤖" },
          ownership: "Workspace agent · Owned by Grace",
          rooms: [
            {
              id: 3,
              name: "Ops",
              messageCommand: "curl -d 'Hello!' https://chat.example/rooms/3/BOT_KEY/messages",
              attachmentCommand:
                'curl -F "attachment=@/path/to/file" https://chat.example/rooms/3/BOT_KEY/messages',
            },
          ],
        },
      ],
    });
    roundTrips(BotChange, {
      bot: {
        id: 9,
        name: "Bender",
        avatarUrl: "/users/9/avatar?v=1",
        avatarAttached: false,
        iconName: "acme",
        icon: { kind: "image", title: "Acme", url: "/icons/acme.svg" },
        webhookUrl: "https://example.com/hook",
        canAdminister: true,
        agent,
        signingSecret: null,
        github: { login: "bender-bot", usable: false, disconnectedReason: "Bad credentials" },
      },
      notice: "Agent suspended; 0 approvals cancelled.",
    });
    roundTrips(BotRemoved, { id: 9 });

    expect(() =>
      Schema.decodeUnknownSync(BotList)({
        bots: [
          { id: 9, name: "B", avatarUrl: "", icon: { kind: "gif" }, ownership: "", rooms: [] },
        ],
      }),
    ).toThrow();
  });

  it("round-trip the bot writes", () => {
    roundTrips(CreateBot, {
      name: "Robo",
      iconName: null,
      webhookUrl: "https://example.com/robo",
      avatar: "signed-blob",
    });
    roundTrips(BotKey, {
      id: 9,
      name: "Robo",
      key: "9-abc",
      exampleCommand: "curl -d 'Hello!' https://chat.example/rooms/ROOM_ID/9-abc/messages",
    });
    roundTrips(UpdateBot, {
      name: "Robo",
      iconName: "",
      webhookUrl: null,
      avatar: null,
      agent: {
        provider: null,
        runtime: null,
        description: null,
        dailyMessageCap: "lots",
        dailyBoardPostCap: null,
        dailyExternalActionCap: null,
      },
    });
    roundTrips(ConnectGithub, { accessToken: "ghp_x" });
  });

  it("round-trip credentials and grants", () => {
    const created = {
      secret: "cfa_secret",
      credentials: {
        botId: 9,
        botName: "Bender",
        canIssue: true,
        credentials: [
          {
            id: 4,
            name: "ci",
            lastFour: "cret",
            createdBy: "Grace Hopper",
            createdAt: "2026-03-02T16:00:00.000Z",
            lastUsedAt: null,
            expiresAt: "2030-01-02T03:04:00-05:00",
            state: "expired",
          },
        ],
      },
    } as const;

    const decoded = Schema.decodeUnknownSync(CredentialCreated)(created);
    const credential = decoded.credentials.credentials[0];

    expect(credential && DateTime.toEpochMillis(credential.createdAt)).toBe(
      Date.UTC(2026, 2, 2, 16),
    );
    roundTrips(CredentialCreated, created);
    roundTrips(CreateCredential, { name: "ci", expiresAt: "2026-10-31T17:00" });
    roundTrips(GrantList, {
      botId: 9,
      botName: "Bender",
      canGrant: false,
      legacy: false,
      grants: [
        {
          id: 2,
          capability: "react",
          roomName: "Workspace-wide",
          grantedBy: "Grace Hopper",
          createdAt: "2026-03-02T16:00:00.000Z",
          revoked: true,
        },
      ],
      capabilities: ["read_messages", "react"],
      rooms: [{ id: 3, name: "Ops" }],
    });
    roundTrips(CreateGrant, { capability: "react", roomId: null });

    expect(() => Schema.decodeUnknownSync(CredentialState)("lost")).toThrow();
  });
});
