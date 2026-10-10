

pub use crate::context::{NotificationSounds, UserPreferences};

/// What the application layout's chrome shows that other domains own. The controller gathers it
/// before rendering (see `PORTING.md`, "Layout view models"); `Default` is a signed-out page in an
/// app with nothing optional configured.
#[derive(Clone, Debug, Default)]
pub struct Chrome {
    /// Rails.env.test?: the layout disables token-driven motion only in tests.
    pub test_environment: bool,
    /// `service_worker_auto_register?`: true outside the test environment (the reference runs in
    /// production), or when the `enable_service_worker` cookie is present.
    pub service_worker_auto_register: bool,
    /// The script chosen for this request, in Turbo-refreshed head metadata, at scope `/`.
    pub service_worker_url: Option<String>,
    /// `Icons.client_icon_names` (the icons domain).
    pub brand_icon_names: Vec<String>,
    /// `Google::Picker.configured?` and its settings (the Google domain).
    pub google_picker: Option<GooglePicker>,
    /// `Huddle.configured?` (the huddles domain): renders the huddle, invitation and join-notice
    /// partials.
    pub huddle_configured: bool,
    /// `global_search_query`: `params[:q].to_s.squish.presence` on the searches controller only.
    pub global_search_query: Option<String>,
    /// `Current.user.searches.ordered.limit(10)` (the searches domain).
    pub recent_searches: Vec<RecentSearch>,
}

#[derive(Clone, Debug)]
pub struct GooglePicker {
    pub client_id: String,
    pub api_key: String,
    pub project_number: String,
}

#[derive(Clone, Debug)]
pub struct RecentSearch {
    pub id: i64,
    pub query: String,
}
