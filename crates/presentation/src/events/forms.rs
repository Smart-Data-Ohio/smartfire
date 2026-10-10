
#[derive(Clone, Debug, serde::Deserialize)]
pub struct VenueOption {
    pub id: i64,
    pub name: String,
    pub stage: bool,
}
#[derive(Clone, Debug, serde::Deserialize)]
pub struct FormView {
    pub room_id: i64,
    pub room_name: String,
    pub id: Option<i64>,
    pub title: String,
    pub title_value: Option<String>,
    pub description: Option<String>,
    pub starts_at: Option<String>,
    pub ends_at: Option<String>,
    pub time_zone: String,
    pub venue_room_id: Option<i64>,
    pub recurrence_rule: Option<String>,
    pub recurrence_until: Option<String>,
    pub meet_link_requested: bool,
    pub meet_link: Option<String>,
    pub series: bool,
    pub head: bool,
    pub errors: Vec<String>,
    pub error_fields: Vec<String>,
    pub venues: Vec<VenueOption>,
}
impl FormView {
    pub fn path(&self) -> String {
        let base = format!("/rooms/{}/events", self.room_id);
        self.id.map(|id| format!("{base}/{id}")).unwrap_or(base)
    }
    pub fn back_path(&self) -> String {
        self.id
            .map(|_| self.path())
            .unwrap_or_else(|| format!("/rooms/{}/events", self.room_id))
    }
    pub fn error_count(&self) -> String {
        super::pages::plural(self.errors.len() as i64, "error")
    }
}
