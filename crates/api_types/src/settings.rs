//! The signed-in person's settings (S7): `/api/v1/settings` and its sections, the SPA's twin of
//! the classic profile page (`users/profiles#show` and the forms on it).
//!
//! `GET /api/v1/settings` returns every section. Each write names every key of its body, `null`
//! for those it leaves as they are (a blank string clears a clearable text), and answers with the
//! whole [`Settings`] again, as the classic forms redirect back to the profile. A rejected change is
//! `ApiError::Validation` with the classic page's messages, its `fields` keyed by the wire names
//! (`currentPassword`, `customStatusText`, `oooUntil`, ...). Integration writes answer with
//! [`IntegrationChange`]. Sessions and push subscriptions have their own lists, never cached.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{Involvement, PresenceSetting, TextSize, Theme, Timestamp, VoiceMode};

/// `GET /api/v1/settings`, and the answer to profile, appearance, notification and status writes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Settings {
    /// Monotonic, persisted ordering for every settings response.
    pub revision: i64,
    /// The injected server clock used for expiry-dependent fields, in UTC with nanoseconds.
    pub evaluated_at: String,
    pub profile: ProfileSettings,
    pub appearance: AppearanceSettings,
    pub notifications: NotificationSettings,
    pub status: StatusSettings,
    pub calls: CallSettings,
    pub integrations: IntegrationSettings,
}

/// `GET /api/v1/settings/account`: the classic profile's lower panels for the signed-in person.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AccountSettings {
    /// Shared rooms first, then direct messages, each in the classic page's order.
    pub shared_rooms: Vec<RoomMembershipRow>,
    pub direct_rooms: Vec<RoomMembershipRow>,
    /// The two-step sign-in panel. These routes answer people only, so it is always there (the
    /// classic page hides it for bots).
    pub two_factor: TwoFactorSettings,
    /// The absolute sign-in transfer URL the classic `_transfer.html` shows.
    pub transfer_url: String,
    /// `transfer_url`'s QR code, a whole SVG document drawn here, so the link never travels in a
    /// request path the way the classic `/qr_code/:id` image's does.
    pub transfer_qr_svg: String,
}

/// A room in the classic profile's membership list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RoomMembershipRow {
    pub room_id: i64,
    /// `membership.room_display_name`, as the classic row shows it.
    pub name: String,
    /// The same involvement the room's existing involvement API changes. `None` for a membership
    /// with none stored: the classic row labels it with nothing, and no mention reaches it.
    pub involvement: Option<Involvement>,
    pub direct: bool,
}

/// The classic two-step sign-in panel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TwoFactorSettings {
    /// When it was turned on; `None` means not set up.
    pub confirmed_at: Option<Timestamp>,
    /// The account can "Confirm with Google" instead of a code (`data.google` in the partial).
    pub google: bool,
    /// The account has a password (decides the classic alert wording).
    pub has_password: bool,
    pub devices: Vec<RememberedDevice>,
}

/// An unexpired device in the classic two-step panel, in its displayed order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RememberedDevice {
    pub id: i64,
    /// The classic row's bold line (`self.agent(device)`), already worded.
    pub description: String,
    /// Where it was last used from, the start of the classic row's second line.
    pub ip_address: Option<String>,
    pub last_used_at: Option<Timestamp>,
}

/// The body of each two-step write: the authenticator code or password. Empty after a finished
/// "Confirm with Google" round trip.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Reauthentication {
    pub reauth: String,
}

/// `POST /api/v1/settings/two_factor/backup_codes`: the new codes, shown once.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct BackupCodes {
    pub codes: Vec<String>,
}

/// A two-step write that changed the account: the classic notice and the panel as it now stands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TwoFactorChange {
    pub notice: String,
    pub two_factor: TwoFactorSettings,
}

/// Name, email, password, bio, avatar and GitHub username (the profile form).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ProfileSettings {
    pub user_id: i64,
    pub name: String,
    pub email_address: Option<String>,
    pub bio: Option<String>,
    /// The avatar image path (initials when none is attached), versioned like `User.avatarUrl`.
    pub avatar_url: String,
    /// An uploaded avatar is attached (it can be removed).
    pub avatar_attached: bool,
    /// The person signs in with a password, so changing the email needs `currentPassword`.
    pub has_password: bool,
    pub github_login: Option<String>,
    /// Set by a linked GitHub account: read-only until GitHub is disconnected.
    pub github_verified: bool,
    /// Bots have no security, sessions or transfer sections.
    pub bot: bool,
    pub pronouns: Option<String>,
    pub nickname: Option<String>,
}

/// `PATCH /api/v1/settings/profile` (`users/profiles#update`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct UpdateProfile {
    pub name: Option<String>,
    pub email_address: Option<String>,
    /// Needed to change `emailAddress` when the person has a password (`hasPassword`). A new
    /// `password` doesn't need it, as on the classic page.
    pub current_password: Option<String>,
    /// A new password; blank keeps the current one.
    pub password: Option<String>,
    pub bio: Option<String>,
    pub github_login: Option<String>,
    pub pronouns: Option<String>,
    pub nickname: Option<String>,
}

/// `PUT /api/v1/settings/avatar`: a blob uploaded with `POST /api/v1/uploads` becomes the avatar.
/// `DELETE` removes it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct UpdateAvatar {
    pub signed_id: String,
}

/// JSON remains lossless across clients that understand different appearance versions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct AppearancePreferences(
    #[ts(type = "null | boolean | number | string | ReadonlyArray<AppearancePreferences> | { [key: string]: AppearancePreferences }")]
    pub serde_json::Value,
);

/// Account appearance and time zone. Explicit device overrides stay in the browser.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AppearanceSettings {
    pub theme: Theme,
    pub text_size: TextSize,
    /// The chosen zone's identifier; `null` is "Not set (use system)".
    pub time_zone: Option<String>,
    /// The classic select's choices, in its order.
    pub time_zones: Vec<TimeZoneChoice>,
    /// Versioned personal appearance; newer documents are kept opaque.
    pub appearance_preferences: Option<AppearancePreferences>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TimeZoneChoice {
    pub label: String,
    pub value: String,
}

/// `PATCH /api/v1/settings/appearance` (`users/profiles#update`'s appearance fields).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct UpdateAppearance {
    pub theme: Option<Theme>,
    pub text_size: Option<TextSize>,
    /// `""` is "Not set (use system)".
    pub time_zone: Option<String>,
    /// Changed fields only; omitted keys stay stored and explicit nulls clear individual keys.
    pub appearance_preferences: Option<AppearancePreferences>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum NotificationLevel { Everything, Mentions, Nothing }

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RoomNotificationUpdate {
    pub room_id: i64,
    /// Null inherits the account default.
    pub level: Option<NotificationLevel>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum RoomMuteDuration { Minutes15, Hour1, Hours8, Hours24, Forever, Off }

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RoomMuteUpdate {
    pub room_id: i64,
    pub duration: RoomMuteDuration,
}

/// Do not disturb, quiet hours, meetings, out of office, keyword alerts and the activity inbox
/// switches (the notifications form and the profile form's inbox switches).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct NotificationSettings {
    pub default_notification_level: NotificationLevel,
    /// An entry with null inherits; rooms without entries keep their membership setting.
    pub room_notification_levels: BTreeMap<String, Option<NotificationLevel>>,
    /// Null means indefinite; expired deadlines read as unmuted.
    pub room_mute_until: BTreeMap<String, Option<String>>,
    /// Manual DND (until turned off); quiet hours and meetings are separate.
    pub dnd_enabled: bool,
    pub quiet_hours_enabled: bool,
    /// `"HH:MM"` in the person's zone; `null` when unset.
    pub quiet_hours_start: Option<String>,
    pub quiet_hours_end: Option<String>,
    pub meeting_dnd_enabled: bool,
    pub ooo_notify_enabled: bool,
    /// People whose messages still get through DND (starred), by name. Starring is
    /// `POST /api/v1/settings/dnd_allowances/:user_id` and unstarring `DELETE` (active people
    /// other than you); both answer with the settings.
    pub allowed_people: Vec<DndAllowedPerson>,
    /// One word or phrase each, up to 20.
    pub keyword_alerts: Vec<String>,
    pub inbox: Vec<InboxSwitch>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DndAllowedPerson {
    pub user_id: i64,
    pub name: String,
}

/// One activity-inbox switch, with the classic page's label and description.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct InboxSwitch {
    pub key: String,
    pub label: String,
    pub description: String,
    pub enabled: bool,
}

/// `PATCH /api/v1/settings/notifications` (`users/notification_settings#update`, plus the
/// profile form's `inbox_preferences`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct UpdateNotifications {
    pub default_notification_level: Option<NotificationLevel>,
    pub room_notification: Option<RoomNotificationUpdate>,
    pub room_mute: Option<RoomMuteUpdate>,
    pub dnd_enabled: Option<bool>,
    pub quiet_hours_enabled: Option<bool>,
    /// `"HH:MM"`; `""` clears it.
    pub quiet_hours_start: Option<String>,
    pub quiet_hours_end: Option<String>,
    pub meeting_dnd_enabled: Option<bool>,
    pub ooo_notify_enabled: Option<bool>,
    /// Replaces the whole list.
    pub keyword_alerts: Option<Vec<String>>,
    /// Inbox switches by key; keys not named keep their value.
    pub inbox: Option<BTreeMap<String, bool>>,
}

/// Presence, custom status, meeting status and out of office (the status form).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct StatusSettings {
    pub presence_setting: PresenceSetting,
    pub custom_status_emoji: Option<String>,
    pub custom_status_text: Option<String>,
    /// When the custom status clears itself; `null` is never.
    pub custom_status_expires_at: Option<Timestamp>,
    pub meeting_status_enabled: bool,
    pub ooo_calendar_enabled: bool,
    /// The out-of-office end that shows (manual or from the calendar, the later one).
    pub ooo_until: Option<Timestamp>,
    /// `oooUntil` is the manual one (clearing ends it).
    pub ooo_manual: bool,
    pub ooo_note: Option<String>,
    /// The calendar fetch's last error, as the classic page words it.
    pub calendar_error: Option<String>,
}

/// When a custom status clears itself (the classic "Clear after" choices).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum StatusExpiry {
    #[serde(rename = "minutes_30")]
    #[ts(rename = "minutes_30")]
    Minutes30,
    #[serde(rename = "hour_1")]
    #[ts(rename = "hour_1")]
    Hour1,
    #[serde(rename = "hours_4")]
    #[ts(rename = "hours_4")]
    Hours4,
    Today,
    Week,
    Never,
}

/// The out-of-office end presets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum OooPreset {
    Tomorrow,
    Monday,
    Week,
    /// `oooUntilCustom`, a local date and time in the person's zone (`YYYY-MM-DDTHH:MM`).
    Custom,
}

/// `PATCH /api/v1/settings/status` (`users/statuses#update`). The custom status and out of
/// office clear with their `clear...` flags, which win over the values beside them.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct UpdateStatus {
    pub presence_setting: Option<PresenceSetting>,
    pub custom_status_emoji: Option<String>,
    pub custom_status_text: Option<String>,
    pub custom_status_expires_in: Option<StatusExpiry>,
    pub clear_custom_status: Option<bool>,
    pub meeting_status_enabled: Option<bool>,
    pub ooo_calendar_enabled: Option<bool>,
    pub ooo_preset: Option<OooPreset>,
    pub ooo_until_custom: Option<String>,
    pub ooo_note: Option<String>,
    pub clear_ooo: Option<bool>,
}

/// Microphone mode and push-to-talk key (the profile form's calls fields).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CallSettings {
    pub voice_mode: VoiceMode,
    pub push_to_talk_key: Option<String>,
}

/// `PATCH /api/v1/settings/calls`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct UpdateCalls {
    pub voice_mode: Option<VoiceMode>,
    /// Up to 20 characters; `""` restores the default.
    pub push_to_talk_key: Option<String>,
}

/// `PUT /api/v1/settings/github_connection` and `/fizzy_connection`: a personal access token,
/// checked with the service before it is stored. It is never shown again.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct IntegrationToken {
    pub access_token: String,
}

/// What a connect or disconnect did: the integrations as they now stand, and the classic notice.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct IntegrationChange {
    pub integrations: IntegrationSettings,
    pub notice: String,
}

/// The connected services. OAuth starts remain browser navigations on the classic page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct IntegrationSettings {
    pub google: GoogleIntegration,
    pub github: Connection,
    /// The workspace has a GitHub App, so connecting can go through it ("Connect with GitHub")
    /// as well as a pasted personal token.
    pub github_app_configured: bool,
    pub fizzy: Connection,
    /// Where the classic page manages these (`/users/me/profile`).
    pub manage_path: String,
    /// The Slack importer (`/slack/imports`).
    pub slack_import_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GoogleIntegration {
    /// "Sign in with Google" is configured for the workspace.
    pub sign_in_configured: bool,
    /// The Google identity linked for sign-in, if any.
    pub identity_email: Option<String>,
    /// Google Calendar is configured for the workspace.
    pub calendar_configured: bool,
    pub connected: bool,
    /// The connection's grants.
    pub calendar: bool,
    pub drive: bool,
    /// The linked Google account's address; `null` when none is linked. A linked account can be
    /// disconnected (`connected: false`) and still have an address: it needs reconnecting.
    pub email: Option<String>,
}

/// A personal connection to GitHub or Fizzy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "state", rename_all = "lowercase", rename_all_fields = "camelCase")]
#[ts(export)]
pub enum Connection {
    Missing,
    /// It stopped working (revoked or expired); reconnecting fixes it.
    Rejected {
        reason: Option<String>,
    },
    Connected {
        name: String,
        workspace: Option<String>,
        /// Connected through the workspace's app rather than a personal token.
        app_token: bool,
    },
}

/// One signed-in browser or device (`GET /api/v1/settings/sessions`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SessionInfo {
    pub id: i64,
    /// This browser.
    pub current: bool,
    /// "Firefox on macOS", as the classic sessions page words it.
    pub description: String,
    pub ip_address: Option<String>,
    pub last_active_at: Timestamp,
    pub created_at: Timestamp,
}

/// `GET /api/v1/settings/sessions`, newest activity first; and the answer to revoking one
/// (`DELETE /api/v1/settings/sessions/:id`) or the rest (`POST .../sessions/revoke_others`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SessionList {
    pub sessions: Vec<SessionInfo>,
    /// What the last revocation did, as the classic notice words it ("Signed out 2 other
    /// sessions."); `null` on a plain read. Revoking the current session signs out instead: the
    /// answer is `Unauthorized`, and the client loads `/`. Pass `?push_subscription_endpoint=` to
    /// drop this browser's push subscription with it, as the classic sign-out does.
    pub notice: Option<String>,
}

/// One browser's Web Push subscription.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PushSubscriptionInfo {
    pub id: i64,
    pub endpoint: String,
    pub browser: String,
    pub version: String,
    pub platform: String,
}

/// `GET /api/v1/settings/push_subscriptions`, and the answer to creating or deleting one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PushSubscriptionList {
    pub push_subscriptions: Vec<PushSubscriptionInfo>,
}

/// `GET /api/v1/settings/push_subscriptions/key`: the same key the classic page presents.
/// `null` means Web Push is not configured with valid keys.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PushPublicKey {
    pub public_key: Option<String>,
}

/// `POST /api/v1/settings/push_subscriptions`: this browser's endpoint and encryption keys.
/// The owner comes from the session and the user agent from the request, never the body.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export)]
pub struct CreatePushSubscription {
    pub endpoint: String,
    pub p256dh_key: String,
    pub auth_key: String,
}
