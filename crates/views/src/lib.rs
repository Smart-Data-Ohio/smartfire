//! Askama templates mirroring `reference/app/views`, one template file per ERB file at the same
//! relative path under `templates/`. Views take plain view-model structs defined here, never
//! database rows, so this crate doesn't depend on `campfire_db`. Rich text arrives pre-rendered
//! as sanitized HTML. Every template renders with the per-request [`ViewContext`] below.

pub mod fragment_cache;
pub use campfire_view_kit::flash;
pub mod accounts;
pub mod activity;
pub mod agents;
pub mod autocompletable;
pub mod channel_threads;
pub mod events;
pub mod first_runs;
pub mod fizzy_cards;
pub mod fizzy_message_cards;
pub mod github;
pub mod helpers;
pub mod huddle;
pub mod huddle_stage;
pub mod integration_health;
pub mod layouts;
pub mod link_embeds;
pub mod linkedin_cards;
pub mod message_links;
pub mod message_providers;
pub mod messages;
pub mod pins;
pub mod public_pages;
pub mod pwa;
pub mod rooms;
pub mod saved_items;
pub mod scheduled_messages;
pub mod searches;
pub mod sessions;
pub mod shared;
pub mod slack;
pub mod sudos;
pub mod time;
pub mod twitter;
pub mod two_factor;
pub mod users;
pub mod welcome;

/// Per-request state every page needs: what `ApplicationController`, the layout and the
/// helpers read from `Current`, `request`, `flash` and the session.
#[derive(Clone)]
pub struct ViewContext<'a> {
    pub current_user: Option<CurrentUser>,
    pub account: AccountSummary,
    pub flash_notice: Option<String>,
    pub flash_alert: Option<String>,
    /// `ApplicationPlatform` facts derived from the user agent.
    pub platform: Platform,
    /// `Rails.configuration.x.vapid.public_key`; `None` omits the meta tag's content attribute.
    pub vapid_public_key: Option<String>,
    /// Resolves a logical asset path ("campfire-icon.png") to its digested URL.
    pub asset_path: &'a dyn Fn(&str) -> String,
    /// The `<script type="importmap">` + modulepreload tags (`javascript_importmap_tags`).
    pub importmap_tags: &'a str,
    /// `<link rel="stylesheet">` tags for `stylesheet_link_tag :all, "data-turbo-track": "reload"`.
    pub stylesheet_tags: &'a str,
    /// The account's custom CSS, if any (`custom_styles_tag`).
    pub custom_styles: Option<String>,
    /// `script_aware_action_cable_meta_tag` content: script_name + "/cable".
    pub cable_url: String,
    /// `request.base_url` ("http://campfire.test"), for the `*_url` helpers.
    pub base_url: String,
    /// `request.url`, compared against the referrer by `link_back`.
    pub request_url: String,
    /// `request.referrer`.
    pub referrer: Option<String>,
    /// Id of `last_room_visited` (`TrackedRoomVisit`): the `last_room` cookie's room if the user
    /// is a member, else `Current.user.rooms.original`. `None` links back to the root.
    pub last_room_visited_id: Option<i64>,
    /// `Rails.application.config.app_version` (APP_VERSION, GIT_REVISION or "0").
    pub app_version: String,
    /// `Turbo::StreamsChannel.signed_stream_name` over already-resolved streamables (a record is
    /// its GID param, see [`helpers::gid_param`]): the stream names `turbo_stream_from` renders.
    /// They depend only on the app's secret, never on the session.
    pub signed_stream_name: &'a dyn Fn(&[&str]) -> String,
    /// `Time.zone` for this request: the user's saved zone if it names one (`SetTimeZone`),
    /// else the default, UTC. Times render in it.
    pub time_zone: time::Zone,
    /// What the layout's chrome shows that belongs to other domains (huddles, Google, searches,
    /// icons), gathered by the controller before rendering.
    pub chrome: layouts::Chrome,
}

impl ViewContext<'_> {
    /// Access to these values corresponds to flash[:notice]/flash[:alert] in ERB.
    pub fn flash_notice(&self) -> Option<&String> {
        flash::read();
        self.flash_notice.as_ref()
    }
    pub fn flash_alert(&self) -> Option<&String> {
        flash::read();
        self.flash_alert.as_ref()
    }

    pub fn asset(&self, logical_path: &str) -> String {
        (self.asset_path)(logical_path)
    }

    /// `root_url`, `session_url`, `join_url(...)`: base URL + path.
    pub fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    pub fn can_administer(&self) -> bool {
        self.current_user
            .as_ref()
            .is_some_and(|user| user.administrator)
    }

    pub fn current_user_id(&self) -> Option<i64> {
        self.current_user.as_ref().map(|user| user.id)
    }

    /// `Current.user && !Current.user.bot?`: the global search, help menu and tour are for people.
    pub fn human_signed_in(&self) -> bool {
        self.current_user.as_ref().is_some_and(|user| !user.bot)
    }

    /// The signed-in user's settings.
    pub fn preferences(&self) -> Option<&layouts::UserPreferences> {
        self.current_user.as_ref().map(|user| &user.preferences)
    }

    /// `Current.user&.google_account&.drive?`.
    pub fn google_drive_previews(&self) -> bool {
        self.preferences()
            .is_some_and(|preferences| preferences.google_drive)
    }

    /// `Google::Picker.configured? && Current.user`.
    pub fn google_picker(&self) -> Option<&layouts::GooglePicker> {
        self.current_user
            .as_ref()
            .and(self.chrome.google_picker.as_ref())
    }

    /// `Current.user.tour_completed_at.nil?`.
    pub fn tour_pending(&self) -> bool {
        self.preferences()
            .is_some_and(|preferences| !preferences.tour_completed)
    }

    /// `Current.user && Huddle.configured?`.
    pub fn huddle_configured(&self) -> bool {
        self.current_user.is_some() && self.chrome.huddle_configured
    }

    /// `Current.user == user`.
    pub fn is_current_user(&self, user_id: impl std::borrow::Borrow<i64>) -> bool {
        self.current_user_id() == Some(*user_id.borrow())
    }
}

pub use campfire_view_kit::{AccountSummary, CurrentUser, Platform};

pub mod room_files;
pub mod work_threads;

#[cfg(test)]
mod card_html_audit;

pub mod rendering;
