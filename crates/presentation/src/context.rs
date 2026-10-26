//! Template-free account and user preference facts.

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
    /// `users.voice_mode` and `users.push_to_talk_key` as stored.
    pub voice_mode: Option<String>,
    pub push_to_talk_key: Option<String>,
    /// `Current.user.google_account&.drive?`.
    pub google_drive: bool,
    pub notification_sounds: NotificationSounds,
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

/// `User.administrator.first`, shown by the help-contact partial.
#[derive(Clone, Debug)]
pub struct HelpContact {
    pub name: String,
    pub email_address: String,
}
