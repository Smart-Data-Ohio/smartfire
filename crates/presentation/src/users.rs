pub mod profile_sections;
pub mod statuses;
pub mod appearance;
pub mod settings;
pub mod people;
pub mod summary;
pub mod sidebar_composition;
pub mod sidebar;
pub mod google;

pub use sidebar::*;
pub use summary::*;
pub use people::*;
pub use settings::*;
pub use appearance::*;
pub use profile_sections::*;

#[derive(Clone)]
pub struct UserSession {
    pub id: i64,
    pub current: bool,
    pub description: String,
    pub ip_address: Option<String>,
    pub last_active_at: jiff::Timestamp,
    pub created_at: jiff::Timestamp,
}

/// A user as `users/_mention` and the autocompletable views see it.
#[derive(Clone, Debug, Default)]
pub struct MentionUser {
    pub user: UserSummary,
    /// `user.attachable_sgid`.
    pub attachable_sgid: String,
}

/// A membership row on the profile (`users/profiles/_membership`).
#[derive(Clone, Debug)]
pub struct ProfileMembership {
    pub room_id: i64,
    /// "rooms_open", "rooms_closed" or "rooms_direct".
    pub room_param_key: String,
    /// `room_display_name(membership.room)`.
    pub room_display_name: String,
    pub involvement: String,
    pub direct: bool,
}

/// A `Push::Subscription`, with its user agent parsed (`UserAgent.parse`).
#[derive(Clone, Debug)]
pub struct PushSubscription {
    pub id: i64,
    pub endpoint: String,
    pub browser: String,
    pub version: String,
    pub platform: String,
}

impl std::ops::Deref for MentionUser {
    type Target = UserSummary;
    fn deref(&self) -> &UserSummary {
        &self.user
    }
}
