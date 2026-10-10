import { Schema } from "effect";
import { describe, expect, it } from "vitest";
import {
  IntegrationChange as IntegrationChangeSchema,
  PushPublicKey as PushPublicKeySchema,
  PushSubscriptionList as PushSubscriptionListSchema,
  SessionList as SessionListSchema,
  Settings as SettingsSchema,
} from "../../src/api/schema/settings.ts";
import type { IntegrationChange } from "../../src/gen/IntegrationChange.ts";
import type { PushPublicKey } from "../../src/gen/PushPublicKey.ts";
import type { PushSubscriptionList } from "../../src/gen/PushSubscriptionList.ts";
import type { SessionList } from "../../src/gen/SessionList.ts";
import type { Settings } from "../../src/gen/Settings.ts";
import type { Json } from "../json.ts";
import { USER_IDS } from "../seed.ts";
import { MOCK_PASSWORD } from "./settings.ts";
import { collect, errorOf, expectStatus, get, harness, NOW } from "./testing.ts";

it("publishes the public identity when the profile nickname changes", async () => {
  const { server } = harness();
  const events = collect(server);

  await expectStatus(
    server,
    "PATCH",
    "/api/v1/settings/profile",
    { nickname: "NickExample", pronouns: "they/them" },
    200,
  );

  expect(events.find((event) => event.type === "user.updated")).toMatchObject({
    topic: "user",
    type: "user.updated",
    data: { id: USER_IDS.riel, name: "NickExample", pronouns: "they/them" },
  });
});

/** Every key of a write body, `null` unless given. */
function body(keys: readonly string[], given: Readonly<Record<string, Json>>): Json {
  return Object.fromEntries(keys.map((key) => [key, given[key] ?? null]));
}

describe("the mock's browser push enrollment", () => {
  it("deduplicates the whole key triple and refreshes the device list", async () => {
    const { server } = harness();
    const path = "/api/v1/settings/push_subscriptions";

    const enrollment = {
      endpoint: "https://fcm.googleapis.com/fcm/send/enrolled",
      p256dhKey: "p256",
      authKey: "auth",
    };

    const saved = await expectStatus<PushSubscriptionList>(server, "POST", path, enrollment, 200);

    expect(saved.pushSubscriptions).toHaveLength(3);
    const repeat = await expectStatus<PushSubscriptionList>(server, "POST", path, enrollment, 200);

    expect(repeat).toEqual(saved);

    const rotated = await expectStatus<PushSubscriptionList>(
      server,
      "POST",
      path,
      { ...enrollment, authKey: "rotated" },
      200,
    );

    expect(rotated.pushSubscriptions).toHaveLength(4);
    expect(await get<PushSubscriptionList>(server, path)).toEqual(rotated);
  });

  it("rejects malformed and non-push-service endpoints without adding a device", async () => {
    const { server } = harness();
    const path = "/api/v1/settings/push_subscriptions";
    const before = await get<PushSubscriptionList>(server, path);

    for (const body of [
      null,
      {},
      { endpoint: "https://fcm.googleapis.com/push", p256dhKey: 2, authKey: "auth" },
      { endpoint: "http://fcm.googleapis.com/push", p256dhKey: "p256", authKey: "auth" },
      { endpoint: "https://private.example/push", p256dhKey: "p256", authKey: "auth" },
    ]) {
      await expectStatus(server, "POST", path, body, 422);
      expect(await get<PushSubscriptionList>(server, path)).toEqual(before);
    }
  });
});

const PROFILE = ["name", "emailAddress", "currentPassword", "password", "bio", "githubLogin"];

const STATUS = [
  "presenceSetting",
  "customStatusEmoji",
  "customStatusText",
  "customStatusExpiresIn",
  "clearCustomStatus",
  "meetingStatusEnabled",
  "oooCalendarEnabled",
  "oooPreset",
  "oooUntilCustom",
  "oooNote",
  "clearOoo",
];

describe("the mock's settings", () => {
  it("answer the contract's shapes", async () => {
    const { server } = harness();

    Schema.decodeUnknownSync(SettingsSchema)(await get<Settings>(server, "/api/v1/settings"));
    Schema.decodeUnknownSync(SessionListSchema)(
      await get<SessionList>(server, "/api/v1/settings/sessions"),
    );
    Schema.decodeUnknownSync(PushSubscriptionListSchema)(
      await get<PushSubscriptionList>(server, "/api/v1/settings/push_subscriptions"),
    );
    Schema.decodeUnknownSync(PushPublicKeySchema)(
      await get<PushPublicKey>(server, "/api/v1/settings/push_subscriptions/key"),
    );
  });

  it("need the current password to change the email", async () => {
    const { server } = harness();
    const path = "/api/v1/settings/profile";

    const missing = await expectStatus<Json>(
      server,
      "PATCH",
      path,
      body(PROFILE, { emailAddress: "new@example.com" }),
      422,
    );

    expect(errorOf(missing).tag).toBe("Validation");

    await expectStatus(
      server,
      "PATCH",
      path,
      body(PROFILE, { emailAddress: "new@example.com", currentPassword: "wrong" }),
      422,
    );

    const saved = await expectStatus<Settings>(
      server,
      "PATCH",
      path,
      body(PROFILE, {
        name: "Riel S.",
        emailAddress: "new@example.com",
        currentPassword: MOCK_PASSWORD,
      }),
      200,
    );

    expect(saved.profile).toMatchObject({ name: "Riel S.", emailAddress: "new@example.com" });
  });

  it("let people through DND, never yourself", async () => {
    const { server } = harness();
    const path = `/api/v1/settings/dnd_allowances/${USER_IDS.maya}`;
    const ids = (settings: Settings) => settings.notifications.allowedPeople.map((p) => p.userId);

    const removed = await expectStatus<Settings>(server, "DELETE", path, null, 200);

    expect(ids(removed)).not.toContain(USER_IDS.maya);

    const added = await expectStatus<Settings>(server, "POST", path, null, 200);

    expect(ids(added)).toContain(USER_IDS.maya);
    expect(ids(added)).toHaveLength(ids(removed).length + 1);
    await expectStatus(
      server,
      "POST",
      `/api/v1/settings/dnd_allowances/${USER_IDS.riel}`,
      null,
      422,
    );
  });

  it("set a custom status and out of office, and refuse an end in the past", async () => {
    const { server } = harness();
    const path = "/api/v1/settings/status";

    const saved = await expectStatus<Settings>(
      server,
      "PATCH",
      path,
      body(STATUS, {
        customStatusEmoji: "🌴",
        customStatusText: "On a beach",
        customStatusExpiresIn: "hour_1",
        oooPreset: "tomorrow",
        oooNote: "Back Friday",
      }),
      200,
    );

    expect(saved.status.customStatusExpiresAt).toBe(new Date(NOW + 3_600_000).toISOString());
    expect(saved.status).toMatchObject({ oooManual: true, oooNote: "Back Friday" });

    const past = await expectStatus<Json>(
      server,
      "PATCH",
      path,
      body(STATUS, { oooPreset: "custom", oooUntilCustom: "2020-01-01T09:00" }),
      422,
    );

    expect(errorOf(past).message).toContain("needs a future date and time");

    const cleared = await expectStatus<Settings>(
      server,
      "PATCH",
      path,
      body(STATUS, { clearCustomStatus: true, clearOoo: true }),
      200,
    );

    expect(cleared.status).toMatchObject({ customStatusText: null, oooUntil: null });
  });

  it("sign other sessions out, and this one out of the app", async () => {
    const { server } = harness();

    const others = await expectStatus<SessionList>(
      server,
      "POST",
      "/api/v1/settings/sessions/revoke_others",
      null,
      200,
    );

    expect(others.notice).toBe("Signed out 2 other sessions.");
    expect(others.sessions.map((session) => session.current)).toEqual([true]);

    const own = await expectStatus<Json>(
      server,
      "DELETE",
      "/api/v1/settings/sessions/3",
      null,
      401,
    );

    expect(errorOf(own).tag).toBe("Unauthorized");
  });

  it("connect and disconnect GitHub, Fizzy and Google with the classic words", async () => {
    const { server } = harness();
    const fizzy = "/api/v1/settings/fizzy_connection";

    for (const [token, message] of [
      ["  ", "Paste a token to connect Fizzy."],
      ["bad-1", "Fizzy rejected that token. Check it and try again."],
      ["none-1", "That token has no Fizzy account to use."],
    ] as const) {
      const refused = await expectStatus<Json>(server, "PUT", fizzy, { accessToken: token }, 422);

      expect(errorOf(refused)).toMatchObject({ tag: "Validation", message });
    }

    const linked = await expectStatus<IntegrationChange>(
      server,
      "PUT",
      fizzy,
      { accessToken: "fizzy-token" },
      200,
    );

    Schema.decodeUnknownSync(IntegrationChangeSchema)(linked);
    expect(linked.notice).toBe("Fizzy connected as Riel (Smart Data).");
    expect(linked.integrations.fizzy.state).toBe("connected");

    const unlinked = await expectStatus<IntegrationChange>(server, "DELETE", fizzy, null, 200);

    expect(unlinked).toMatchObject({
      notice: "Fizzy disconnected.",
      integrations: { fizzy: { state: "missing" } },
    });

    const github = await expectStatus<IntegrationChange>(
      server,
      "DELETE",
      "/api/v1/settings/github_connection",
      null,
      200,
    );

    expect(github.notice).toBe("GitHub disconnected.");
    expect((await get<Settings>(server, "/api/v1/settings")).profile.githubVerified).toBe(false);

    const google = await expectStatus<IntegrationChange>(
      server,
      "DELETE",
      "/api/v1/settings/google_connection",
      null,
      200,
    );

    expect(google.notice).toBe("Google Calendar disconnected.");
    expect(google.integrations.google).toMatchObject({ connected: false, email: null });
  });

  it("start over on reset", async () => {
    const { server } = harness();

    await expectStatus(server, "DELETE", "/api/v1/settings/push_subscriptions/4", null, 200);
    expect(
      (await get<PushSubscriptionList>(server, "/api/v1/settings/push_subscriptions"))
        .pushSubscriptions,
    ).toHaveLength(1);

    server.reset();

    expect(
      (await get<PushSubscriptionList>(server, "/api/v1/settings/push_subscriptions"))
        .pushSubscriptions,
    ).toHaveLength(2);
  });
});
