// users/profiles/_appearance.html.erb. The choices are the pinned Rails catalogue.

#[derive(Clone)]
pub struct AppearanceData {
    pub theme: String,
    pub text_size: String,
    pub zone_identifier: Option<String>,
    pub theme_errors: Vec<String>,
    pub text_size_errors: Vec<String>,
    pub time_zone_errors: Vec<String>,
    /// The old switch between the two UIs. Always `None` since the SPA became everyone's UI;
    /// the panel goes with the classic pages.
    pub next_ui: Option<NextUi>,
}

/// "Try the new Smartfire": a form posting the person's UI to `/app/ui_preference`.
#[derive(Clone, Debug)]
pub struct NextUi {
    /// They use the new UI (`ui_preference`, else `SPA_DEFAULT`).
    pub on: bool,
    /// This page, where "Switch to classic" comes back to.
    pub return_to: String,
}

/// The time zone select's choices (label, IANA identifier), in its order, after "Not set".
pub fn profile_time_zones() -> &'static [(String, String)] {
    static CHOICES: std::sync::LazyLock<Vec<(String, String)>> =
        std::sync::LazyLock::new(|| serde_json::from_str(include_str!("profile_time_zones.json")).unwrap());
    &CHOICES
}
