/**
 * What the settings screens call: plain promises over the S7 endpoints. A write answers the whole
 * settings page, and the signed-in person (`/me`: name, avatar, DND, out of office, appearance)
 * is reloaded after it so the rest of the app shows the change at once. Failures reject with an
 * `ActionError`; a rejected change carries its field messages in `fields`.
 */
import { me } from "../api/endpoints.ts";
import {
  accountSettings,
  connectService,
  createPushSubscription,
  disableTwoFactor,
  disconnectService,
  forgetDevices,
  settings as loadSettings,
  newBackupCodes,
  pushPublicKey,
  pushSubscriptions,
  removeAvatar,
  removePushSubscription,
  revokeOtherSessions,
  revokeSession,
  sessions,
  setDndAllowance,
  type TokenService,
  testPush,
  updateAppearance,
  updateAvatar,
  updateCalls,
  updateNotifications,
  updateProfile,
  updateStatus,
} from "../api/settings-endpoints.ts";
import type { AccountSettings } from "../gen/AccountSettings.ts";
import type { BackupCodes } from "../gen/BackupCodes.ts";
import type { CreatePushSubscription } from "../gen/CreatePushSubscription.ts";
import type { IntegrationChange } from "../gen/IntegrationChange.ts";
import type { PushPublicKey } from "../gen/PushPublicKey.ts";
import type { PushSubscriptionList } from "../gen/PushSubscriptionList.ts";
import type { SessionList } from "../gen/SessionList.ts";
import type { Settings } from "../gen/Settings.ts";
import type { TwoFactorChange } from "../gen/TwoFactorChange.ts";
import type { UpdateAppearance } from "../gen/UpdateAppearance.ts";
import type { UpdateCalls } from "../gen/UpdateCalls.ts";
import type { UpdateNotifications } from "../gen/UpdateNotifications.ts";
import type { UpdateProfile } from "../gen/UpdateProfile.ts";
import type { UpdateStatus } from "../gen/UpdateStatus.ts";
import { mutations } from "../store/store.ts";
import { runAction } from "./runtime.ts";

export {
  enablePushNotifications,
  inspectPush,
  type PushEnrollmentOutcome,
  type PushPermission,
  unsubscribePushEndpoint,
} from "./push-enrollment.ts";

/** Every key of a write body, unchanged (`null`), so a caller names only what it changes. */
const UNCHANGED = {
  profile: {
    name: null,
    emailAddress: null,
    currentPassword: null,
    password: null,
    bio: null,
    githubLogin: null,
  },
  appearance: { theme: null, textSize: null, timeZone: null },
  calls: { voiceMode: null, pushToTalkKey: null },
  notifications: {
    dndEnabled: null,
    quietHoursEnabled: null,
    quietHoursStart: null,
    quietHoursEnd: null,
    meetingDndEnabled: null,
    oooNotifyEnabled: null,
    keywordAlerts: null,
    inbox: null,
  },
  status: {
    presenceSetting: null,
    customStatusEmoji: null,
    customStatusText: null,
    customStatusExpiresIn: null,
    clearCustomStatus: null,
    meetingStatusEnabled: null,
    oooCalendarEnabled: null,
    oooPreset: null,
    oooUntilCustom: null,
    oooNote: null,
    clearOoo: null,
  },
} as const satisfies {
  readonly profile: UpdateProfile;
  readonly appearance: UpdateAppearance;
  readonly calls: UpdateCalls;
  readonly notifications: UpdateNotifications;
  readonly status: UpdateStatus;
};

/** Reloads the signed-in person; a failure leaves the store as it was (the next boot fixes it). */
function refreshMe(): void {
  runAction(me()).then(mutations.setMe, () => undefined);
}

/** Runs a write, then refreshes `/me`. */
async function write<A>(run: Promise<A>): Promise<A> {
  const next = await run;

  refreshMe();

  return next;
}

export type { TokenService };

export const settings = {
  load: (): Promise<Settings> => runAction(loadSettings()),

  updateProfile: (change: Partial<UpdateProfile>): Promise<Settings> =>
    write(runAction(updateProfile({ ...UNCHANGED.profile, ...change }))),

  setAvatar: (signedId: string): Promise<Settings> => write(runAction(updateAvatar(signedId))),

  removeAvatar: (): Promise<Settings> => write(runAction(removeAvatar())),

  updateAppearance: (change: Partial<UpdateAppearance>): Promise<Settings> =>
    write(runAction(updateAppearance({ ...UNCHANGED.appearance, ...change }))),

  updateCalls: (change: Partial<UpdateCalls>): Promise<Settings> =>
    write(runAction(updateCalls({ ...UNCHANGED.calls, ...change }))),

  updateNotifications: (change: Partial<UpdateNotifications>): Promise<Settings> =>
    write(runAction(updateNotifications({ ...UNCHANGED.notifications, ...change }))),

  updateStatus: (change: Partial<UpdateStatus>): Promise<Settings> =>
    write(runAction(updateStatus({ ...UNCHANGED.status, ...change }))),

  /** Lets someone's messages through DND (`true`), or stops (`false`). */
  setDndAllowance: (userId: number, allowed: boolean): Promise<Settings> =>
    runAction(setDndAllowance(userId, allowed)),

  sessions: (): Promise<SessionList> => runAction(sessions()),

  /**
   * Signs a session out. For this browser's own session pass its push subscription's endpoint;
   * the server signs it out and the app goes to the sign-in page.
   */
  revokeSession: (sessionId: number, endpoint: string | null = null): Promise<SessionList> =>
    runAction(revokeSession(sessionId, endpoint)),

  revokeOtherSessions: (): Promise<SessionList> => runAction(revokeOtherSessions()),

  pushSubscriptions: (): Promise<PushSubscriptionList> => runAction(pushSubscriptions()),

  pushPublicKey: (): Promise<PushPublicKey> => runAction(pushPublicKey()),

  createPushSubscription: (body: CreatePushSubscription): Promise<PushSubscriptionList> =>
    runAction(createPushSubscription(body)),

  removePushSubscription: (subscriptionId: number): Promise<PushSubscriptionList> =>
    runAction(removePushSubscription(subscriptionId)),

  /** Sends that device a test notification. */
  testPush: (subscriptionId: number): Promise<void> => runAction(testPush(subscriptionId)),

  /** Connects GitHub or Fizzy with a personal access token. */
  connect: (service: TokenService, accessToken: string): Promise<IntegrationChange> =>
    write(runAction(connectService(service, accessToken))),

  /**
   * Disconnects GitHub, Fizzy or Google Calendar (dropping Google can end a calendar out of
   * office, so `/me` is reloaded after each change).
   */
  disconnect: (service: TokenService | "google"): Promise<IntegrationChange> =>
    write(runAction(disconnectService(service))),

  /** The rooms you're in, two-step sign-in and the sign-in link. */
  account: (): Promise<AccountSettings> => runAction(accountSettings()),

  /** New backup codes, confirmed with a code or password (empty after "Confirm with Google"). */
  newBackupCodes: (reauth: string): Promise<BackupCodes> => runAction(newBackupCodes(reauth)),

  /** Turns two-step sign-in off. */
  disableTwoFactor: (reauth: string): Promise<TwoFactorChange> =>
    runAction(disableTwoFactor(reauth)),

  /** Forgets one remembered browser, or every one with `null`. */
  forgetDevices: (deviceId: number | null, reauth: string): Promise<TwoFactorChange> =>
    runAction(forgetDevices(deviceId, reauth)),
};
