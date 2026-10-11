pub mod forms;
pub mod pages;
// Event fragments use plain, preloaded facts; app/helpers/rooms/events_helper.rb.

#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct CardView {
    pub id: i64,
    pub room_id: i64,
    pub title: String,
    pub organizer_name: String,
    pub starts_at: crate::time::CalendarTime,
    pub ends_at: Option<crate::time::CalendarTime>,
    pub time_zone: String,
    pub series: bool,
    pub cancelled: bool,
    pub venue_name: Option<String>,
    pub meet_link: Option<String>,
}

impl CardView {
    pub fn path(&self) -> String {
        format!("/rooms/{}/events/{}", self.room_id, self.id)
    }
}
