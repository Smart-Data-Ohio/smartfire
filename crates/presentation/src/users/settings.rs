use crate::helpers as h;

use serde::Deserialize;

#[derive(Clone, Debug, Default, Deserialize)]
pub struct SettingsPerson {
    pub id: i64,
    pub name: String,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct SettingsFormData {
    #[serde(default)]
    pub theme: String,
    #[serde(default)]
    pub text_size: String,
    #[serde(default)]
    pub time_zone: Option<String>,
    #[serde(default)]
    pub time_zone_choices: Vec<(String, String)>,
    pub presence_setting: String,
    pub custom_status_emoji: Option<String>,
    pub custom_status_text: Option<String>,
    pub ooo_note: Option<String>,
    pub dnd_active: bool,
    pub quiet_hours_enabled: bool,
    pub quiet_hours_start: Option<String>,
    pub quiet_hours_end: Option<String>,
    pub meeting_status_enabled: bool,
    pub meeting_dnd_enabled: bool,
    pub ooo_calendar_enabled: bool,
    pub ooo_notify_enabled: bool,
    pub out_of_office: bool,
    pub manual_ooo_active: bool,
    pub ooo_until_date: Option<String>,
    pub google_configured: bool,
    pub calendar_connected: bool,
    pub google_email: Option<String>,
    pub fetch_error: Option<String>,
    pub allowed_people: Vec<SettingsPerson>,
    pub keyword_alerts: String,
    pub errors: Vec<(String, String)>,
}
pub fn profile_zone_table() -> &'static serde_json::Value {
    static TABLE: std::sync::LazyLock<serde_json::Value> = std::sync::LazyLock::new(|| {
        serde_json::from_str(include_str!("profile_zones.json")).expect("pinned Rails zone choices")
    });
    &TABLE
}

/// Rails TimeZone#to_s uses TZInfo's current *base* UTC offset (including negative DST),
/// rather than the total wall-clock offset. Vendored transitions come from the pinned image.
pub fn profile_time_zone_choices(now: jiff::Timestamp) -> Vec<(String, String)> {
    profile_zone_table()["choice_zones"]
        .as_array()
        .unwrap()
        .iter()
        .map(|zone| {
            let mut offset = zone["initial"].as_i64().unwrap();
            for change in zone["changes"].as_array().unwrap() {
                if change[0].as_i64().unwrap() > now.as_second() {
                    break;
                }
                offset = change[1].as_i64().unwrap();
            }
            let label = format!(
                "(GMT{}{:02}:{:02}) {}",
                if offset < 0 { "-" } else { "+" },
                offset.abs() / 3600,
                offset.abs() % 3600 / 60,
                zone["name"].as_str().unwrap()
            );
            (label, zone["id"].as_str().unwrap().to_owned())
        })
        .collect()
}
impl SettingsFormData {
    pub fn has_errors(&self, attribute: &str) -> bool {
        self.errors.iter().any(|(a, _)| a == attribute)
    }
    pub fn error_sentence(&self, attribute: &str) -> String {
        let errors = self
            .errors
            .iter()
            .filter(|(a, _)| a == attribute)
            .map(|(_, message)| message.clone())
            .collect::<Vec<_>>();
        h::to_sentence(&errors, " and ")
    }
    pub fn fetch_error_present(&self) -> bool {
        self.fetch_error
            .as_deref()
            .is_some_and(|s| !s.chars().all(char::is_whitespace))
    }
}
