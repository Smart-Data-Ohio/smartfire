// users/profiles/_appearance.html.erb. The choices are the pinned Rails catalogue.

/// The time zone select's choices (label, IANA identifier), in its order, after "Not set".
pub fn profile_time_zones() -> &'static [(String, String)] {
    static CHOICES: std::sync::LazyLock<Vec<(String, String)>> =
        std::sync::LazyLock::new(|| serde_json::from_str(include_str!("profile_time_zones.json")).unwrap());
    &CHOICES
}
