import { Schema } from "effect";
import { describe, expect, it } from "vitest";
import {
  PushSubscriptionList as PushSubscriptionListSchema,
  SessionList as SessionListSchema,
  Settings as SettingsSchema,
} from "../../src/api/schema/settings.ts";
import type { PushSubscriptionList } from "../../src/gen/PushSubscriptionList.ts";
import type { SessionList } from "../../src/gen/SessionList.ts";
import type { Settings } from "../../src/gen/Settings.ts";
import type { Json } from "../json.ts";
import { USER_IDS } from "../seed.ts";
import { MOCK_PASSWORD } from "./settings.ts";
import { errorOf, expectStatus, get, harness, NOW } from "./testing.ts";

/** Every key of a write body, `null` unless given. */
function body(keys: readonly string[], given: Readonly<Record<string, Json>>): Json {
  return Object.fromEntries(keys.map((key) => [key, given[key] ?? null]));
}

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

  it("let starred people through DND, never yourself", async () => {
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
