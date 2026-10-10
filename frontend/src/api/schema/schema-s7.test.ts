import { describe, expect, it } from "@effect/vitest";
import { DateTime, Schema } from "effect";
import {
  AccountSettings,
  BackupCodes,
  Connection,
  CreatePushSubscription,
  IntegrationChange,
  IntegrationToken,
  PushPublicKey,
  PushSubscriptionList,
  Reauthentication,
  SessionList,
  Settings,
  StatusExpiry,
  TwoFactorChange,
  UpdateAppearance,
  UpdateAvatar,
  UpdateCalls,
  UpdateNotifications,
  UpdateProfile,
  UpdateStatus,
} from "./settings.ts";

// The wire JSON below mirrors crates/api_types/src/tests_s7.rs.
const settingsJson = {
  revision: 3,
  evaluatedAt: "2026-10-10T12:00:00.000000000Z",
  profile: {
    userId: 7,
    name: "Ada Lovelace",
    emailAddress: "ada@example.com",
    bio: null,
    avatarUrl: "/users/7/avatar?v=1700000000",
    avatarAttached: true,
    hasPassword: true,
    githubLogin: "ada",
    githubVerified: false,
    bot: false,
  },
  appearance: {
    theme: "dark",
    textSize: "default",
    timeZone: "Europe/London",
    timeZones: [{ label: "(GMT+00:00) London", value: "Europe/London" }],
    appearancePreferences: null,
  },
  notifications: {
    defaultNotificationLevel: "everything",
    roomNotificationLevels: {},
    roomMuteUntil: {},
    dndEnabled: false,
    quietHoursEnabled: true,
    quietHoursStart: "22:00",
    quietHoursEnd: "07:30",
    meetingDndEnabled: false,
    oooNotifyEnabled: true,
    allowedPeople: [{ userId: 8, name: "Grace Hopper" }],
    keywordAlerts: ["deploy freeze"],
    inbox: [
      {
        key: "github_review_requests",
        label: "GitHub review requests",
        description: "When someone asks you to review a pull request.",
        enabled: true,
      },
    ],
  },
  status: {
    presenceSetting: "auto",
    customStatusEmoji: "🌴",
    customStatusText: "On a beach",
    customStatusExpiresAt: null,
    meetingStatusEnabled: true,
    oooCalendarEnabled: false,
    oooUntil: "2026-10-09T17:00:00.000Z",
    oooManual: true,
    oooNote: "Back Friday",
    calendarError: null,
  },
  calls: { voiceMode: "push_to_talk", pushToTalkKey: "`" },
  integrations: {
    google: {
      signInConfigured: true,
      identityEmail: null,
      calendarConfigured: true,
      connected: true,
      calendar: true,
      drive: false,
      email: "ada@example.com",
    },
    github: { state: "connected", name: "ada", workspace: null, appToken: false },
    githubAppConfigured: false,
    fizzy: { state: "rejected", reason: "The token was revoked." },
    managePath: "/users/me/profile",
    slackImportPath: "/slack/imports",
  },
} as const;

const roundTrips = <S extends Schema.Codec<unknown, unknown>>(schema: S, wire: S["Encoded"]) =>
  expect(Schema.encodeSync(schema)(Schema.decodeUnknownSync(schema)(wire))).toEqual(wire);

describe("S7 settings schemas", () => {
  it("pins the push key and enrollment body to the Rust wire contract", () => {
    roundTrips(PushPublicKey, { publicKey: null });
    roundTrips(PushPublicKey, { publicKey: "public-key" });
    roundTrips(CreatePushSubscription, {
      endpoint: "https://fcm.googleapis.com/push",
      p256dhKey: "p256",
      authKey: "auth",
    });
    expect(() =>
      Schema.decodeUnknownSync(CreatePushSubscription)({
        endpoint: "https://fcm.googleapis.com/push",
        p256dhKey: 2,
        authKey: "auth",
      }),
    ).toThrow();
  });
  it("decode the settings page", () => {
    const settings = Schema.decodeUnknownSync(Settings)(settingsJson);

    expect(settings.status.oooUntil && DateTime.toEpochMillis(settings.status.oooUntil)).toBe(
      Date.UTC(2026, 9, 9, 17),
    );
    expect(settings.integrations.github.state).toBe("connected");
    expect(settings.notifications.allowedPeople[0]?.name).toBe("Grace Hopper");
    roundTrips(Settings, settingsJson);
    roundTrips(Connection, { state: "missing" });
  });

  it("round-trip a connect and its answer", () => {
    roundTrips(IntegrationToken, { accessToken: "github_pat_1" });
    roundTrips(IntegrationChange, {
      integrations: settingsJson.integrations,
      notice: "GitHub connected as ada.",
    });
  });

  it("round-trip the write bodies", () => {
    roundTrips(UpdateProfile, {
      name: "Ada",
      emailAddress: null,
      currentPassword: "secret",
      password: null,
      bio: null,
      githubLogin: null,
    });
    roundTrips(UpdateAvatar, { signedId: "eyJf--1" });
    roundTrips(UpdateAppearance, {
      theme: "system",
      textSize: null,
      timeZone: "",
      appearancePreferences: null,
    });
    roundTrips(UpdateNotifications, {
      defaultNotificationLevel: null,
      roomNotification: null,
      roomMute: null,
      dndEnabled: true,
      quietHoursEnabled: null,
      quietHoursStart: null,
      quietHoursEnd: null,
      meetingDndEnabled: null,
      oooNotifyEnabled: null,
      keywordAlerts: ["prod"],
      inbox: { mentions: false },
    });
    roundTrips(UpdateStatus, {
      presenceSetting: "dnd",
      customStatusEmoji: null,
      customStatusText: null,
      customStatusExpiresIn: "minutes_30",
      clearCustomStatus: null,
      meetingStatusEnabled: null,
      oooCalendarEnabled: null,
      oooPreset: "custom",
      oooUntilCustom: "2026-10-09T17:00",
      oooNote: null,
      clearOoo: null,
    });
    roundTrips(UpdateCalls, { voiceMode: "voice_activity", pushToTalkKey: null });

    for (const expiry of ["hour_1", "hours_4", "today", "week", "never"] as const) {
      roundTrips(StatusExpiry, expiry);
    }

    expect(() => Schema.decodeUnknownSync(StatusExpiry)("forever")).toThrow();
  });

  it("round-trip sessions and push subscriptions", () => {
    roundTrips(SessionList, {
      sessions: [
        {
          id: 3,
          current: true,
          description: "Firefox on macOS",
          ipAddress: "203.0.113.9",
          lastActiveAt: "2026-10-06T10:00:00.000Z",
          createdAt: "2026-10-01T09:00:00.000Z",
        },
      ],
      notice: "Signed out 1 other session.",
    });
    roundTrips(PushSubscriptionList, {
      pushSubscriptions: [
        {
          id: 4,
          endpoint: "https://push.example/abc",
          browser: "Chrome",
          version: "141",
          platform: "Android",
        },
      ],
    });
  });

  it("round-trip the account panels and the two-step writes", () => {
    const panel = {
      confirmedAt: "2026-10-07T10:00:00.000Z",
      google: true,
      hasPassword: false,
      devices: [
        {
          id: 7,
          description: "Firefox",
          ipAddress: "203.0.113.9",
          lastUsedAt: "2026-10-07T11:00:00.000Z",
        },
      ],
    };

    roundTrips(AccountSettings, {
      sharedRooms: [
        { roomId: 12, name: "Everyone", involvement: "everything", direct: false },
        { roomId: 14, name: "Old room", involvement: null, direct: false },
      ],
      directRooms: [{ roomId: 13, name: "Grace", involvement: "mentions", direct: true }],
      twoFactor: panel,
      transferUrl: "https://chat.example/session/transfers/signed",
      transferQrSvg: '<svg xmlns="http://www.w3.org/2000/svg"/>',
    });
    roundTrips(Reauthentication, { reauth: "" });
    roundTrips(BackupCodes, { codes: ["1234-5678"] });
    roundTrips(TwoFactorChange, {
      notice: "Device forgotten. It will ask for a code at next sign-in.",
      twoFactor: {
        ...panel,
        devices: [{ id: 6, description: "Unknown browser", ipAddress: null, lastUsedAt: null }],
      },
    });
  });
});
