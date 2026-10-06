use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{Timestamp, User};

/// `GET /api/v1/me` and the boot JSON: the signed-in person, with the settings only they see.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Me {
    pub user: User,
    pub email_address: Option<String>,
    pub preferences: Preferences,
    pub presence_setting: PresenceSetting,
    pub do_not_disturb: DoNotDisturb,
    /// `null` when quiet hours are off.
    pub quiet_hours: Option<QuietHours>,
    /// `null` when not out of office.
    pub out_of_office: Option<OutOfOffice>,
}

/// Appearance and huddle settings, already normalized the way the layouts read them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Preferences {
    pub theme: Theme,
    pub text_size: TextSize,
    /// An IANA or Rails zone name; `null` when unset.
    pub time_zone: Option<String>,
    /// "Not set" was chosen on purpose, so the browser's zone isn't adopted.
    pub time_zone_explicit: bool,
    pub tour_completed: bool,
    pub voice_mode: VoiceMode,
    pub push_to_talk_key: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum Theme {
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum TextSize {
    Smaller,
    Small,
    Default,
    Large,
    Larger,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum VoiceMode {
    VoiceActivity,
    PushToTalk,
}

/// `users.presence_setting`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum PresenceSetting {
    Auto,
    Dnd,
    Invisible,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DoNotDisturb {
    pub enabled: bool,
    /// `null` with `enabled` means until turned off.
    pub until: Option<Timestamp>,
}

/// Minutes after midnight in the person's time zone; `end` may be before `start` (overnight).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct QuietHours {
    pub start_minute: u16,
    pub end_minute: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct OutOfOffice {
    pub until: Timestamp,
    pub note: Option<String>,
    /// Notifications still arrive while away.
    pub keep_notifications: bool,
}
