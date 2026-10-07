/**
 * The S7 settings endpoints: the profile page's sections, sessions, push subscriptions and the
 * personal GitHub, Fizzy and Google Calendar connections, the rooms list, two-step sign-in and
 * the sign-in link.
 */
import { Effect } from "effect";
import type { AccountSettings } from "../gen/AccountSettings.ts";
import type { BackupCodes } from "../gen/BackupCodes.ts";
import type { IntegrationChange } from "../gen/IntegrationChange.ts";
import type { IntegrationToken } from "../gen/IntegrationToken.ts";
import type { PushSubscriptionList } from "../gen/PushSubscriptionList.ts";
import type { Reauthentication } from "../gen/Reauthentication.ts";
import type { SessionList } from "../gen/SessionList.ts";
import type { Settings } from "../gen/Settings.ts";
import type { TwoFactorChange } from "../gen/TwoFactorChange.ts";
import type { UpdateAppearance } from "../gen/UpdateAppearance.ts";
import type { UpdateCalls } from "../gen/UpdateCalls.ts";
import type { UpdateNotifications } from "../gen/UpdateNotifications.ts";
import type { UpdateProfile } from "../gen/UpdateProfile.ts";
import type { UpdateStatus } from "../gen/UpdateStatus.ts";
import { call, get, noContent } from "./call.ts";
import type { ApiRequest } from "./client.ts";
import {
  AccountSettings as AccountSettingsSchema,
  BackupCodes as BackupCodesSchema,
  IntegrationChange as IntegrationChangeSchema,
  PushSubscriptionList as PushSubscriptionListSchema,
  SessionList as SessionListSchema,
  Settings as SettingsSchema,
  TwoFactorChange as TwoFactorChangeSchema,
} from "./schema/settings.ts";
import { wire } from "./wire.ts";

const settingsReply = wire<Settings>(SettingsSchema);

/** `GET /settings`: every section the classic profile page shows. */
export const settings = Effect.fn("api.settings")(function* () {
  return yield* call(get("/settings"), settingsReply);
});

/** `PATCH /settings/profile`: name, email, password, bio and GitHub username. */
export const updateProfile = Effect.fn("api.updateProfile")(function* (body: UpdateProfile) {
  return yield* call({ method: "PATCH", path: "/settings/profile", body }, settingsReply);
});

/** `PUT /settings/avatar`: a finished direct upload's signed id becomes the avatar. */
export const updateAvatar = Effect.fn("api.updateAvatar")(function* (signedId: string) {
  return yield* call(
    { method: "PUT", path: "/settings/avatar", body: { signedId } },
    settingsReply,
  );
});

/** `DELETE /settings/avatar`: back to the generated initials. */
export const removeAvatar = Effect.fn("api.removeAvatar")(function* () {
  return yield* call({ method: "DELETE", path: "/settings/avatar" }, settingsReply);
});

/** `PATCH /settings/appearance`: the account's theme, text size and time zone. */
export const updateAppearance = Effect.fn("api.updateAppearance")(function* (
  body: UpdateAppearance,
) {
  return yield* call({ method: "PATCH", path: "/settings/appearance", body }, settingsReply);
});

/** `PATCH /settings/calls`: microphone mode and push-to-talk key. */
export const updateCalls = Effect.fn("api.updateCalls")(function* (body: UpdateCalls) {
  return yield* call({ method: "PATCH", path: "/settings/calls", body }, settingsReply);
});

/** `PATCH /settings/notifications`: DND, quiet hours, keyword alerts and inbox switches. */
export const updateNotifications = Effect.fn("api.updateNotifications")(function* (
  body: UpdateNotifications,
) {
  return yield* call({ method: "PATCH", path: "/settings/notifications", body }, settingsReply);
});

/** `PATCH /settings/status`: presence, custom status, meetings and out of office. */
export const updateStatus = Effect.fn("api.updateStatus")(function* (body: UpdateStatus) {
  return yield* call({ method: "PATCH", path: "/settings/status", body }, settingsReply);
});

/** `POST` (let through) or `DELETE` (stop) `/settings/dnd_allowances/:user_id`. */
export const setDndAllowance = Effect.fn("api.setDndAllowance")(function* (
  userId: number,
  allowed: boolean,
) {
  return yield* call(
    { method: allowed ? "POST" : "DELETE", path: `/settings/dnd_allowances/${userId}` },
    settingsReply,
  );
});

const sessionsReply = wire<SessionList>(SessionListSchema);

/** `GET /settings/sessions`: every browser signed in to the account. */
export const sessions = Effect.fn("api.sessions")(function* () {
  return yield* call(get("/settings/sessions"), sessionsReply);
});

/**
 * `DELETE /settings/sessions/:id`. Another session answers the list; this browser's own answers
 * `Unauthorized` once it is signed out, with its push subscription (`endpoint`) dropped too.
 */
export const revokeSession = Effect.fn("api.revokeSession")(function* (
  sessionId: number,
  endpoint: string | null,
) {
  const path = `/settings/sessions/${sessionId}`;

  const request: ApiRequest =
    endpoint === null
      ? { method: "DELETE", path }
      : { method: "DELETE", path, query: { push_subscription_endpoint: endpoint } };

  return yield* call(request, sessionsReply);
});

/** `POST /settings/sessions/revoke_others`: signs out everywhere but here. */
export const revokeOtherSessions = Effect.fn("api.revokeOtherSessions")(function* () {
  return yield* call({ method: "POST", path: "/settings/sessions/revoke_others" }, sessionsReply);
});

const pushReply = wire<PushSubscriptionList>(PushSubscriptionListSchema);

/** `GET /settings/push_subscriptions`: the devices that get push notifications. */
export const pushSubscriptions = Effect.fn("api.pushSubscriptions")(function* () {
  return yield* call(get("/settings/push_subscriptions"), pushReply);
});

/** `DELETE /settings/push_subscriptions/:id`: that device stops getting them. */
export const removePushSubscription = Effect.fn("api.removePushSubscription")(function* (
  subscriptionId: number,
) {
  return yield* call(
    { method: "DELETE", path: `/settings/push_subscriptions/${subscriptionId}` },
    pushReply,
  );
});

/** `POST /settings/push_subscriptions/:id/test`: sends that device a test notification. */
export const testPush = Effect.fn("api.testPush")(function* (subscriptionId: number) {
  return yield* call(
    { method: "POST", path: `/settings/push_subscriptions/${subscriptionId}/test` },
    noContent,
  );
});

const integrationReply = wire<IntegrationChange>(IntegrationChangeSchema);

/** A personal connection made with a pasted token. */
export type TokenService = "github" | "fizzy";

/**
 * `PUT /settings/{github,fizzy}_connection`: the service checks the token before it is stored. A
 * refusal (blank, rejected, unreachable, no Fizzy account) fails `Validation` with the classic
 * alert; a lapsed password confirmation fails `SudoRequired`.
 */
export const connectService = Effect.fn("api.connectService")(function* (
  service: TokenService,
  accessToken: string,
) {
  const body: IntegrationToken = { accessToken };

  return yield* call(
    { method: "PUT", path: `/settings/${service}_connection`, body },
    integrationReply,
  );
});

/** `DELETE /settings/{github,fizzy,google}_connection`: the classic profile's disconnect. */
export const disconnectService = Effect.fn("api.disconnectService")(function* (
  service: TokenService | "google",
) {
  return yield* call(
    { method: "DELETE", path: `/settings/${service}_connection` },
    integrationReply,
  );
});

const accountReply = wire<AccountSettings>(AccountSettingsSchema);

/** `GET /settings/account`: the rooms you're in, two-step sign-in and the sign-in link. */
export const accountSettings = Effect.fn("api.accountSettings")(function* () {
  return yield* call(get("/settings/account"), accountReply);
});

const codesReply = wire<BackupCodes>(BackupCodesSchema);

const twoFactorReply = wire<TwoFactorChange>(TwoFactorChangeSchema);

/**
 * `POST /settings/two_factor/backup_codes`: replaces the backup codes. Each two-step write is
 * confirmed with `reauth` (a code or password); a refusal fails `Validation` or `RateLimited`
 * with the classic alert, and two-step sign-in being off fails `Conflict`.
 */
export const newBackupCodes = Effect.fn("api.newBackupCodes")(function* (reauth: string) {
  const body: Reauthentication = { reauth };

  return yield* call(
    { method: "POST", path: "/settings/two_factor/backup_codes", body },
    codesReply,
  );
});

/** `DELETE /settings/two_factor`: turns two-step sign-in off (the session is replaced). */
export const disableTwoFactor = Effect.fn("api.disableTwoFactor")(function* (reauth: string) {
  const body: Reauthentication = { reauth };

  return yield* call({ method: "DELETE", path: "/settings/two_factor", body }, twoFactorReply);
});

/**
 * `DELETE /settings/two_factor/devices/:id` forgets one remembered browser; without an id,
 * `DELETE /settings/two_factor/devices` forgets them all.
 */
export const forgetDevices = Effect.fn("api.forgetDevices")(function* (
  deviceId: number | null,
  reauth: string,
) {
  const body: Reauthentication = { reauth };

  const path =
    deviceId === null ? "/settings/two_factor/devices" : `/settings/two_factor/devices/${deviceId}`;

  return yield* call({ method: "DELETE", path, body }, twoFactorReply);
});
