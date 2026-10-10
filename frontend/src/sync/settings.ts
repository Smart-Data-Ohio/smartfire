/**
 * What the settings screens call: plain promises over the S7 endpoints. A write answers the whole
 * settings page, and the signed-in person (`/me`: name, avatar, DND, out of office, appearance)
 * is reloaded after it so the rest of the app shows the change at once. Failures reject with an
 * `ActionError`; a rejected change carries its field messages in `fields`.
 */

import { me, sidebar } from "../api/endpoints.ts";
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
import { completeTour as stampTour } from "../api/tour-endpoints.ts";
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
import {
  appearanceSnapshot,
  applyAccountAppearance,
  showAccountTheme,
  type ThemePreference,
} from "../lib/appearance.ts";
import { beginSnapshotRequest, newerSnapshotRequest } from "../store/request-order.ts";
import type { State } from "../store/state.ts";
import { mutations, sidebarRowClock, store } from "../store/store.ts";
import { loadUnreadCount } from "./activity-actions.ts";
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
    defaultNotificationLevel: null,
    roomNotification: null,
    roomMute: null,
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

let latestSettings: { readonly sequence: number; readonly value: Settings } | null = null;

async function settingsSnapshot(run: () => Promise<Settings>): Promise<Settings> {
  const sequence = beginSnapshotRequest();
  const next = await run();

  if (latestSettings === null || newerSnapshotRequest(sequence, latestSettings.sequence)) {
    latestSettings = { sequence, value: next };
    mutations.setNotificationPreferences(next.notifications);
  }

  // Settings screens also replace their local page with the returned snapshot.
  return latestSettings.value;
}

export const settings = {
  load: (): Promise<Settings> => settingsSnapshot(() => runAction(loadSettings())),

  updateProfile: (change: Partial<UpdateProfile>): Promise<Settings> =>
    write(settingsSnapshot(() => runAction(updateProfile({ ...UNCHANGED.profile, ...change })))),

  setAvatar: (signedId: string): Promise<Settings> =>
    write(settingsSnapshot(() => runAction(updateAvatar(signedId)))),

  removeAvatar: (): Promise<Settings> => write(settingsSnapshot(() => runAction(removeAvatar()))),

  updateAppearance: (change: Partial<UpdateAppearance>): Promise<Settings> =>
    write(
      settingsSnapshot(() => runAction(updateAppearance({ ...UNCHANGED.appearance, ...change }))),
    ),

  updateCalls: (change: Partial<UpdateCalls>): Promise<Settings> =>
    write(settingsSnapshot(() => runAction(updateCalls({ ...UNCHANGED.calls, ...change })))),

  updateNotifications: (change: Partial<UpdateNotifications>): Promise<Settings> =>
    write(
      settingsSnapshot(() =>
        runAction(updateNotifications({ ...UNCHANGED.notifications, ...change })),
      ),
    ),

  updateStatus: (change: Partial<UpdateStatus>): Promise<Settings> =>
    write(settingsSnapshot(() => runAction(updateStatus({ ...UNCHANGED.status, ...change })))),

  /** Lets someone's messages through DND (`true`), or stops (`false`). */
  setDndAllowance: (userId: number, allowed: boolean): Promise<Settings> =>
    settingsSnapshot(() => runAction(setDndAllowance(userId, allowed))),

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

/**
 * Records the product tour as done, skipped or finished: `/me` says so at once, so it won't start
 * by itself again, and classic's stamp goes out. Best effort, as classic's is: if the stamp fails,
 * the next page load offers the tour again.
 */
export function completeTour(): void {
  const me = store.getState().me;

  if (me !== null && !me.preferences.tourCompleted) {
    mutations.setMe({ ...me, preferences: { ...me.preferences, tourCompleted: true } });
  }

  runAction(stampTour()).catch(() => undefined);
}

/** The account's theme and size as the store knows them: `/me` once loaded, else boot. */
function accountOf(state: State) {
  return state.me?.preferences ?? state.boot ?? null;
}

/**
 * Keeps the account's theme and text size on screen as boot and `/me` report them (every settings
 * save reloads `/me`, so a save on another tab or device shows on the next one). Returns the
 * unsubscribe.
 */
export function followAccountAppearance(): () => void {
  let last = accountOf(store.getState());

  if (last !== null) {
    applyAccountAppearance({ theme: last.theme, textSize: last.textSize });
  }

  return store.subscribe((state) => {
    const account = accountOf(state);

    if (account === null || account === last) {
      return;
    }

    last = account;
    applyAccountAppearance({ theme: account.theme, textSize: account.textSize });
  });
}

/**
 * Saves `theme` as the account's theme, showing it at once; a refusal puts the previous one back
 * and rejects with the reason.
 */
export async function saveAccountTheme(theme: ThemePreference): Promise<void> {
  const before = appearanceSnapshot().accountTheme;

  showAccountTheme(theme);

  try {
    await settings.updateAppearance({ theme });
  } catch (error) {
    showAccountTheme(before);
    throw error;
  }
}

/** Refreshes preferences across tabs and schedules the next mute expiry. */
export function followNotificationPreferences(): () => void {
  let timer: ReturnType<typeof setTimeout> | undefined;
  let viewerId = store.getState().me?.user.id;
  let held = store.getState().sidebar.notificationPreferences;

  const refreshSidebar = () => {
    const clock = sidebarRowClock();
    const generation = store.getState().activity.generation;
    void runAction(loadUnreadCount(generation)).catch(() => undefined);
    void runAction(sidebar()).then(
      (next) => mutations.loadSidebar(next, clock),
      () => undefined,
    );
  };

  const refresh = () => {
    if (store.getState().me !== null) {
      void settings.load().catch(() => undefined);
    }
  };

  const schedule = () => {
    clearTimeout(timer);
    const now = Date.now();

    const ends = Object.values(held?.roomMuteUntil ?? {})
      .flatMap((until) => (until === null ? [] : [Date.parse(until)]))
      .filter((end) => end > now);

    if (ends.length === 0) return;
    timer = setTimeout(
      () => {
        mutations.tickNotificationClock();
        refreshSidebar();
        schedule();
      },
      Math.min(Math.min(...ends) - now, 2_147_483_647),
    );
  };

  const unsubscribe = store.subscribe((state) => {
    const currentId = state.me?.user.id;

    if (currentId !== viewerId) {
      viewerId = currentId;
      refresh();
    }

    if (state.sidebar.notificationPreferences !== held) {
      held = state.sidebar.notificationPreferences;
      schedule();
      refreshSidebar();
    }
  });

  const visible = () => {
    if (document.visibilityState === "visible") {
      mutations.tickNotificationClock();
      refresh();
    }
  };

  refresh();
  schedule();
  document.addEventListener("visibilitychange", visible);

  return () => {
    unsubscribe();
    clearTimeout(timer);
    document.removeEventListener("visibilitychange", visible);
  };
}
