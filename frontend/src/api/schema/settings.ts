import { Schema } from "effect";
import type { AppearanceSettings as GeneratedAppearanceSettings } from "../../gen/AppearanceSettings.ts";
import type { CallSettings as GeneratedCallSettings } from "../../gen/CallSettings.ts";
import type { Connection as GeneratedConnection } from "../../gen/Connection.ts";
import type { DndAllowedPerson as GeneratedDndAllowedPerson } from "../../gen/DndAllowedPerson.ts";
import type { GoogleIntegration as GeneratedGoogleIntegration } from "../../gen/GoogleIntegration.ts";
import type { InboxSwitch as GeneratedInboxSwitch } from "../../gen/InboxSwitch.ts";
import type { IntegrationSettings as GeneratedIntegrationSettings } from "../../gen/IntegrationSettings.ts";
import type { NotificationSettings as GeneratedNotificationSettings } from "../../gen/NotificationSettings.ts";
import type { OooPreset as GeneratedOooPreset } from "../../gen/OooPreset.ts";
import type { ProfileSettings as GeneratedProfileSettings } from "../../gen/ProfileSettings.ts";
import type { PushSubscriptionInfo as GeneratedPushSubscriptionInfo } from "../../gen/PushSubscriptionInfo.ts";
import type { PushSubscriptionList as GeneratedPushSubscriptionList } from "../../gen/PushSubscriptionList.ts";
import type { SessionInfo as GeneratedSessionInfo } from "../../gen/SessionInfo.ts";
import type { SessionList as GeneratedSessionList } from "../../gen/SessionList.ts";
import type { Settings as GeneratedSettings } from "../../gen/Settings.ts";
import type { StatusExpiry as GeneratedStatusExpiry } from "../../gen/StatusExpiry.ts";
import type { StatusSettings as GeneratedStatusSettings } from "../../gen/StatusSettings.ts";
import type { TimeZoneChoice as GeneratedTimeZoneChoice } from "../../gen/TimeZoneChoice.ts";
import type { UpdateAppearance as GeneratedUpdateAppearance } from "../../gen/UpdateAppearance.ts";
import type { UpdateAvatar as GeneratedUpdateAvatar } from "../../gen/UpdateAvatar.ts";
import type { UpdateCalls as GeneratedUpdateCalls } from "../../gen/UpdateCalls.ts";
import type { UpdateNotifications as GeneratedUpdateNotifications } from "../../gen/UpdateNotifications.ts";
import type { UpdateProfile as GeneratedUpdateProfile } from "../../gen/UpdateProfile.ts";
import type { UpdateStatus as GeneratedUpdateStatus } from "../../gen/UpdateStatus.ts";
import { UserId } from "./ids.ts";
import { PresenceSetting, TextSize, Theme, VoiceMode } from "./me.ts";
import type { Assert, Pinned } from "./pin.ts";
import { Timestamp } from "./time.ts";

/** One time zone the appearance form offers. */
export const TimeZoneChoice = Schema.Struct({ label: Schema.String, value: Schema.String });

export type TimeZoneChoicePin = Assert<Pinned<typeof TimeZoneChoice, GeneratedTimeZoneChoice>>;

/** The profile section of the settings page. */
export const ProfileSettings = Schema.Struct({
  userId: UserId,
  name: Schema.String,
  emailAddress: Schema.NullOr(Schema.String),
  bio: Schema.NullOr(Schema.String),
  avatarUrl: Schema.String,
  avatarAttached: Schema.Boolean,
  hasPassword: Schema.Boolean,
  githubLogin: Schema.NullOr(Schema.String),
  githubVerified: Schema.Boolean,
  bot: Schema.Boolean,
});

export type ProfileSettings = typeof ProfileSettings.Type;

export type ProfileSettingsPin = Assert<Pinned<typeof ProfileSettings, GeneratedProfileSettings>>;

/** Theme, text size and time zone. */
export const AppearanceSettings = Schema.Struct({
  theme: Theme,
  textSize: TextSize,
  timeZone: Schema.NullOr(Schema.String),
  timeZones: Schema.Array(TimeZoneChoice),
});

export type AppearanceSettings = typeof AppearanceSettings.Type;

export type AppearanceSettingsPin = Assert<
  Pinned<typeof AppearanceSettings, GeneratedAppearanceSettings>
>;

/** Someone who may reach the user through do not disturb. */
export const DndAllowedPerson = Schema.Struct({ userId: UserId, name: Schema.String });

export type DndAllowedPersonPin = Assert<
  Pinned<typeof DndAllowedPerson, GeneratedDndAllowedPerson>
>;

/** One inbox notification switch. */
export const InboxSwitch = Schema.Struct({
  key: Schema.String,
  label: Schema.String,
  description: Schema.String,
  enabled: Schema.Boolean,
});

export type InboxSwitchPin = Assert<Pinned<typeof InboxSwitch, GeneratedInboxSwitch>>;

/** Do not disturb, quiet hours, keyword alerts and inbox switches. */
export const NotificationSettings = Schema.Struct({
  dndEnabled: Schema.Boolean,
  quietHoursEnabled: Schema.Boolean,
  quietHoursStart: Schema.NullOr(Schema.String),
  quietHoursEnd: Schema.NullOr(Schema.String),
  meetingDndEnabled: Schema.Boolean,
  oooNotifyEnabled: Schema.Boolean,
  allowedPeople: Schema.Array(DndAllowedPerson),
  keywordAlerts: Schema.Array(Schema.String),
  inbox: Schema.Array(InboxSwitch),
});

export type NotificationSettings = typeof NotificationSettings.Type;

export type NotificationSettingsPin = Assert<
  Pinned<typeof NotificationSettings, GeneratedNotificationSettings>
>;

/** Presence, custom status and out of office. */
export const StatusSettings = Schema.Struct({
  presenceSetting: PresenceSetting,
  customStatusEmoji: Schema.NullOr(Schema.String),
  customStatusText: Schema.NullOr(Schema.String),
  customStatusExpiresAt: Schema.NullOr(Timestamp),
  meetingStatusEnabled: Schema.Boolean,
  oooCalendarEnabled: Schema.Boolean,
  oooUntil: Schema.NullOr(Timestamp),
  oooManual: Schema.Boolean,
  oooNote: Schema.NullOr(Schema.String),
  calendarError: Schema.NullOr(Schema.String),
});

export type StatusSettings = typeof StatusSettings.Type;

export type StatusSettingsPin = Assert<Pinned<typeof StatusSettings, GeneratedStatusSettings>>;

/** How huddle audio is sent. */
export const CallSettings = Schema.Struct({
  voiceMode: VoiceMode,
  pushToTalkKey: Schema.NullOr(Schema.String),
});

export type CallSettings = typeof CallSettings.Type;

export type CallSettingsPin = Assert<Pinned<typeof CallSettings, GeneratedCallSettings>>;

/** The Google connection: sign-in identity, calendar and drive. */
export const GoogleIntegration = Schema.Struct({
  signInConfigured: Schema.Boolean,
  identityEmail: Schema.NullOr(Schema.String),
  calendarConfigured: Schema.Boolean,
  connected: Schema.Boolean,
  calendar: Schema.Boolean,
  drive: Schema.Boolean,
  email: Schema.NullOr(Schema.String),
});

export type GoogleIntegrationPin = Assert<
  Pinned<typeof GoogleIntegration, GeneratedGoogleIntegration>
>;

/** A token-based connection (GitHub, Fizzy), tagged by `state`. */
export const Connection = Schema.Union([
  Schema.Struct({ state: Schema.Literal("missing") }),
  Schema.Struct({ state: Schema.Literal("rejected"), reason: Schema.NullOr(Schema.String) }),
  Schema.Struct({
    state: Schema.Literal("connected"),
    name: Schema.String,
    workspace: Schema.NullOr(Schema.String),
    appToken: Schema.Boolean,
  }),
]);

export type Connection = typeof Connection.Type;

export type ConnectionPin = Assert<Pinned<typeof Connection, GeneratedConnection>>;

/** Connected accounts. */
export const IntegrationSettings = Schema.Struct({
  google: GoogleIntegration,
  github: Connection,
  githubAppConfigured: Schema.Boolean,
  fizzy: Connection,
  managePath: Schema.String,
  slackImportPath: Schema.String,
});

export type IntegrationSettings = typeof IntegrationSettings.Type;

export type IntegrationSettingsPin = Assert<
  Pinned<typeof IntegrationSettings, GeneratedIntegrationSettings>
>;

/** `GET /api/v1/settings`: everything the settings screens show. */
export const Settings = Schema.Struct({
  profile: ProfileSettings,
  appearance: AppearanceSettings,
  notifications: NotificationSettings,
  status: StatusSettings,
  calls: CallSettings,
  integrations: IntegrationSettings,
});

export type Settings = typeof Settings.Type;

export type SettingsPin = Assert<Pinned<typeof Settings, GeneratedSettings>>;

// Write bodies: every key is present and `null` leaves the setting unchanged.

/** `PATCH /api/v1/settings/profile`. */
export const UpdateProfile = Schema.Struct({
  name: Schema.NullOr(Schema.String),
  emailAddress: Schema.NullOr(Schema.String),
  currentPassword: Schema.NullOr(Schema.String),
  password: Schema.NullOr(Schema.String),
  bio: Schema.NullOr(Schema.String),
  githubLogin: Schema.NullOr(Schema.String),
});

export type UpdateProfile = typeof UpdateProfile.Type;

export type UpdateProfilePin = Assert<Pinned<typeof UpdateProfile, GeneratedUpdateProfile>>;

/** `PUT /api/v1/settings/avatar`: a direct-uploaded blob's signed id. */
export const UpdateAvatar = Schema.Struct({ signedId: Schema.String });

export type UpdateAvatarPin = Assert<Pinned<typeof UpdateAvatar, GeneratedUpdateAvatar>>;

/** `PATCH /api/v1/settings/appearance`. An empty time zone clears it. */
export const UpdateAppearance = Schema.Struct({
  theme: Schema.NullOr(Theme),
  textSize: Schema.NullOr(TextSize),
  timeZone: Schema.NullOr(Schema.String),
});

export type UpdateAppearance = typeof UpdateAppearance.Type;

export type UpdateAppearancePin = Assert<
  Pinned<typeof UpdateAppearance, GeneratedUpdateAppearance>
>;

/** `PATCH /api/v1/settings/notifications`. `inbox` changes only the keys it names. */
export const UpdateNotifications = Schema.Struct({
  dndEnabled: Schema.NullOr(Schema.Boolean),
  quietHoursEnabled: Schema.NullOr(Schema.Boolean),
  quietHoursStart: Schema.NullOr(Schema.String),
  quietHoursEnd: Schema.NullOr(Schema.String),
  meetingDndEnabled: Schema.NullOr(Schema.Boolean),
  oooNotifyEnabled: Schema.NullOr(Schema.Boolean),
  keywordAlerts: Schema.NullOr(Schema.Array(Schema.String)),
  inbox: Schema.NullOr(Schema.Record(Schema.String, Schema.Boolean)),
});

export type UpdateNotifications = typeof UpdateNotifications.Type;

export type UpdateNotificationsPin = Assert<
  Pinned<typeof UpdateNotifications, GeneratedUpdateNotifications>
>;

/** When a custom status clears itself. */
export const StatusExpiry = Schema.Literals([
  "minutes_30",
  "hour_1",
  "hours_4",
  "today",
  "week",
  "never",
]);

export type StatusExpiry = typeof StatusExpiry.Type;

export type StatusExpiryPin = Assert<Pinned<typeof StatusExpiry, GeneratedStatusExpiry>>;

/** How long a manual out of office lasts. */
export const OooPreset = Schema.Literals(["tomorrow", "monday", "week", "custom"]);

export type OooPreset = typeof OooPreset.Type;

export type OooPresetPin = Assert<Pinned<typeof OooPreset, GeneratedOooPreset>>;

/** `PATCH /api/v1/settings/status`. */
export const UpdateStatus = Schema.Struct({
  presenceSetting: Schema.NullOr(PresenceSetting),
  customStatusEmoji: Schema.NullOr(Schema.String),
  customStatusText: Schema.NullOr(Schema.String),
  customStatusExpiresIn: Schema.NullOr(StatusExpiry),
  clearCustomStatus: Schema.NullOr(Schema.Boolean),
  meetingStatusEnabled: Schema.NullOr(Schema.Boolean),
  oooCalendarEnabled: Schema.NullOr(Schema.Boolean),
  oooPreset: Schema.NullOr(OooPreset),
  oooUntilCustom: Schema.NullOr(Schema.String),
  oooNote: Schema.NullOr(Schema.String),
  clearOoo: Schema.NullOr(Schema.Boolean),
});

export type UpdateStatus = typeof UpdateStatus.Type;

export type UpdateStatusPin = Assert<Pinned<typeof UpdateStatus, GeneratedUpdateStatus>>;

/** `PATCH /api/v1/settings/calls`. */
export const UpdateCalls = Schema.Struct({
  voiceMode: Schema.NullOr(VoiceMode),
  pushToTalkKey: Schema.NullOr(Schema.String),
});

export type UpdateCalls = typeof UpdateCalls.Type;

export type UpdateCallsPin = Assert<Pinned<typeof UpdateCalls, GeneratedUpdateCalls>>;

/** One signed-in session. */
export const SessionInfo = Schema.Struct({
  id: Schema.Int,
  current: Schema.Boolean,
  description: Schema.String,
  ipAddress: Schema.NullOr(Schema.String),
  lastActiveAt: Timestamp,
  createdAt: Timestamp,
});

export type SessionInfo = typeof SessionInfo.Type;

export type SessionInfoPin = Assert<Pinned<typeof SessionInfo, GeneratedSessionInfo>>;

/** `GET /api/v1/settings/sessions`, and the reply to revoking. */
export const SessionList = Schema.Struct({
  sessions: Schema.Array(SessionInfo),
  notice: Schema.NullOr(Schema.String),
});

export type SessionList = typeof SessionList.Type;

export type SessionListPin = Assert<Pinned<typeof SessionList, GeneratedSessionList>>;

/** One browser or device receiving push notifications. */
export const PushSubscriptionInfo = Schema.Struct({
  id: Schema.Int,
  endpoint: Schema.String,
  browser: Schema.String,
  version: Schema.String,
  platform: Schema.String,
});

export type PushSubscriptionInfo = typeof PushSubscriptionInfo.Type;

export type PushSubscriptionInfoPin = Assert<
  Pinned<typeof PushSubscriptionInfo, GeneratedPushSubscriptionInfo>
>;

/** `GET /api/v1/settings/push_subscriptions`. */
export const PushSubscriptionList = Schema.Struct({
  pushSubscriptions: Schema.Array(PushSubscriptionInfo),
});

export type PushSubscriptionList = typeof PushSubscriptionList.Type;

export type PushSubscriptionListPin = Assert<
  Pinned<typeof PushSubscriptionList, GeneratedPushSubscriptionList>
>;
