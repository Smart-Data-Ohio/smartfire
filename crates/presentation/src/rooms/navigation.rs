use crate::{huddle::Participant, huddle_stage::Stage};
use crate::helpers as h;
// Complete room header region: app/views/rooms/show/_nav.html.erb.
use super::RoomView;
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Navigation {
    pub room: RoomView,
    #[serde(default)]
    pub icon: Option<h::AvatarIcon>,
    pub pins_count: i64,
    pub involvement: String,
    pub participants: Vec<Participant>,
    pub stage: Option<Stage>,
}
