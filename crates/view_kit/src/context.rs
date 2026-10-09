//! The account, user, and platform facts both shells read. Classic chrome (import map,
//! stylesheets, huddles) stays on `campfire_views::ViewContext`.

/// The signed-in user as the layout's meta tags and helpers see them.
#[derive(Clone, Debug)]
pub struct CurrentUser {
    pub id: i64,
    pub name: String,
    pub administrator: bool,
    pub bot: bool,
    /// `fresh_user_avatar_path(Current.user)`.
    pub avatar_url: String,
    /// The settings the layout reads off `Current.user`.
    pub preferences: UserPreferences,
}

/// `Current.account` for the layout: its name, logo path, and whether a logo is attached.
#[derive(Clone, Debug)]
pub struct AccountSummary {
    pub name: String,
    /// `fresh_account_logo_path` (no size).
    pub logo_url: String,
    /// `Current.account.logo.attached?` (adds the `account-has-logo` body class).
    pub has_logo: bool,
}

/// `ApplicationPlatform` facts derived from the user agent.
#[derive(Clone, Debug, Default)]
pub struct Platform {
    pub ios: bool,
    pub android: bool,
    pub mac: bool,
    pub windows: bool,
    pub chrome: bool,
    pub firefox: bool,
    pub safari: bool,
    pub edge: bool,
    pub mobile: bool,
    pub desktop: bool,
    /// `ApplicationPlatform#apple_messages?`.
    pub apple_messages: bool,
    /// `user_agent.browser` from the useragent gem ("Chrome", "Safari", "Firefox", "Edge", ...).
    pub browser: String,
    /// `ApplicationPlatform#operating_system` ("macOS", "Windows", "iPhone", ...).
    pub operating_system: String,
}

/// The settings the layout reads off `Current.user` (`User::StatusSettings` and friends).
#[derive(Clone, Debug, Default)]
pub struct UserPreferences {
    /// `users.theme`, as stored (`user_theme` falls back to "system" for anything else).
    pub theme: Option<String>,
    /// `users.text_size`, as stored (`user_text_size` falls back to "default").
    pub text_size: Option<String>,
    /// `users.time_zone`, as stored (possibly blank or a name Rails doesn't know).
    pub time_zone: Option<String>,
    /// `users.time_zone_explicit`: "Not set" was chosen on purpose.
    pub time_zone_explicit: bool,
    /// `tour_completed_at.present?`.
    pub tour_completed: bool,
    /// `users.voice_mode` and `users.push_to_talk_key` as stored, for the huddle partial (see
    /// [`UserPreferences::voice_mode`]).
    pub voice_mode: Option<String>,
    pub push_to_talk_key: Option<String>,
    /// `Current.user.google_account&.drive?`.
    pub google_drive: bool,
    pub notification_sounds: NotificationSounds,
}

impl UserPreferences {
    /// `User#voice_mode`: the stored mode if it's one of `VOICE_MODES`, else "voice_activity".
    pub fn voice_mode(&self) -> &str {
        match self.voice_mode.as_deref() {
            Some(mode @ ("voice_activity" | "push_to_talk")) => mode,
            _ => "voice_activity",
        }
    }

    /// `User#push_to_talk_key`: the stored key unless blank, else "`".
    pub fn push_to_talk_key(&self) -> &str {
        match self.push_to_talk_key.as_deref() {
            Some(key) if !crate::helpers::is_blank(key) => key,
            _ => "`",
        }
    }
}

/// What `notification_sound_meta_tags` sends (`app/helpers/application_helper.rb`), already
/// decided by the owner of each setting (the presence and calendar domains):
#[derive(Clone, Debug, Default)]
pub struct NotificationSounds {
    /// `manual_dnd_active? || presence_setting == "dnd"`.
    pub muted: bool,
    /// `[quiet_hours_start_minute, quiet_hours_end_minute]` when quiet hours are enabled and both
    /// are set.
    pub quiet_hours: Option<(i64, i64)>,
    /// `meeting_cache.quiet_window_epochs` when meeting DND and meeting status are both enabled.
    pub meeting_quiet: Vec<(i64, i64)>,
    /// Unless `ooo_notify_enabled?`: `[0, ooo_until]` when manual OOO is active, then the
    /// calendar's `ooo_window_epochs` when calendar OOO is enabled.
    pub ooo_quiet: Vec<(i64, i64)>,
}
