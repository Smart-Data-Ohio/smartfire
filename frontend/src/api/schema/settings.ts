import { Schema } from "effect";
import type { AccountSettings as GeneratedAccountSettings } from "../../gen/AccountSettings.ts";
import type { AppearanceSettings as GeneratedAppearanceSettings } from "../../gen/AppearanceSettings.ts";
import type { BackupCodes as GeneratedBackupCodes } from "../../gen/BackupCodes.ts";
import type { CallSettings as GeneratedCallSettings } from "../../gen/CallSettings.ts";
import type { Connection as GeneratedConnection } from "../../gen/Connection.ts";
import type { CreatePushSubscription as GeneratedCreatePushSubscription } from "../../gen/CreatePushSubscription.ts";
import type { DndAllowedPerson as GeneratedDndAllowedPerson } from "../../gen/DndAllowedPerson.ts";
import type { GoogleIntegration as GeneratedGoogleIntegration } from "../../gen/GoogleIntegration.ts";
import type { InboxSwitch as GeneratedInboxSwitch } from "../../gen/InboxSwitch.ts";
import type { IntegrationChange as GeneratedIntegrationChange } from "../../gen/IntegrationChange.ts";
import type { IntegrationSettings as GeneratedIntegrationSettings } from "../../gen/IntegrationSettings.ts";
import type { IntegrationToken as GeneratedIntegrationToken } from "../../gen/IntegrationToken.ts";
import type { NotificationSettings as GeneratedNotificationSettings } from "../../gen/NotificationSettings.ts";
import type { OooPreset as GeneratedOooPreset } from "../../gen/OooPreset.ts";
import type { ProfileSettings as GeneratedProfileSettings } from "../../gen/ProfileSettings.ts";
import type { PushPublicKey as GeneratedPushPublicKey } from "../../gen/PushPublicKey.ts";
import type { PushSubscriptionInfo as GeneratedPushSubscriptionInfo } from "../../gen/PushSubscriptionInfo.ts";
import type { PushSubscriptionList as GeneratedPushSubscriptionList } from "../../gen/PushSubscriptionList.ts";
import type { Reauthentication as GeneratedReauthentication } from "../../gen/Reauthentication.ts";
import type { RememberedDevice as GeneratedRememberedDevice } from "../../gen/RememberedDevice.ts";
import type { RoomMembershipRow as GeneratedRoomMembershipRow } from "../../gen/RoomMembershipRow.ts";
import type { SessionInfo as GeneratedSessionInfo } from "../../gen/SessionInfo.ts";
import type { SessionList as GeneratedSessionList } from "../../gen/SessionList.ts";
import type { Settings as GeneratedSettings } from "../../gen/Settings.ts";
import type { StatusExpiry as GeneratedStatusExpiry } from "../../gen/StatusExpiry.ts";
import type { StatusSettings as GeneratedStatusSettings } from "../../gen/StatusSettings.ts";
import type { TimeZoneChoice as GeneratedTimeZoneChoice } from "../../gen/TimeZoneChoice.ts";
import type { TwoFactorChange as GeneratedTwoFactorChange } from "../../gen/TwoFactorChange.ts";
import type { TwoFactorSettings as GeneratedTwoFactorSettings } from "../../gen/TwoFactorSettings.ts";
import type { UpdateAppearance as GeneratedUpdateAppearance } from "../../gen/UpdateAppearance.ts";
import type { UpdateAvatar as GeneratedUpdateAvatar } from "../../gen/UpdateAvatar.ts";
import type { UpdateCalls as GeneratedUpdateCalls } from "../../gen/UpdateCalls.ts";
import type { UpdateNotifications as GeneratedUpdateNotifications } from "../../gen/UpdateNotifications.ts";
import type { UpdateProfile as GeneratedUpdateProfile } from "../../gen/UpdateProfile.ts";
import type { UpdateStatus as GeneratedUpdateStatus } from "../../gen/UpdateStatus.ts";
import { PushSubscriptionId, RememberedDeviceId, RoomId, SessionId, UserId } from "./ids.ts";
import { PresenceSetting, TextSize, Theme, VoiceMode } from "./me.ts";
import type { Assert, Pinned } from "./pin.ts";
import { Involvement } from "./room.ts";
import { Timestamp } from "./time.ts";

/** One time zone the appearance form offers. */
export const TimeZoneChoice = Schema.Struct({ label: Schema.String, value: Schema.String });

export type TimeZoneChoicePin = Assert<Pinned<typeof TimeZoneChoice, GeneratedTimeZoneChoice>>;

export const NotificationLevel = Schema.Literals(["everything", "mentions", "nothing"]);

export const RoomMuteDuration = Schema.Literals([
  "minutes15",
  "hour1",
  "hours8",
  "hours24",
  "forever",
  "off",
]);

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
  defaultNotificationLevel: NotificationLevel,
  roomNotificationLevels: Schema.Record(Schema.String, Schema.NullOr(NotificationLevel)),
  roomMuteUntil: Schema.Record(Schema.String, Schema.NullOr(Schema.String)),
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

/** `PUT /api/v1/settings/github_connection` and `/fizzy_connection`: a personal access token. */
export const IntegrationToken = Schema.Struct({ accessToken: Schema.String });

export type IntegrationTokenPin = Assert<
  Pinned<typeof IntegrationToken, GeneratedIntegrationToken>
>;

/** A connect or disconnect: the integrations as they now stand, and the classic notice. */
export const IntegrationChange = Schema.Struct({
  integrations: IntegrationSettings,
  notice: Schema.String,
});

export type IntegrationChange = typeof IntegrationChange.Type;

export type IntegrationChangePin = Assert<
  Pinned<typeof IntegrationChange, GeneratedIntegrationChange>
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
  defaultNotificationLevel: Schema.NullOr(NotificationLevel),
  roomNotification: Schema.NullOr(
    Schema.Struct({ roomId: RoomId, level: Schema.NullOr(NotificationLevel) }),
  ),
  roomMute: Schema.NullOr(Schema.Struct({ roomId: RoomId, duration: RoomMuteDuration })),
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
  id: SessionId,
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
  id: PushSubscriptionId,
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

/** The classic page's public VAPID key, or `null` when push is not configured. */
export const PushPublicKey = Schema.Struct({ publicKey: Schema.NullOr(Schema.String) });

export type PushPublicKeyPin = Assert<Pinned<typeof PushPublicKey, GeneratedPushPublicKey>>;

/** Only the browser's endpoint and keys; identity comes from the authenticated session. */
export const CreatePushSubscription = Schema.Struct({
  endpoint: Schema.String,
  p256dhKey: Schema.String,
  authKey: Schema.String,
});

export type CreatePushSubscriptionPin = Assert<
  Pinned<typeof CreatePushSubscription, GeneratedCreatePushSubscription>
>;

/** A room on the profile's "Rooms you're in" list, with the viewer's notification level. */
export const RoomMembershipRow = Schema.Struct({
  roomId: RoomId,
  name: Schema.String,
  /** `null` when the membership has no level stored (classic labels it with nothing). */
  involvement: Schema.NullOr(Involvement),
  direct: Schema.Boolean,
});

export type RoomMembershipRow = typeof RoomMembershipRow.Type;

export type RoomMembershipRowPin = Assert<
  Pinned<typeof RoomMembershipRow, GeneratedRoomMembershipRow>
>;

/** A browser that skips the two-step code for 30 days. */
export const RememberedDevice = Schema.Struct({
  id: RememberedDeviceId,
  description: Schema.String,
  ipAddress: Schema.NullOr(Schema.String),
  lastUsedAt: Schema.NullOr(Timestamp),
});

export type RememberedDevicePin = Assert<
  Pinned<typeof RememberedDevice, GeneratedRememberedDevice>
>;

/** The two-step sign-in panel: whether it is on, how to confirm, the remembered browsers. */
export const TwoFactorSettings = Schema.Struct({
  confirmedAt: Schema.NullOr(Timestamp),
  google: Schema.Boolean,
  hasPassword: Schema.Boolean,
  devices: Schema.Array(RememberedDevice),
});

export type TwoFactorSettingsPin = Assert<
  Pinned<typeof TwoFactorSettings, GeneratedTwoFactorSettings>
>;

/** `GET /api/v1/settings/account`: rooms, two-step sign-in and the sign-in link. */
export const AccountSettings = Schema.Struct({
  sharedRooms: Schema.Array(RoomMembershipRow),
  directRooms: Schema.Array(RoomMembershipRow),
  twoFactor: TwoFactorSettings,
  transferUrl: Schema.String,
  /** The transfer link's QR code, a whole SVG document drawn on the server (never in a URL). */
  transferQrSvg: Schema.String,
});

export type AccountSettings = typeof AccountSettings.Type;

export type AccountSettingsPin = Assert<Pinned<typeof AccountSettings, GeneratedAccountSettings>>;

/** The body of each two-step write: a code or password, or empty after "Confirm with Google". */
export const Reauthentication = Schema.Struct({ reauth: Schema.String });

export type ReauthenticationPin = Assert<
  Pinned<typeof Reauthentication, GeneratedReauthentication>
>;

/** New backup codes, shown once. */
export const BackupCodes = Schema.Struct({ codes: Schema.Array(Schema.String) });

export type BackupCodesPin = Assert<Pinned<typeof BackupCodes, GeneratedBackupCodes>>;

/** A two-step write that changed the account: the classic notice and the panel as it stands. */
export const TwoFactorChange = Schema.Struct({
  notice: Schema.String,
  twoFactor: TwoFactorSettings,
});

export type TwoFactorChangePin = Assert<Pinned<typeof TwoFactorChange, GeneratedTwoFactorChange>>;
